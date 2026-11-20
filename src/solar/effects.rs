//! Extra geometry of the solar scene attached to its bodies (the royal
//! crown on The King's Crown, the diamond cards around House of Cards) and
//! to the spacecraft (the exhaust of its three nozzles), drawn through the
//! shared CPU pipeline and depth buffer. Each effect lists its draws (a
//! prebuilt mesh, the model matrix of what it rides on and a shader), so
//! the frame batch and the one-by-one path draw exactly the same thing and
//! `scene.rs` stays a thin dispatcher.

use crate::geometry::Vertex;
use crate::pipeline::{
    FragmentView, NormalSource, RenderError, ShadedOptions, ShadedStats, SurfaceSample, Transform,
    Uniforms, render_shaded_with_scratch,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::shading::lighting::lambert;
use crate::solar::SolarScene;
use crate::solar::attach::AttachmentKind;
use crate::solar::attach::props::{PROP_MIN_PX, SECTOR_FACING};
use crate::solar::card_ring::CardShader;
use crate::solar::instance::eye_towards;
use crate::solar::royal_crown::{CrownPart, CrownShader};
use crate::solar::thrusters::{FlameShader, NOZZLES, flame_length, flame_matrix};
use raylib::math::Vector3;

/// What a draw is shaded with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectShader {
    Crown(CrownShader),
    Card(CardShader),
    Flame(FlameShader),
    Prop(PropShader),
}

/// Flat-shaded surface prop: its part's color lit by the star, the light
/// given in the host's object space (where the baked meshes live).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropShader {
    pub color: Vector3,
    pub light: Vector3,
}

impl PropShader {
    pub fn shade(&self, face_normal: Vector3) -> Vector3 {
        self.color * (0.06 + 0.94 * lambert(face_normal, self.light))
    }
}

impl EffectShader {
    pub fn shade(&self, s: &SurfaceSample, view: &FragmentView) -> Vector3 {
        match self {
            Self::Crown(c) => c.shade(s.object_position, s.face_normal, view.view_direction),
            Self::Card(c) => c.shade(s.object_position, s.face_normal),
            Self::Flame(f) => f.shade(s.object_position, s.face_normal, view.view_direction),
            Self::Prop(p) => p.shade(s.face_normal),
        }
    }
}

/// One mesh of an effect ready to draw this frame.
#[derive(Debug, Clone, Copy)]
pub struct EffectDraw<'a> {
    pub mesh: &'a [Vertex],
    pub model: Uniforms,
    pub options: ShadedOptions,
    pub shader: EffectShader,
}

/// Closed meshes wound outwards with flat normals.
pub fn solid_options(threads: usize) -> ShadedOptions {
    ShadedOptions {
        cull_back_faces: true,
        normal_source: NormalSource::Face,
        threads,
    }
}

impl SolarScene {
    /// The body wearing the royal crown (its attachment's host), when the
    /// scene has both.
    pub fn crown_host(&self) -> Option<usize> {
        self.crown.as_ref()?;
        self.attachment_host(AttachmentKind::RoyalCrown)
    }

    /// The body the card ring circles (its attachment's host), when the
    /// scene has both.
    pub fn card_host(&self) -> Option<usize> {
        self.cards.as_ref()?;
        self.attachment_host(AttachmentKind::CardRing)
    }

    /// The crown's transform: the star's, turned by the crown's own spin.
    pub fn crown_transform(&self, host: usize) -> Transform {
        let star = self.bodies[host].transform;
        let spin = self.crown.as_ref().map_or(0.0, |c| c.spin);
        Transform {
            yaw: star.yaw + spin * self.clock.time,
            ..star
        }
    }

    /// Every effect draw that can appear on a `width` x `height` frame.
    pub fn effect_draws(&self, width: u32, height: u32, threads: usize) -> Vec<EffectDraw<'_>> {
        let mut draws = Vec::new();
        // Attachments in their list order (the draw order), each on the
        // body its id names.
        for attachment in &self.attachments {
            let Some(host) = self.body_index(attachment.host) else {
                continue;
            };
            match attachment.kind {
                AttachmentKind::RoyalCrown => {
                    let Some(crown) = &self.crown else { continue };
                    let transform = self.crown_transform(host);
                    let reach = self.radius_of(host) * crown.extent;
                    if self.sphere_in_view(transform.position, reach, width, height) {
                        let model = Uniforms {
                            model_matrix: transform.matrix(),
                        };
                        for (mesh, part) in [
                            (&crown.gold, CrownPart::Gold),
                            (&crown.jewels, CrownPart::Jewels),
                        ] {
                            draws.push(EffectDraw {
                                mesh,
                                model,
                                options: solid_options(threads),
                                shader: EffectShader::Crown(crown.shader(
                                    part,
                                    transform,
                                    self.clock.time,
                                )),
                            });
                        }
                    }
                }
                AttachmentKind::Props => {
                    let Some(props) = &self.props else { continue };
                    let projection = self.projection();
                    // The whole layer hides while the planet is small.
                    if self.radius_px(host, &projection, height) < PROP_MIN_PX {
                        continue;
                    }
                    let t = self.bodies[host].transform;
                    let model = Uniforms {
                        model_matrix: t.matrix(),
                    };
                    let light = t.to_object_dir(self.light_at(t.position)).normalized();
                    for layer in &props.layers {
                        for sector in &layer.sectors {
                            let center = t.position + t.to_world_dir(sector.bound_center) * t.scale;
                            let normal = t.to_world_dir(sector.normal);
                            // Sectors on the far hemisphere, then out of view.
                            if normal.dot(eye_towards(&projection, center)) < SECTOR_FACING
                                || !self.sphere_in_view(
                                    center,
                                    sector.bound_radius * t.scale,
                                    width,
                                    height,
                                )
                            {
                                continue;
                            }
                            draws.push(EffectDraw {
                                mesh: &sector.mesh,
                                model,
                                options: solid_options(threads),
                                shader: EffectShader::Prop(PropShader {
                                    color: layer.part.color(),
                                    light,
                                }),
                            });
                        }
                    }
                }
                AttachmentKind::CardRing => {
                    let Some(ring) = &self.cards else { continue };
                    let (center, radius) =
                        (self.bodies[host].transform.position, self.radius_of(host));
                    if self.sphere_in_view(center, radius * ring.extent(), width, height) {
                        let reach = radius * ring.card_reach();
                        for i in 0..ring.slots.len() {
                            let card = ring.pose(i, center, radius, self.clock.time);
                            if !self.sphere_in_view(card.origin, reach, width, height) {
                                continue;
                            }
                            let light = card.to_card_dir(self.light_at(card.origin));
                            draws.push(EffectDraw {
                                mesh: &ring.mesh,
                                model: Uniforms {
                                    model_matrix: card.matrix(),
                                },
                                options: solid_options(threads),
                                shader: EffectShader::Card(CardShader { light }),
                            });
                        }
                    }
                }
            }
        }
        if let Some(thrusters) = &self.thrusters {
            let ship = &self.ship;
            let time = self.clock.time;
            let lengths = [0, 1, 2].map(|i| flame_length(ship, i, time));
            let longest = lengths.iter().fold(0.0f32, |a, &b| a.max(b));
            let reach = ship.radius() + longest * ship.scale;
            if self.sphere_in_view(ship.position, reach, width, height) {
                for (i, (nozzle, length)) in NOZZLES.iter().zip(lengths).enumerate() {
                    draws.push(EffectDraw {
                        mesh: &thrusters.mesh,
                        model: Uniforms {
                            model_matrix: flame_matrix(ship, nozzle, length),
                        },
                        options: solid_options(threads),
                        shader: EffectShader::Flame(FlameShader::new(ship, i, length, time)),
                    });
                }
            }
        }
        draws
    }

    /// Draws every effect without clearing, one draw at a time. Returns
    /// the stats of each draw.
    pub fn render_effects_without_clear(
        &self,
        framebuffer: &mut Framebuffer,
        mode: TriangleMode,
        options: ShadedOptions,
    ) -> Result<Vec<ShadedStats>, RenderError> {
        let draws = self.effect_draws(framebuffer.width(), framebuffer.height(), options.threads);
        let projection = self.projection();
        let mut scratch = self.scratch_mut();
        draws
            .iter()
            .map(|d| {
                render_shaded_with_scratch(
                    framebuffer,
                    &d.model,
                    d.mesh,
                    &projection,
                    mode,
                    d.options,
                    &|s, v| d.shader.shade(s, v),
                    &mut scratch,
                )
            })
            .collect()
    }
}

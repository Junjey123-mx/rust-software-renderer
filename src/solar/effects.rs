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
    Uniforms, ViewProjection, render_shaded_with_scratch,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::shading::lighting::lambert;
use crate::solar::SPHERE_RADIUS;
use crate::solar::SolarScene;
use crate::solar::attach::AttachmentKind;
use crate::solar::attach::props::{PROP_MIN_PX, PropLayers, PropPart, beyond_horizon};
use crate::solar::attach::traffic::{
    CYCLE_EDGE, MAX_TRAFFIC, NEON_BODY, NEON_EDGE, NEON_FRESNEL, NeonShader, TrailBatchShader,
    TrailShader,
};
use crate::solar::attach::waterfalls::WaterfallShader;
use crate::solar::card_ring::CardShader;
use crate::solar::instance::{eye_towards, projected_radius_px};
use crate::solar::planet::PlanetMaterial;
use crate::solar::planet::ace::CrystalShader;
use crate::solar::royal_crown::{CrownPart, CrownShader};
use crate::solar::ship::ship_options;
use crate::solar::thrusters::{FlameShader, NOZZLES, flame_length, flame_matrix};
use raylib::math::Vector3;

/// Color of the Hearts identity disc.
pub const HEART_DISC: Vector3 = Vector3::new(1.0, 0.08, 0.3);

/// What a draw is shaded with.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectShader {
    Crown(CrownShader),
    Card(CardShader),
    Flame(FlameShader),
    Prop(PropShader),
    Waterfall(WaterfallShader),
    Neon(NeonShader),
    Trail(TrailShader),
    /// Boxed: its chord lists outweigh every other shader.
    TrailBatch(Box<TrailBatchShader>),
    Crystal(CrystalShader),
}

/// Flat-shaded surface prop: its part's color lit by the star, the light
/// given in the host's object space (where the baked meshes live).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropShader {
    pub color: Vector3,
    pub light: Vector3,
    /// Share of the color that glows (neon edges) instead of being lit.
    pub glow: f32,
}

impl PropShader {
    pub fn shade(&self, face_normal: Vector3) -> Vector3 {
        let lit = self.color * (0.06 + 0.94 * lambert(face_normal, self.light));
        if self.glow > 0.0 {
            lit * (1.0 - self.glow) + self.color * self.glow
        } else {
            lit
        }
    }
}

impl EffectShader {
    pub fn shade(&self, s: &SurfaceSample, view: &FragmentView) -> Vector3 {
        match self {
            Self::Crown(c) => c.shade(s.object_position, s.face_normal, view.view_direction),
            Self::Card(c) => c.shade(s.object_position, s.face_normal),
            Self::Flame(f) => f.shade(s.object_position, s.face_normal, view.view_direction),
            Self::Prop(p) => p.shade(s.face_normal),
            Self::Waterfall(w) => w.shade(s.object_position, s.face_normal),
            Self::Neon(n) => n.shade(s.face_normal),
            Self::Trail(t) => t.shade(s.object_position),
            Self::TrailBatch(t) => t.shade(s.object_position),
            Self::Crystal(c) => c.shade(s.object_position, s.face_normal, view.view_direction),
        }
    }
}

/// One mesh of an effect ready to draw this frame.
#[derive(Debug, Clone)]
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

    /// Whether the surface patch of body `host` bounded by the sphere at
    /// `bound_center` of `bound_radius` (object space, on the mesh of
    /// radius `SPHERE_RADIUS`) is hidden behind the body for this frame's
    /// eye (`beyond_horizon`). The body occludes as its lowest surface: the
    /// sea, or the deepest ground, less 0.02 radii for the chords of the
    /// coarsest mesh. Never hidden with `culling` off.
    pub fn patch_hidden(
        &self,
        host: usize,
        projection: &ViewProjection,
        bound_center: Vector3,
        bound_radius: f32,
    ) -> bool {
        if !self.culling {
            return false;
        }
        let t = self.bodies[host].transform;
        let radius = self.radius_of(host);
        let (to_eye, eye) = match projection {
            ViewProjection::Perspective(camera) => {
                let d = camera.eye - t.position;
                (d.normalized(), d.length() / radius)
            }
            ViewProjection::Orbit(view) => (view.camera.eye_direction(), f32::INFINITY),
        };
        let lowest = self
            .relief_of(host)
            .map_or(1.0, |r| match r.field.sea_level() {
                Some(sea) => 1.0 + sea,
                None => 1.0 - r.field.max_height(),
            });
        let distance = bound_center.length();
        let angle = if bound_radius < distance {
            (bound_radius / distance).asin()
        } else {
            std::f32::consts::PI
        };
        beyond_horizon(
            to_eye,
            eye,
            t.to_world_dir(bound_center).normalized(),
            angle,
            (distance + bound_radius) / SPHERE_RADIUS,
            lowest - 0.02,
        )
    }

    /// Draws of the baked `layers` on body `host`: none while the planet is
    /// under `PROP_MIN_PX`, then each sector unless it is past the horizon
    /// or out of view.
    fn prop_layer_draws<'a>(
        &'a self,
        draws: &mut Vec<EffectDraw<'a>>,
        host: usize,
        layers: &'a PropLayers,
        width: u32,
        height: u32,
        threads: usize,
    ) {
        let projection = self.projection();
        // The whole layer hides while the planet is small.
        if self.radius_px(host, &projection, height) < PROP_MIN_PX {
            return;
        }
        let t = self.bodies[host].transform;
        let model = Uniforms {
            model_matrix: t.matrix(),
        };
        let light = t.to_object_dir(self.light_at(t.position)).normalized();
        for layer in &layers.layers {
            for sector in &layer.sectors {
                let center = t.position + t.to_world_dir(sector.bound_center) * t.scale;
                // Sectors past the horizon, then out of view.
                if self.patch_hidden(host, &projection, sector.bound_center, sector.bound_radius)
                    || !self.sphere_in_view(center, sector.bound_radius * t.scale, width, height)
                {
                    continue;
                }
                draws.push(EffectDraw {
                    mesh: &sector.mesh,
                    model,
                    options: solid_options(threads),
                    shader: match (layer.part, self.bodies[host].surface) {
                        (PropPart::Crystal, Some(PlanetMaterial::Ace(params))) => {
                            EffectShader::Crystal(CrystalShader {
                                light,
                                transform: t,
                                time: self.clock.time,
                                params,
                            })
                        }
                        _ => EffectShader::Prop(PropShader {
                            color: layer.part.color(),
                            light,
                            glow: layer.part.glow(),
                        }),
                    },
                });
            }
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
                    self.prop_layer_draws(&mut draws, host, props, width, height, threads);
                }
                AttachmentKind::Traffic => {
                    let Some(traffic) = &self.traffic else {
                        continue;
                    };
                    let projection = self.projection();
                    let t = self.bodies[host].transform;
                    for (i, (_, ship)) in traffic.ships.iter().enumerate() {
                        let (center, forward, up) = traffic.world_pose(i, self.clock.time, &t);
                        let r = ship.radius();
                        let local = t.to_object_dir(center - t.position) / t.scale;
                        // One test per ship: past the horizon, then out of
                        // view.
                        if self.patch_hidden(host, &projection, local, r / t.scale)
                            || !self.sphere_in_view(center, r, width, height)
                        {
                            continue;
                        }
                        let px = projected_radius_px(&projection, center, r, height);
                        // Object axes: +X right, +Y up, +Z back (the nose
                        // is -Z).
                        let right = forward.cross(up).normalized();
                        let to_ship = |v: Vector3| {
                            Vector3::new(v.dot(right), v.dot(up), -v.dot(forward)).normalized()
                        };
                        draws.push(EffectDraw {
                            mesh: ship.lod(px),
                            model: Uniforms {
                                model_matrix: ship.matrix(center, forward, up),
                            },
                            options: ship_options(threads),
                            shader: EffectShader::Neon(NeonShader {
                                body: NEON_BODY,
                                edge: NEON_EDGE,
                                light: to_ship(self.light_at(center)),
                                view: to_ship(eye_towards(&projection, center)),
                                fresnel: NEON_FRESNEL,
                            }),
                        });
                    }
                    // Trails, in the host's frame: the cycles' ones (and
                    // the cycles) hide with the props while the planet is
                    // small; each is tested on the sphere around its chord.
                    let small = self.radius_px(host, &projection, height) < PROP_MIN_PX;
                    let model = Uniforms {
                        model_matrix: t.matrix(),
                    };
                    let visible = |i: usize| {
                        if traffic.trails[i].is_empty() || (small && i >= traffic.ships.len()) {
                            return None;
                        }
                        let (head, tail) = traffic.trail_ends(i, self.clock.time);
                        let mid = (head + tail) * 0.5;
                        let r = (head - tail).length() * 0.5 + 0.05 * SPHERE_RADIUS;
                        let center = t.position + t.to_world_dir(mid) * t.scale;
                        let hidden = self.patch_hidden(host, &projection, mid, r)
                            || !self.sphere_in_view(center, r * t.scale, width, height);
                        (!hidden).then_some((head, tail))
                    };
                    if traffic.batched {
                        // One draw per group with a trail in view.
                        for g in 0..2 {
                            let (range, batch) = traffic.group(g);
                            if batch.is_empty() || !range.clone().any(|i| visible(i).is_some()) {
                                continue;
                            }
                            let mut shader = TrailBatchShader {
                                color: traffic.trail_color(range.start),
                                heads: [Vector3::zero(); MAX_TRAFFIC as usize],
                                tails: [Vector3::zero(); MAX_TRAFFIC as usize],
                                count: range.len(),
                            };
                            for (k, i) in range.enumerate() {
                                let (head, tail) = traffic.trail_ends(i, self.clock.time);
                                shader.heads[k] = head;
                                shader.tails[k] = tail;
                            }
                            draws.push(EffectDraw {
                                mesh: batch,
                                model,
                                options: ship_options(threads),
                                shader: EffectShader::TrailBatch(Box::new(shader)),
                            });
                        }
                    } else {
                        for (i, trail) in traffic.trails.iter().enumerate() {
                            let Some((head, tail)) = visible(i) else {
                                continue;
                            };
                            draws.push(EffectDraw {
                                mesh: trail,
                                model,
                                options: ship_options(threads),
                                shader: EffectShader::Trail(TrailShader {
                                    color: traffic.trail_color(i),
                                    head,
                                    tail,
                                }),
                            });
                        }
                    }
                    // Lightcycles: like the props, hidden while the planet
                    // is small, then one test each.
                    if traffic.cycles.is_empty() || small {
                        continue;
                    }
                    let r = traffic.cycle_radius(&t);
                    for i in 0..traffic.cycles.len() {
                        let (base, forward, up) = traffic.cycle_pose(i, self.clock.time, &t);
                        let center = base + up * r;
                        let local = t.to_object_dir(center - t.position) / t.scale;
                        if self.patch_hidden(host, &projection, local, r / t.scale)
                            || !self.sphere_in_view(center, r, width, height)
                        {
                            continue;
                        }
                        let right = forward.cross(up).normalized();
                        let to_cycle = |v: Vector3| {
                            Vector3::new(v.dot(right), v.dot(up), -v.dot(forward)).normalized()
                        };
                        draws.push(EffectDraw {
                            mesh: &traffic.cycle_mesh,
                            model: Uniforms {
                                model_matrix: traffic.cycle_matrix(i, self.clock.time, &t),
                            },
                            options: solid_options(threads),
                            shader: EffectShader::Neon(NeonShader {
                                body: NEON_BODY,
                                edge: CYCLE_EDGE,
                                light: to_cycle(self.light_at(center)),
                                view: to_cycle(eye_towards(&projection, center)),
                                fresnel: NEON_FRESNEL,
                            }),
                        });
                    }
                }
                AttachmentKind::City => {
                    let Some(city) = &self.city else { continue };
                    self.prop_layer_draws(&mut draws, host, city, width, height, threads);
                }
                AttachmentKind::Crystals => {
                    let Some(crystals) = &self.crystals else {
                        continue;
                    };
                    self.prop_layer_draws(&mut draws, host, crystals, width, height, threads);
                }
                AttachmentKind::Waterfalls => {
                    let Some(set) = &self.waterfalls else {
                        continue;
                    };
                    let projection = self.projection();
                    // Hidden while the planet is small, but from farther
                    // away than the props.
                    if self.radius_px(host, &projection, height)
                        < crate::solar::attach::waterfalls::FALL_MIN_PX
                    {
                        continue;
                    }
                    let t = self.bodies[host].transform;
                    let model = Uniforms {
                        model_matrix: t.matrix(),
                    };
                    let light = t.to_object_dir(self.light_at(t.position)).normalized();
                    for placed in set.falls.iter().chain(&set.monumental) {
                        let center = t.position + t.to_world_dir(placed.bound_center) * t.scale;
                        if self.patch_hidden(
                            host,
                            &projection,
                            placed.bound_center,
                            placed.bound_radius,
                        ) || !self.sphere_in_view(
                            center,
                            placed.bound_radius * t.scale,
                            width,
                            height,
                        ) {
                            continue;
                        }
                        draws.push(EffectDraw {
                            mesh: &placed.soup,
                            model,
                            // Double-sided: seen from behind the curtain too.
                            options: ShadedOptions {
                                cull_back_faces: false,
                                ..solid_options(threads)
                            },
                            shader: EffectShader::Waterfall(
                                placed.fall.shader(light, self.clock.time),
                            ),
                        });
                    }
                }
                AttachmentKind::Landmark => {
                    let Some(landmark) = &self.landmark else {
                        continue;
                    };
                    let projection = self.projection();
                    let t = self.bodies[host].transform;
                    let center = t.position + t.to_world_dir(landmark.bound_center) * t.scale;
                    // Seen from afar too: only the horizon and the view
                    // skip it.
                    if self.patch_hidden(
                        host,
                        &projection,
                        landmark.bound_center,
                        landmark.bound_radius,
                    ) || !self.sphere_in_view(
                        center,
                        landmark.bound_radius * t.scale,
                        width,
                        height,
                    ) {
                        continue;
                    }
                    let model = Uniforms {
                        model_matrix: t.matrix(),
                    };
                    let light = t.to_object_dir(self.light_at(t.position)).normalized();
                    for (mesh, part) in [
                        (&landmark.mesh, PropPart::BuildingBody),
                        (&landmark.edges, PropPart::BuildingEdge),
                    ] {
                        draws.push(EffectDraw {
                            mesh,
                            model,
                            options: solid_options(threads),
                            shader: EffectShader::Prop(PropShader {
                                color: part.color(),
                                light,
                                glow: part.glow(),
                            }),
                        });
                    }
                    // The heart disc turning over the top.
                    let time = self.clock.time;
                    draws.push(EffectDraw {
                        mesh: &landmark.disc,
                        model: Uniforms {
                            model_matrix: landmark.disc_matrix(&t, time),
                        },
                        options: solid_options(threads),
                        shader: EffectShader::Prop(PropShader {
                            color: HEART_DISC,
                            light: landmark.disc_light(&t, time, self.light_at(t.position)),
                            glow: 0.55,
                        }),
                    });
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
            let drawn = self.drawn_ship();
            let ship = &drawn;
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

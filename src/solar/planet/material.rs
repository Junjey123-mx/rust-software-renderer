//! Closed set of planet materials. `Lab6` wraps a laboratory 6 material and
//! delegates to the existing instance shading (`prepare_instance`,
//! `shade_instance`, `adapted_material`), so its pixels stay bit for bit.

use crate::celestial::{CelestialMaterial, PreparedShader};
use crate::pipeline::{FragmentView, SurfaceSample, Transform};
use crate::solar::instance::{
    BodyInstance, FrameContext, Quality, adapted_material, prepare_instance, shade_instance,
};
use raylib::math::Vector3;

/// Material of one solar body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlanetMaterial {
    /// A laboratory 6 body (The King's Crown, Crimson Entrance, House of
    /// Cards) with its own validated parameters.
    Lab6(CelestialMaterial),
}

/// A material with its per-frame values computed, ready for the fragments.
#[derive(Debug, Clone)]
pub enum PreparedPlanet {
    Lab6(PreparedShader),
}

impl PlanetMaterial {
    /// Per-frame values of `instance` drawn with this material.
    pub fn prepare(&self, instance: &BodyInstance, frame: &FrameContext) -> PreparedPlanet {
        match self {
            Self::Lab6(material) => PreparedPlanet::Lab6(prepare_instance(
                &BodyInstance {
                    material: *material,
                    ..*instance
                },
                frame,
            )),
        }
    }

    /// This material with its cost adapted to `quality` and the body's
    /// radius on screen (`adapted_material`).
    pub fn adapted(&self, quality: Quality, radius_px: f32) -> Self {
        match self {
            Self::Lab6(material) => Self::Lab6(adapted_material(material, quality, radius_px)),
        }
    }
}

impl PreparedPlanet {
    /// Color of one fragment of a body placed with `transform`.
    #[inline]
    pub fn shade(&self, transform: &Transform, s: &SurfaceSample, v: &FragmentView) -> Vector3 {
        match self {
            Self::Lab6(prepared) => shade_instance(prepared, transform, s, v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::celestial::Body;
    use crate::pipeline::{
        OrbitCamera, OrbitView, ShadedOptions, ViewPreset, ViewProjection, Viewport,
    };
    use crate::rasterization::TriangleMode;

    #[test]
    fn lab6_wrapper_shades_identically() {
        let view = OrbitView::for_world(
            OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
            8.0,
            Viewport::new(160, 120, 0.1),
        )
        .unwrap();
        let frame = FrameContext {
            projection: ViewProjection::Orbit(view),
            time: 3.7,
            light_world: Vector3::new(0.3, 0.8, -0.5).normalized(),
            mode: TriangleMode::FilledScanline,
            options: ShadedOptions::default(),
        };
        for body in Body::ALL {
            let transform = Transform {
                position: Vector3::new(1.0, 0.2, -2.0),
                yaw: 0.9,
                ..Transform::default()
            };
            let instance = BodyInstance::new(body, transform, 0.4);
            let direct = prepare_instance(&instance, &frame);
            let wrapped = PlanetMaterial::Lab6(instance.material).prepare(&instance, &frame);
            for k in 0..64 {
                let a = k as f32 * 0.37;
                let p = Vector3::new(a.cos() * 0.8, (a * 0.5).sin() * 0.4, a.sin() * 0.5);
                let s = SurfaceSample {
                    x: k,
                    y: k / 2,
                    depth: 1.0,
                    object_position: p,
                    normal: p.normalized(),
                    face_normal: p.normalized(),
                };
                let v = FragmentView {
                    view_direction: Vector3::new(0.1 * a.sin(), 0.3, 1.0).normalized(),
                };
                let x = shade_instance(&direct, &transform, &s, &v);
                let y = wrapped.shade(&transform, &s, &v);
                assert_eq!(
                    (x.x.to_bits(), x.y.to_bits(), x.z.to_bits()),
                    (y.x.to_bits(), y.y.to_bits(), y.z.to_bits()),
                    "{body:?} sample {k}"
                );
            }
            for px in [2.0, 20.0, 90.0] {
                assert_eq!(
                    PlanetMaterial::Lab6(instance.material).adapted(Quality::Adaptive, px),
                    PlanetMaterial::Lab6(adapted_material(
                        &instance.material,
                        Quality::Adaptive,
                        px
                    ))
                );
            }
        }
    }
}

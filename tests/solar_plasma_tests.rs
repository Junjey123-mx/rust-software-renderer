//! External plasma loops of The King's Crown in the solar scene.

use rust_software_renderer::solar::plasma_loops::CROWN_ARCS;

fn angle(a: raylib::math::Vector3, b: raylib::math::Vector3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos()
}

#[test]
fn the_crown_has_a_handful_of_slender_arcs() {
    assert!((3..=8).contains(&CROWN_ARCS.len()));
    for (i, a) in CROWN_ARCS.iter().enumerate() {
        assert!(a.span > 0.2 && a.span < 1.0, "{i}: span {}", a.span);
        assert!(a.height > 0.2 && a.height < 0.8, "{i}: height {}", a.height);
        assert!(a.width > 0.0 && a.width < a.height / 3.0, "{i}: width");
        assert!(a.intensity > 0.5 && a.intensity <= 1.5, "{i}: intensity");
        assert!(a.heat.abs() <= 0.3, "{i}: heat");
        assert!(a.latitude.abs() < 0.8, "{i}: away from the poles");
    }
}

#[test]
fn the_feet_lie_on_the_unit_sphere_half_a_span_from_the_center() {
    for (i, a) in CROWN_ARCS.iter().enumerate() {
        let [f0, f1] = a.feet();
        assert!((f0.length() - 1.0).abs() < 1e-5 && (f1.length() - 1.0).abs() < 1e-5);
        assert!((angle(f0, f1) - a.span).abs() < 1e-4, "{i}: span");
        assert!(
            ((f0 + f1).normalized() - a.center()).length() < 1e-5,
            "{i}: center"
        );
    }
}

#[test]
fn the_arcs_spread_around_the_star_without_sharing_feet() {
    let north = CROWN_ARCS.iter().filter(|a| a.latitude > 0.0).count();
    assert!(north > 0 && north < CROWN_ARCS.len(), "both hemispheres");
    for (i, a) in CROWN_ARCS.iter().enumerate() {
        for b in &CROWN_ARCS[i + 1..] {
            assert!(angle(a.center(), b.center()) > 0.6, "{i}: crowded");
            for fa in a.feet() {
                for fb in b.feet() {
                    assert!(angle(fa, fb) > 0.2, "{i}: shared foot");
                }
            }
        }
    }
}

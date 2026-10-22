use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{rasterize, rasterize_with_mode};
use rust_software_renderer::rasterization::{
    RasterBounds, TriangleMode, fill_barycentric, fill_scanline, triangle,
};
use rust_software_renderer::rendering::Fragment;
use std::collections::HashSet;

fn v(x: f32, y: f32) -> Vertex {
    Vertex::new(Vector3::new(x, y, 0.0))
}

fn tri() -> [Vertex; 3] {
    [v(10.0, 10.0), v(90.0, 20.0), v(40.0, 85.0)]
}

fn pixels(frags: &[Fragment]) -> HashSet<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

const B: RasterBounds = RasterBounds {
    width: 100,
    height: 100,
};

#[test]
fn wireframe_mode_is_the_original_rasterize() {
    let tris = [tri(), [v(1.0, 1.0), v(9.0, 1.0), v(1.0, 9.0)]];
    assert_eq!(
        rasterize_with_mode(&tris, TriangleMode::Wireframe, B),
        rasterize(&tris)
    );
    let mut expected = triangle(&tris[0][0], &tris[0][1], &tris[0][2]);
    expected.extend(triangle(&tris[1][0], &tris[1][1], &tris[1][2]));
    assert_eq!(rasterize(&tris), expected);
}

#[test]
fn filled_modes_dispatch_to_their_algorithms_and_fill_the_interior() {
    let t = tri();
    let bary = rasterize_with_mode(&[t], TriangleMode::FilledBarycentric, B);
    let scan = rasterize_with_mode(&[t], TriangleMode::FilledScanline, B);
    assert_eq!(bary, fill_barycentric(&t[0], &t[1], &t[2], B));
    assert_eq!(scan, fill_scanline(&t[0], &t[1], &t[2], B));
    let wire = pixels(&rasterize_with_mode(&[t], TriangleMode::Wireframe, B));
    for set in [pixels(&bary), pixels(&scan)] {
        assert!(set.contains(&(46, 38)), "interior pixel present");
        assert!(
            set.len() > wire.len() * 5,
            "filled output is much larger than the outline"
        );
    }
    assert!(!wire.contains(&(46, 38)));
}

#[test]
fn every_mode_keeps_the_vertex_color() {
    let red = Vector3::new(1.0, 0.0, 0.0);
    let t = [
        Vertex::new_with_color(Vector3::new(10.0, 10.0, 0.0), red),
        v(90.0, 20.0),
        v(40.0, 85.0),
    ];
    for mode in TriangleMode::ALL {
        let frags = rasterize_with_mode(&[t], mode, B);
        assert!(!frags.is_empty(), "{mode}");
        assert!(frags.iter().all(|f| f.color == red), "{mode}");
    }
}

#[test]
fn empty_input_and_multiple_triangles_preserve_order() {
    for mode in TriangleMode::ALL {
        assert!(rasterize_with_mode(&[], mode, B).is_empty());
    }
    let a = [v(0.0, 0.0), v(10.0, 0.0), v(0.0, 10.0)];
    let b = [v(50.0, 50.0), v(60.0, 50.0), v(50.0, 60.0)];
    for mode in TriangleMode::ALL {
        let both = rasterize_with_mode(&[a, b], mode, B);
        let first = rasterize_with_mode(&[a], mode, B);
        let second = rasterize_with_mode(&[b], mode, B);
        assert_eq!(both.len(), first.len() + second.len());
        assert_eq!(&both[..first.len()], &first[..]);
        assert_eq!(&both[first.len()..], &second[..]);
    }
}

#[test]
fn filled_modes_respect_bounds() {
    let t = tri();
    for mode in [
        TriangleMode::FilledBarycentric,
        TriangleMode::FilledScanline,
    ] {
        let small = rasterize_with_mode(&[t], mode, RasterBounds::new(50, 50));
        assert!(
            small
                .iter()
                .all(|f| f.position.x < 50.0 && f.position.y < 50.0)
        );
        assert!(!small.is_empty());
        assert!(rasterize_with_mode(&[t], mode, RasterBounds::new(0, 0)).is_empty());
    }
}

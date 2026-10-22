use raylib::math::{Vector2, Vector3};
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::rasterization::{line, triangle};
use std::collections::HashSet;

fn screen_vertex(x: f32, y: f32) -> Vertex {
    // Positions already in screen space, as in the lesson's first triangle.
    Vertex::new(Vector3::new(x, y, 0.0))
}

fn pixel_set(frags: &[rust_software_renderer::rendering::Fragment]) -> HashSet<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

#[test]
fn lesson_triangle_is_exactly_the_union_of_three_lines() {
    let a = screen_vertex(320.0, 100.0);
    let b = screen_vertex(150.0, 350.0);
    let c = screen_vertex(500.0, 350.0);
    let frags = triangle(&a, &b, &c);

    let w = Vertex::DEFAULT_COLOR;
    let ab = line(Vector2::new(320.0, 100.0), Vector2::new(150.0, 350.0), w);
    let bc = line(Vector2::new(150.0, 350.0), Vector2::new(500.0, 350.0), w);
    let ca = line(Vector2::new(500.0, 350.0), Vector2::new(320.0, 100.0), w);

    assert_eq!(frags.len(), ab.len() + bc.len() + ca.len());
    assert_eq!(&frags[..ab.len()], &ab[..]);
    assert_eq!(&frags[ab.len()..ab.len() + bc.len()], &bc[..]);
    assert_eq!(&frags[ab.len() + bc.len()..], &ca[..]);
}

#[test]
fn edges_contain_vertices_and_midpoints_but_not_interior() {
    let a = screen_vertex(320.0, 100.0);
    let b = screen_vertex(150.0, 350.0);
    let c = screen_vertex(500.0, 350.0);
    let px = pixel_set(&triangle(&a, &b, &c));

    for v in [(320, 100), (150, 350), (500, 350)] {
        assert!(px.contains(&v), "vertex {v:?} missing");
    }
    // Midpoint of the horizontal edge B-C and of A-B.
    assert!(px.contains(&(325, 350)));
    assert!(px.contains(&(235, 225)));
    // Interior point (centroid) must not be covered.
    assert!(!px.contains(&(323, 266)));
    assert!(!px.contains(&(320, 300)));
    // No horizontal run inside the triangle: row 300 has only two edge pixels
    // (plus at most their neighbours), far fewer than the ~136 px span.
    let row300 = px.iter().filter(|p| p.1 == 300).count();
    assert!(row300 <= 4, "row 300 has {row300} pixels");
}

#[test]
fn small_triangle_and_reversed_order_cover_the_same_pixels() {
    let a = screen_vertex(10.0, 10.0);
    let b = screen_vertex(18.0, 12.0);
    let c = screen_vertex(12.0, 20.0);
    let forward = pixel_set(&triangle(&a, &b, &c));
    let reversed = pixel_set(&triangle(&c, &b, &a));
    // Bresenham may differ by one pixel on ties when a segment is reversed,
    // so compare coverage up to a one-pixel neighbourhood.
    assert_eq!(forward.len(), reversed.len());
    for p in &reversed {
        assert!(
            forward
                .iter()
                .any(|q| (q.0 - p.0).abs() <= 1 && (q.1 - p.1).abs() <= 1),
            "{p:?} not near the forward outline"
        );
    }
    assert!(
        reversed.contains(&(10, 10))
            && reversed.contains(&(18, 12))
            && reversed.contains(&(12, 20))
    );
    assert!(
        forward.contains(&(10, 10)) && forward.contains(&(18, 12)) && forward.contains(&(12, 20))
    );
    assert!(!forward.contains(&(13, 14)), "interior must stay empty");
}

#[test]
fn triangle_uses_transformed_position_and_first_vertex_color() {
    let red = Vector3::new(1.0, 0.0, 0.0);
    let mut a = Vertex::new_with_color(Vector3::new(-1.0, -1.0, 0.0), red);
    let mut b = Vertex::new(Vector3::new(1.0, -1.0, 0.0));
    let mut c = Vertex::new(Vector3::new(0.0, 1.0, 0.0));
    a.transformed_position = Vector3::new(0.0, 0.0, 0.0);
    b.transformed_position = Vector3::new(4.0, 0.0, 0.0);
    c.transformed_position = Vector3::new(0.0, 4.0, 0.0);

    let frags = triangle(&a, &b, &c);
    let px = pixel_set(&frags);
    assert!(px.contains(&(0, 0)) && px.contains(&(4, 0)) && px.contains(&(0, 4)));
    assert!(
        !px.contains(&(-1, -1)),
        "original position must not be used"
    );
    assert!(frags.iter().all(|f| f.color == red));
}

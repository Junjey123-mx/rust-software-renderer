use raylib::math::Vector3;
use rust_software_renderer::geometry::{Mesh, Vertex};
use rust_software_renderer::pipeline::{
    IncompletePrimitive, Uniforms, primitive_assembly, vertex_shader,
};

fn v(x: f32) -> Vertex {
    Vertex::new(Vector3::new(x, 0.0, 0.0))
}

#[test]
fn six_vertices_become_two_ordered_triangles() {
    let input: Vec<Vertex> = (0..6).map(|i| v(i as f32)).collect();
    let tris = primitive_assembly(&input).unwrap();
    assert_eq!(tris.len(), 2);
    assert_eq!(tris[0], [v(0.0), v(1.0), v(2.0)]);
    assert_eq!(tris[1], [v(3.0), v(4.0), v(5.0)]);
}

#[test]
fn seven_vertices_are_rejected() {
    let input: Vec<Vertex> = (0..7).map(|i| v(i as f32)).collect();
    assert_eq!(
        primitive_assembly(&input).unwrap_err(),
        IncompletePrimitive { vertex_count: 7 }
    );
}

#[test]
fn empty_input_yields_no_triangles() {
    assert!(primitive_assembly(&[]).unwrap().is_empty());
}

#[test]
fn expansion_follows_indices_not_vertex_order() {
    // Four positions, two faces that both reuse position 0.
    let positions = vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    ];
    let mesh = Mesh::new(positions.clone(), vec![0, 1, 2, 0, 2, 3]).unwrap();
    let expanded = mesh.expand_vertices(Vertex::DEFAULT_COLOR);

    assert_eq!(expanded.len(), 6, "one Vertex per face corner");
    assert_eq!(mesh.vertices.len(), 4, "mesh stays compact");
    let got: Vec<Vector3> = expanded.iter().map(|v| v.position).collect();
    assert_eq!(
        got,
        vec![
            positions[0],
            positions[1],
            positions[2],
            positions[0],
            positions[2],
            positions[3]
        ]
    );

    let tris = primitive_assembly(&expanded).unwrap();
    assert_eq!(tris.len(), 2);
    assert_eq!(
        tris[1][0].position, positions[0],
        "position 0 reused by face 2"
    );
    assert_eq!(tris[1][2].position, positions[3]);
}

#[test]
fn shader_then_assembly_keeps_face_order() {
    let mesh = Mesh::new(
        vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ],
        vec![2, 0, 1],
    )
    .unwrap();
    let transformed: Vec<Vertex> = mesh
        .expand_vertices(Vertex::DEFAULT_COLOR)
        .iter()
        .map(|v| vertex_shader(v, &Uniforms::default()))
        .collect();
    let tris = primitive_assembly(&transformed).unwrap();
    assert_eq!(tris[0][0].transformed_position, Vector3::new(0.0, 1.0, 0.0));
    assert_eq!(tris[0][1].transformed_position, Vector3::new(0.0, 0.0, 0.0));
    assert_eq!(tris[0][2].transformed_position, Vector3::new(1.0, 0.0, 0.0));
}

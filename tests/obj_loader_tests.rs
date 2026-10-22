use raylib::math::Vector3;
use rust_software_renderer::io::{ObjError, load_obj, parse_obj};
use std::path::Path;

#[test]
fn parses_vertex_with_signs_and_decimals() {
    let mesh = parse_obj("v 1.25 -2 3.5\nv 0 0 0\nv 1 1 1\nf 1 2 3\n").unwrap();
    assert_eq!(mesh.vertices[0], Vector3::new(1.25, -2.0, 3.5));
}

#[test]
fn converts_one_based_face_to_zero_based() {
    let mesh = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
fn reads_only_position_index_from_slash_corners() {
    let src = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvn 0 0 1\nf 1/4/2 3/1/2 2/7/2\n";
    let mesh = parse_obj(src).unwrap();
    assert_eq!(mesh.indices, vec![0, 2, 1]);

    let mesh = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1//5 2//6 3//7\n").unwrap();
    assert_eq!(mesh.indices, vec![0, 1, 2]);

    let mesh = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/2 2/3 3/4\n").unwrap();
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
fn ignores_normals_textures_and_metadata() {
    let src = "\
# comment
mtllib something.mtl
o Thing
vn 9 9 9
vt 9 9
v 0 0 0
vn 8 8 8
v 1 0 0
v 0 1 0
s off
usemtl Mat
g group

f 1 2 3
";
    let mesh = parse_obj(src).unwrap();
    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.vertices[1], Vector3::new(1.0, 0.0, 0.0));
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
fn rejects_non_triangular_face() {
    let err = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\nf 1 2 3 4\n").unwrap_err();
    assert!(matches!(
        err,
        ObjError::NonTriangularFace {
            line: 5,
            corners: 4
        }
    ));
}

#[test]
fn rejects_zero_and_negative_indices() {
    let err = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 1 2\n").unwrap_err();
    assert!(
        matches!(err, ObjError::InvalidIndex { line: 4, .. }),
        "{err}"
    );

    let err = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -1 2 3\n").unwrap_err();
    assert!(
        matches!(err, ObjError::InvalidIndex { line: 4, .. }),
        "{err}"
    );
}

#[test]
fn rejects_index_out_of_range() {
    let err = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 999\n").unwrap_err();
    assert!(matches!(
        err,
        ObjError::IndexOutOfRange {
            line: 4,
            index: 999,
            vertex_count: 3
        }
    ));
}

#[test]
fn rejects_invalid_vertex_coordinates() {
    let err = parse_obj("v 1 two 3\nf 1 1 1\n").unwrap_err();
    assert!(matches!(err, ObjError::InvalidVertex { line: 1, .. }));

    let err = parse_obj("v 1 2\nf 1 1 1\n").unwrap_err();
    assert!(matches!(err, ObjError::InvalidVertex { line: 1, .. }));
}

#[test]
fn rejects_file_without_geometry() {
    assert!(matches!(parse_obj("").unwrap_err(), ObjError::NoGeometry));
    assert!(matches!(
        parse_obj("# only comments\n\n").unwrap_err(),
        ObjError::NoGeometry
    ));
    assert!(matches!(
        parse_obj("v 0 0 0\nv 1 0 0\n").unwrap_err(),
        ObjError::NoGeometry
    ));
}

#[test]
fn missing_file_is_an_io_error() {
    let err = load_obj(Path::new("tests/fixtures/does_not_exist.obj")).unwrap_err();
    assert!(matches!(err, ObjError::Io { .. }));
    assert!(err.to_string().contains("does_not_exist.obj"));
}

#[test]
fn loads_triangulated_cube_fixture() {
    let mesh = load_obj(Path::new("tests/fixtures/cube_triangulated.obj")).unwrap();
    assert_eq!(mesh.vertices.len(), 8);
    assert_eq!(mesh.indices.len(), 36);
    assert_eq!(mesh.triangle_count(), 12);
    assert!(mesh.indices.iter().all(|&i| i < 8));
    // First face `f 1//1 3//1 2//1`.
    assert_eq!(&mesh.indices[..3], &[0, 2, 1]);
}

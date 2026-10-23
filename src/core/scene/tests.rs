use super::*;

#[test]
fn every_generated_triangle_has_an_outward_finite_normal() {
    for primitive in [
        Primitive::Cube,
        Primitive::Sphere,
        Primitive::Cylinder,
        Primitive::Cone,
    ] {
        let mesh = primitive_mesh(primitive);
        assert!(!mesh.triangles.is_empty());
        for indices in mesh.triangles {
            let [a, b, c] = indices.map(|index| mesh.vertices[index]);
            assert!(a.finite() && b.finite() && c.finite());
            let normal = b.sub(a).cross(c.sub(a));
            assert!(
                normal.length() > 1.0e-8,
                "degenerate {} triangle",
                primitive.label()
            );
            let center = a.add(b).add(c).mul(1.0 / 3.0);
            assert!(
                normal.dot(center) > 0.0,
                "inward {} triangle",
                primitive.label()
            );
        }
    }
}

#[test]
fn near_clip_keeps_crossing_triangles_in_front_of_the_plane() {
    let polygon = clip_near([
        V3::new(-1.0, 0.0, -0.3),
        V3::new(1.0, 0.0, 1.0),
        V3::new(0.0, 1.0, 1.0),
    ]);
    assert_eq!(polygon.len(), 4);
    assert!(polygon.iter().all(|p| p.finite() && p.z >= NEAR));
    assert!(clip_near([V3::new(0.0, 0.0, -1.0); 3]).is_empty());
}

#[test]
fn transform_applies_scale_then_xyz_rotation_then_translation() {
    let transform = Transform {
        position: [3.0, 4.0, 5.0],
        rotation: [0.0, 0.0, 90.0],
        scale: [2.0, 1.0, 1.0],
    };
    let point = transform_vertex(V3::new(1.0, 0.0, 0.0), &transform);
    assert!((point.x - 3.0).abs() < 1.0e-10);
    assert!((point.y - 6.0).abs() < 1.0e-10);
    assert!((point.z - 5.0).abs() < 1.0e-10);
}

#[test]
fn added_meshes_have_finite_outward_faces() {
    for kind in [
        Primitive::Pyramid,
        Primitive::Prism,
        Primitive::Torus,
        Primitive::Capsule,
    ] {
        let mesh = primitive_mesh(kind);
        for indices in mesh.triangles {
            let [a, b, c] = indices.map(|i| mesh.vertices[i]);
            let center = a.add(b).add(c).mul(1.0 / 3.0);
            let normal = b.sub(a).cross(c.sub(a));
            let outward = if kind == Primitive::Torus {
                let angle = center.z.atan2(center.x);
                center.sub(V3::new(0.52 * angle.cos(), 0.0, 0.52 * angle.sin()))
            } else {
                center
            };
            assert!(
                normal.finite() && normal.length() > 1e-10 && normal.dot(outward) > 0.0,
                "{}",
                kind.label()
            );
        }
    }
}

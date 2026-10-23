//! Primitive meshes and object-space transforms.
use super::*;
pub(super) fn primitive_mesh(primitive: Primitive) -> Mesh {
    match primitive {
        Primitive::Pyramid | Primitive::Prism | Primitive::Torus | Primitive::Capsule => {
            super::shapes::mesh(primitive)
        }
        Primitive::Cube => Mesh {
            vertices: vec![
                V3::new(-RADIUS, -RADIUS, -RADIUS),
                V3::new(RADIUS, -RADIUS, -RADIUS),
                V3::new(RADIUS, RADIUS, -RADIUS),
                V3::new(-RADIUS, RADIUS, -RADIUS),
                V3::new(-RADIUS, -RADIUS, RADIUS),
                V3::new(RADIUS, -RADIUS, RADIUS),
                V3::new(RADIUS, RADIUS, RADIUS),
                V3::new(-RADIUS, RADIUS, RADIUS),
            ],
            triangles: vec![
                [4, 5, 6],
                [4, 6, 7],
                [0, 3, 2],
                [0, 2, 1],
                [0, 4, 7],
                [0, 7, 3],
                [1, 2, 6],
                [1, 6, 5],
                [0, 1, 5],
                [0, 5, 4],
                [3, 7, 6],
                [3, 6, 2],
            ],
        },
        Primitive::Sphere => {
            const SLICES: usize = 28;
            const RINGS: usize = 18;
            let mut mesh = Mesh {
                vertices: vec![V3::new(0.0, RADIUS, 0.0)],
                triangles: Vec::new(),
            };
            for ring in 1..RINGS {
                let phi = std::f64::consts::PI * ring as f64 / RINGS as f64;
                for slice in 0..SLICES {
                    let theta = std::f64::consts::TAU * slice as f64 / SLICES as f64;
                    mesh.vertices.push(V3::new(
                        RADIUS * phi.sin() * theta.cos(),
                        RADIUS * phi.cos(),
                        RADIUS * phi.sin() * theta.sin(),
                    ));
                }
            }
            let bottom = mesh.vertices.len();
            mesh.vertices.push(V3::new(0.0, -RADIUS, 0.0));
            for slice in 0..SLICES {
                let next = (slice + 1) % SLICES;
                mesh.triangles.push([0, 1 + next, 1 + slice]);
                for ring in 0..RINGS - 2 {
                    let a = 1 + ring * SLICES + slice;
                    let b = 1 + ring * SLICES + next;
                    let c = a + SLICES;
                    let d = b + SLICES;
                    mesh.triangles.push([a, b, c]);
                    mesh.triangles.push([b, d, c]);
                }
                let last = 1 + (RINGS - 2) * SLICES;
                mesh.triangles.push([bottom, last + slice, last + next]);
            }
            mesh
        }
        Primitive::Cylinder | Primitive::Cone => {
            const SLICES: usize = 32;
            let mut mesh = Mesh {
                vertices: Vec::new(),
                triangles: Vec::new(),
            };
            for slice in 0..SLICES {
                let theta = std::f64::consts::TAU * slice as f64 / SLICES as f64;
                mesh.vertices
                    .push(V3::new(RADIUS * theta.cos(), -RADIUS, RADIUS * theta.sin()));
            }
            let bottom_center = mesh.vertices.len();
            mesh.vertices.push(V3::new(0.0, -RADIUS, 0.0));
            if primitive == Primitive::Cone {
                let apex = mesh.vertices.len();
                mesh.vertices.push(V3::new(0.0, RADIUS, 0.0));
                for slice in 0..SLICES {
                    let next = (slice + 1) % SLICES;
                    mesh.triangles.push([slice, apex, next]);
                    mesh.triangles.push([bottom_center, slice, next]);
                }
            } else {
                let top_start = mesh.vertices.len();
                for slice in 0..SLICES {
                    let theta = std::f64::consts::TAU * slice as f64 / SLICES as f64;
                    mesh.vertices
                        .push(V3::new(RADIUS * theta.cos(), RADIUS, RADIUS * theta.sin()));
                }
                let top_center = mesh.vertices.len();
                mesh.vertices.push(V3::new(0.0, RADIUS, 0.0));
                for slice in 0..SLICES {
                    let next = (slice + 1) % SLICES;
                    mesh.triangles.push([slice, top_start + slice, next]);
                    mesh.triangles
                        .push([next, top_start + slice, top_start + next]);
                    mesh.triangles.push([bottom_center, slice, next]);
                    mesh.triangles
                        .push([top_center, top_start + next, top_start + slice]);
                }
            }
            mesh
        }
    }
}

pub(super) fn transform_vertex(v: V3, transform: &Transform) -> V3 {
    let mut p = V3::new(
        v.x * f64::from(transform.scale[0]),
        v.y * f64::from(transform.scale[1]),
        v.z * f64::from(transform.scale[2]),
    );
    let [rx, ry, rz] = transform
        .rotation
        .map(|angle| f64::from(angle).to_radians());
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    p = V3::new(p.x, p.y * cx - p.z * sx, p.y * sx + p.z * cx);
    p = V3::new(p.x * cy + p.z * sy, p.y, -p.x * sy + p.z * cy);
    p = V3::new(p.x * cz - p.y * sz, p.x * sz + p.y * cz, p.z);
    p.add(V3::new(
        f64::from(transform.position[0]),
        f64::from(transform.position[1]),
        f64::from(transform.position[2]),
    ))
}

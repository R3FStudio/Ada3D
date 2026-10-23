//! Additional closed primitive meshes, centered on the object origin.
use super::*;
pub(super) fn mesh(kind: Primitive) -> Mesh {
    if matches!(kind, Primitive::Pyramid | Primitive::Prism) {
        let (vertices, triangles) = if kind == Primitive::Pyramid {
            (
                vec![
                    V3::new(-0.75, -0.75, -0.75),
                    V3::new(0.75, -0.75, -0.75),
                    V3::new(0.75, -0.75, 0.75),
                    V3::new(-0.75, -0.75, 0.75),
                    V3::new(0.0, 0.75, 0.0),
                ],
                vec![
                    [0, 1, 2],
                    [0, 2, 3],
                    [0, 4, 1],
                    [1, 4, 2],
                    [2, 4, 3],
                    [3, 4, 0],
                ],
            )
        } else {
            (
                vec![
                    V3::new(-0.75, -0.6, -0.65),
                    V3::new(0.75, -0.6, -0.65),
                    V3::new(0.0, 0.75, -0.65),
                    V3::new(-0.75, -0.6, 0.65),
                    V3::new(0.75, -0.6, 0.65),
                    V3::new(0.0, 0.75, 0.65),
                ],
                vec![
                    [0, 2, 1],
                    [3, 4, 5],
                    [0, 1, 4],
                    [0, 4, 3],
                    [1, 2, 5],
                    [1, 5, 4],
                    [2, 0, 3],
                    [2, 3, 5],
                ],
            )
        };
        return Mesh {
            vertices,
            triangles,
        };
    }
    let (slices, rings) = (32, 20);
    let mut mesh = Mesh {
        vertices: Vec::new(),
        triangles: Vec::new(),
    };
    for row in 0..=rings {
        for col in 0..slices {
            let theta = col as f64 / slices as f64 * std::f64::consts::TAU;
            let (radius, y) = if kind == Primitive::Torus {
                let phi = row as f64 / rings as f64 * std::f64::consts::TAU;
                (0.52 + 0.23 * phi.cos(), 0.23 * phi.sin())
            } else {
                let phi = row as f64 / rings as f64 * std::f64::consts::PI;
                (
                    0.45 * phi.sin(),
                    0.45 * phi.cos() + if row <= rings / 2 { 0.3 } else { -0.3 },
                )
            };
            mesh.vertices
                .push(V3::new(radius * theta.cos(), y, radius * theta.sin()));
        }
    }
    for row in 0..rings {
        for col in 0..slices {
            let next = (col + 1) % slices;
            let a = row * slices + col;
            let b = row * slices + next;
            let c = (row + 1) * slices + col;
            let d = (row + 1) * slices + next;
            for triangle in [[a, b, c], [b, d, c]] {
                let [p, q, r] = triangle.map(|i| mesh.vertices[i]);
                if q.sub(p).cross(r.sub(p)).length() > 1e-10 {
                    mesh.triangles.push(if kind == Primitive::Torus {
                        [triangle[0], triangle[2], triangle[1]]
                    } else {
                        triangle
                    });
                }
            }
        }
    }
    mesh
}

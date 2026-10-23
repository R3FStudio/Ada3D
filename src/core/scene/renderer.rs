//! Projection, clipping, shading and rasterization.
use super::*;
struct View {
    eye: V3,
    right: V3,
    up: V3,
    forward: V3,
}

impl View {
    fn new(camera: &Camera) -> Self {
        let fallback = Camera::default();
        let yaw = if camera.yaw.is_finite() {
            camera.yaw
        } else {
            fallback.yaw
        };
        let pitch = if camera.pitch.is_finite() {
            camera.pitch.clamp(-85.0, 85.0)
        } else {
            fallback.pitch
        };
        let distance = if camera.distance.is_finite() {
            camera.distance.clamp(1.6, 500.0)
        } else {
            fallback.distance
        };
        let (sy, cy) = f64::from(yaw).to_radians().sin_cos();
        let (sp, cp) = f64::from(pitch).to_radians().sin_cos();
        let toward_eye = V3::new(sy * cp, sp, cy * cp);
        Self {
            eye: toward_eye.mul(f64::from(distance)),
            right: V3::new(cy, 0.0, -sy),
            up: V3::new(-sy * sp, cp, -cy * sp),
            forward: toward_eye.mul(-1.0),
        }
    }

    fn point(&self, p: V3) -> V3 {
        let d = p.sub(self.eye);
        V3::new(d.dot(self.right), d.dot(self.up), d.dot(self.forward))
    }
}

/// Clip in camera space before perspective division. Triangles crossing the
/// camera plane are never projected with negative or zero depth.
#[cfg(test)]
pub(super) fn clip_near(vertices: [V3; 3]) -> Vec<V3> {
    let mut clipped = Vec::with_capacity(4);
    let mut previous = vertices[2];
    for current in vertices {
        let previous_inside = previous.z >= NEAR;
        let current_inside = current.z >= NEAR;
        if previous_inside != current_inside {
            let t = (NEAR - previous.z) / (current.z - previous.z);
            let mut intersection = previous.add(current.sub(previous).mul(t));
            intersection.z = NEAR;
            clipped.push(intersection);
        }
        if current_inside {
            clipped.push(current);
        }
        previous = current;
    }
    clipped
}

fn clip_surface(vertices: [(V3, V3); 3]) -> Vec<(V3, V3)> {
    let mut out = Vec::with_capacity(4);
    let mut previous = vertices[2];
    for current in vertices {
        let a = previous.0.z >= NEAR;
        let b = current.0.z >= NEAR;
        if a != b {
            let t = (NEAR - previous.0.z) / (current.0.z - previous.0.z);
            let mut camera = previous.0.add(current.0.sub(previous.0).mul(t));
            camera.z = NEAR;
            out.push((camera, previous.1.add(current.1.sub(previous.1).mul(t))));
        }
        if b {
            out.push(current);
        }
        previous = current;
    }
    out
}

/// Position a dropped primitive on the camera-facing plane through the origin.
pub fn placement(camera: &Camera, point: [f32; 2], width: u32, height: u32) -> [f32; 3] {
    let view = View::new(camera);
    let focal = f64::from(width.min(height).max(1)) * 0.5 / (45.0_f64.to_radians() * 0.5).tan();
    let distance = view.eye.length();
    let p = view
        .right
        .mul((f64::from(point[0]) - 0.5) * f64::from(width) * distance / focal)
        .add(
            view.up
                .mul((0.5 - f64::from(point[1])) * f64::from(height) * distance / focal),
        );
    [p.x as f32, p.y as f32, p.z as f32].map(|v| v.clamp(-20.0, 20.0))
}

#[derive(Clone, Copy)]
struct ScreenPoint {
    x: f64,
    y: f64,
    inverse_depth: f64,
    local: V3,
}

#[derive(Clone, Copy)]
struct ScreenTriangle {
    points: [ScreenPoint; 3],
    id: u64,
    color: [u8; 4],
    object_index: usize,
    primitive: Primitive,
    normal: V3,
}

fn edge(a: ScreenPoint, b: ScreenPoint, x: f64, y: f64) -> f64 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

fn shaded(color: [u8; 4], normal: V3) -> [u8; 4] {
    let light = V3::new(-0.45, 0.8, 0.65).normalized();
    let intensity = 0.36 + 0.64 * normal.dot(light).max(0.0);
    [
        (f64::from(color[0]) * intensity).round().clamp(0.0, 255.0) as u8,
        (f64::from(color[1]) * intensity).round().clamp(0.0, 255.0) as u8,
        (f64::from(color[2]) * intensity).round().clamp(0.0, 255.0) as u8,
        color[3],
    ]
}

fn background_sample(
    background: Option<&Raster>,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> [u8; 4] {
    let Some(source) = background else {
        return [0; 4];
    };
    if source.width == 0 || source.height == 0 {
        return [0; 4];
    }
    let sx = u64::from(x) * u64::from(source.width) / u64::from(width);
    let sy = u64::from(y) * u64::from(source.height) / u64::from(height);
    let Some(index) = sy
        .checked_mul(u64::from(source.width))
        .and_then(|n| n.checked_add(sx))
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| usize::try_from(n).ok())
    else {
        return [0; 4];
    };
    let Some(end) = index.checked_add(4) else {
        return [0; 4];
    };
    source
        .pixels
        .get(index..end)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .unwrap_or([0; 4])
}

/// Straight-alpha source-over blending. Transparent canvas pixels remain
/// transparent, which makes PNG scene export useful in other applications.
fn composite(destination: [u8; 4], source: [u8; 4]) -> [u8; 4] {
    let sa = f64::from(source[3]) / 255.0;
    let da = f64::from(destination[3]) / 255.0;
    let alpha = sa + da * (1.0 - sa);
    if alpha <= 0.0 {
        return [0; 4];
    }
    let mut result = [0; 4];
    for channel in 0..3 {
        result[channel] = ((f64::from(source[channel]) * sa
            + f64::from(destination[channel]) * da * (1.0 - sa))
            / alpha)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    result[3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    result
}

fn rasterize_triangle(
    triangle: ScreenTriangle,
    width: u32,
    height: u32,
    mut fragment: impl FnMut(usize, f64, [u8; 4], u64, [f32; 2]),
    edges_only: bool,
) {
    let [a, b, c] = triangle.points;
    let area = edge(a, b, c.x, c.y);
    if !area.is_finite() || area.abs() < 1.0e-9 {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0).min(f64::from(width)) as u32;
    let max_x = a.x.max(b.x).max(c.x).ceil().max(0.0).min(f64::from(width)) as u32;
    let min_y =
        a.y.min(b.y)
            .min(c.y)
            .floor()
            .max(0.0)
            .min(f64::from(height)) as u32;
    let max_y = a.y.max(b.y).max(c.y).ceil().max(0.0).min(f64::from(height)) as u32;
    let edge_lengths = [
        (b.x - c.x).hypot(b.y - c.y),
        (c.x - a.x).hypot(c.y - a.y),
        (a.x - b.x).hypot(a.y - b.y),
    ];
    for y in min_y..max_y {
        for x in min_x..max_x {
            let px = f64::from(x) + 0.5;
            let py = f64::from(y) + 0.5;
            let e = [edge(b, c, px, py), edge(c, a, px, py), edge(a, b, px, py)];
            let weights = e.map(|value| value / area);
            if weights.iter().any(|weight| *weight < -1.0e-9) {
                continue;
            }
            if edges_only
                && !(0..3).any(|i| edge_lengths[i] > 0.0 && e[i].abs() / edge_lengths[i] <= 0.9)
            {
                continue;
            }
            let inverse_depth = weights[0] * a.inverse_depth
                + weights[1] * b.inverse_depth
                + weights[2] * c.inverse_depth;
            if !inverse_depth.is_finite() || inverse_depth <= 0.0 {
                continue;
            }
            let local = a
                .local
                .mul(weights[0] * a.inverse_depth)
                .add(b.local.mul(weights[1] * b.inverse_depth))
                .add(c.local.mul(weights[2] * c.inverse_depth))
                .mul(1.0 / inverse_depth);
            let uv = super::paint::surface_uv(triangle.primitive, local);
            fragment(
                (y * width + x) as usize,
                1.0 / inverse_depth,
                triangle.color,
                triangle.id,
                uv,
            );
        }
    }
}

/// Render independently generated meshes over an optional 2D canvas.
///
/// Output sides are clamped to 1..=2048. Invalid transforms are skipped. Fully
/// transparent objects do not render or pick. Partially transparent surfaces
/// blend with the canvas after nearest-surface depth resolution; layered 3D
/// transparency is intentionally outside this renderer's current scope.
pub fn render(
    width: u32,
    height: u32,
    objects: &[SceneObject],
    camera: &Camera,
    background: Option<&Raster>,
    selected: Option<u64>,
    wireframe: bool,
) -> RenderedScene {
    render_with_progress(
        RenderInput {
            width,
            height,
            objects,
            camera,
            background,
            selected,
            wireframe,
        },
        &mut |_| Ok(()),
    )
    .expect("no-op progress callback")
}

pub struct RenderInput<'a> {
    pub width: u32,
    pub height: u32,
    pub objects: &'a [SceneObject],
    pub camera: &'a Camera,
    pub background: Option<&'a Raster>,
    pub selected: Option<u64>,
    pub wireframe: bool,
}
pub fn render_with_progress(
    input: RenderInput<'_>,
    report: &mut crate::progress::Reporter<'_>,
) -> Result<RenderedScene, String> {
    let RenderInput {
        width,
        height,
        objects,
        camera,
        background,
        selected,
        wireframe,
    } = input;
    let width = width.clamp(1, MAX_RENDER_SIDE);
    let height = height.clamp(1, MAX_RENDER_SIDE);
    let count = (width * height) as usize;
    let mut raster = Raster::new(width, height, [0; 4]);
    let mut object_ids = vec![None; count];
    let mut surface_uvs = vec![[0.0; 2]; count];
    let mut surface_colors = vec![[0u8; 4]; count];
    let mut depth = vec![f64::INFINITY; count];
    let view = View::new(camera);
    let focal = f64::from(width.min(height)) * 0.5 / (45.0_f64.to_radians() * 0.5).tan();
    let project = |(point, local): (V3, V3)| ScreenPoint {
        x: f64::from(width) * 0.5 + point.x / point.z * focal,
        y: f64::from(height) * 0.5 - point.y / point.z * focal,
        inverse_depth: 1.0 / point.z,
        local,
    };
    let mut triangles = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        report(crate::progress::Progress::new(
            "Projecting scene",
            index as u64,
            objects.len() as u64,
            "objects",
        ))?;
        if object.validate().is_err() || object.color[3] == 0 && object.paint.is_none() {
            continue;
        }
        let mesh = primitive_mesh(object.primitive);
        let vertices: Vec<V3> = mesh
            .vertices
            .iter()
            .map(|vertex| transform_vertex(*vertex, &object.transform))
            .collect();
        for indices in mesh.triangles {
            let world = indices.map(|index| vertices[index]);
            if world.iter().any(|vertex| !vertex.finite()) {
                continue;
            }
            let normal = world[1]
                .sub(world[0])
                .cross(world[2].sub(world[0]))
                .normalized();
            let color = shaded(object.color, normal);
            let clipped =
                clip_surface(indices.map(|i| (view.point(vertices[i]), mesh.vertices[i])));
            for i in 1..clipped.len().saturating_sub(1) {
                let points = [clipped[0], clipped[i], clipped[i + 1]].map(project);
                if points
                    .iter()
                    .all(|p| p.x.is_finite() && p.y.is_finite() && p.inverse_depth.is_finite())
                {
                    triangles.push(ScreenTriangle {
                        points,
                        id: object.id,
                        color,
                        object_index: index,
                        primitive: object.primitive,
                        normal,
                    });
                }
            }
        }
    }
    report(crate::progress::Progress::new(
        "Projecting scene",
        objects.len() as u64,
        objects.len() as u64,
        "objects",
    ))?;
    for (index, triangle) in triangles.iter().enumerate() {
        if index % 16 == 0 {
            report(crate::progress::Progress::new(
                "Rendering surfaces",
                index as u64,
                triangles.len() as u64,
                "triangles",
            ))?;
        }
        rasterize_triangle(
            *triangle,
            width,
            height,
            |index, z, color, id, uv| {
                if z < depth[index] {
                    depth[index] = z;
                    if !wireframe {
                        let object = &objects[triangle.object_index];
                        surface_colors[index] = if object.paint.is_some() {
                            shaded(object.surface_color(uv), triangle.normal)
                        } else {
                            color
                        };
                        surface_uvs[index] = uv;
                        object_ids[index] = Some(id);
                    }
                }
            },
            false,
        );
    }
    report(crate::progress::Progress::new(
        "Rendering surfaces",
        triangles.len() as u64,
        triangles.len() as u64,
        "triangles",
    ))?;
    if wireframe {
        for triangle in &triangles {
            rasterize_triangle(
                *triangle,
                width,
                height,
                |index, z, color, id, uv| {
                    // Allow only floating-point roundoff between coplanar
                    // triangles, rather than a visible world-space depth bias.
                    let tolerance = (depth[index].abs() * f64::EPSILON * 64.0).max(1.0e-12);
                    if z <= depth[index] + tolerance {
                        let object = &objects[triangle.object_index];
                        surface_colors[index] = if object.paint.is_some() {
                            shaded(object.surface_color(uv), triangle.normal)
                        } else {
                            color
                        };
                        surface_uvs[index] = uv;
                        object_ids[index] = Some(id);
                    }
                },
                true,
            );
        }
    }
    report(crate::progress::Progress::new(
        "Compositing canvas",
        0,
        u64::from(height),
        "rows",
    ))?;
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            let pixel = composite(
                background_sample(background, x, y, width, height),
                surface_colors[index],
            );
            raster.pixels[index * 4..index * 4 + 4].copy_from_slice(&pixel);
        }
        if (y + 1) % 16 == 0 {
            report(crate::progress::Progress::new(
                "Compositing canvas",
                u64::from(y + 1),
                u64::from(height),
                "rows",
            ))?;
        }
    }
    report(crate::progress::Progress::new(
        "Compositing canvas",
        u64::from(height),
        u64::from(height),
        "rows",
    ))?;
    if let Some(selected_id) = selected {
        // Restrict silhouette work to the selected visible footprint. A
        // selected object outside the view adds no per-pixel neighborhood pass.
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        for (index, id) in object_ids.iter().enumerate() {
            if *id != Some(selected_id) {
                continue;
            }
            let x = index as u32 % width;
            let y = index as u32 / width;
            bounds = Some(match bounds {
                Some((left, top, right, bottom)) => {
                    (left.min(x), top.min(y), right.max(x), bottom.max(y))
                }
                None => (x, y, x, y),
            });
        }
        let (left, top, right, bottom) = bounds
            .map(|(l, t, r, b)| {
                (
                    l.saturating_sub(2),
                    t.saturating_sub(2),
                    (r + 3).min(width),
                    (b + 3).min(height),
                )
            })
            .unwrap_or((0, 0, 0, 0));
        for y in top..bottom {
            for x in left..right {
                let index = (y * width + x) as usize;
                let own = object_ids[index];
                // Do not paint a highlight over another object's surface.
                if own.is_some() && own != Some(selected_id) {
                    continue;
                }
                let mut boundary = false;
                'neighbors: for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        if dx * dx + dy * dy > 4 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        let neighbor =
                            if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                                object_ids[(ny as u32 * width + nx as u32) as usize]
                            } else {
                                None
                            };
                        if (own == Some(selected_id)) != (neighbor == Some(selected_id)) {
                            boundary = true;
                            break 'neighbors;
                        }
                    }
                }
                if boundary {
                    raster.pixels[index * 4..index * 4 + 4].copy_from_slice(&SELECTION_COLOR);
                }
            }
        }
    }
    Ok(RenderedScene {
        raster,
        object_ids,
        surface_uvs,
    })
}

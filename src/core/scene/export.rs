//! Wavefront OBJ export.
use super::*;
/// Write original world-space primitive geometry as a Wavefront OBJ file.
/// The export includes generated face normals. OBJ does not store our object
/// color or editable primitive parameters; the native project retains them.
pub fn export_obj(path: &Path, objects: &[SceneObject]) -> Result<(), String> {
    export_obj_with_progress(path, objects, &mut |_| Ok(()))
}

pub fn export_obj_with_progress(
    path: &Path,
    objects: &[SceneObject],
    report: &mut crate::progress::Reporter<'_>,
) -> Result<(), String> {
    if objects.len() > crate::document::MAX_OBJECTS {
        return Err(format!(
            "OBJ export supports at most {} objects",
            crate::document::MAX_OBJECTS
        ));
    }
    report(crate::progress::Progress::new(
        "Checking scene",
        0,
        objects.len() as u64,
        "objects",
    ))?;
    for (index, object) in objects.iter().enumerate() {
        report(crate::progress::Progress::new(
            "Checking scene",
            index as u64,
            objects.len() as u64,
            "objects",
        ))?;
        object.validate()?;
    }
    let mut text = String::from(
        "# Ada3D Studio: 3D geometry\n# Units: world units; Y up; rotations baked into vertices\n",
    );
    let mut vertex_offset = 1usize;
    let mut normal_offset = 1usize;
    for (index, object) in objects.iter().enumerate() {
        report(crate::progress::Progress::new(
            "Building geometry",
            index as u64,
            objects.len() as u64,
            "objects",
        ))?;
        let mesh = primitive_mesh(object.primitive);
        let vertices: Vec<V3> = mesh
            .vertices
            .iter()
            .map(|vertex| transform_vertex(*vertex, &object.transform))
            .collect();
        if vertices.iter().any(|vertex| !vertex.finite()) {
            return Err(format!(
                "{} exceeds the geometry coordinate range",
                object.name
            ));
        }
        let name: String = object
            .name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        writeln!(
            text,
            "o {}_{}",
            if name.is_empty() { "Object" } else { &name },
            object.id
        )
        .map_err(|error| error.to_string())?;
        for vertex in &vertices {
            writeln!(text, "v {:.8} {:.8} {:.8}", vertex.x, vertex.y, vertex.z)
                .map_err(|error| error.to_string())?;
        }
        for indices in &mesh.triangles {
            let normal = vertices[indices[1]]
                .sub(vertices[indices[0]])
                .cross(vertices[indices[2]].sub(vertices[indices[0]]))
                .normalized();
            writeln!(text, "vn {:.8} {:.8} {:.8}", normal.x, normal.y, normal.z)
                .map_err(|error| error.to_string())?;
        }
        for (index, indices) in mesh.triangles.iter().enumerate() {
            let n = normal_offset + index;
            writeln!(
                text,
                "f {}//{} {}//{} {}//{}",
                vertex_offset + indices[0],
                n,
                vertex_offset + indices[1],
                n,
                vertex_offset + indices[2],
                n
            )
            .map_err(|error| error.to_string())?;
        }
        vertex_offset += vertices.len();
        normal_offset += mesh.triangles.len();
    }
    // Reuse the native project/image save path: write and sync a complete
    // sibling using create_new, then rename over the destination. Filesystem
    // replacement semantics apply; no destination is deleted beforehand.
    report(crate::progress::Progress::new(
        "Building geometry",
        objects.len() as u64,
        objects.len() as u64,
        "objects",
    ))?;
    crate::persistence::atomic_write_with_progress(path, text.as_bytes(), report)
        .map_err(|error| format!("Could not export OBJ: {error}"))
}

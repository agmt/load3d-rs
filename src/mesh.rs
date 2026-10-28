use crate::error::Res;
use crate::types::*;

pub(crate) fn update_vertex_first_index(mesh: &mut Mesh) {
    for v in &mut mesh.vertex_first_index {
        *v = NO_INDEX;
    }
    let num_vertices = mesh.num_vertices as u32;
    for ix in 0..mesh.num_indices {
        let vx = mesh.vertex_indices[ix];
        if vx < num_vertices && mesh.vertex_first_index[vx as usize] == NO_INDEX {
            mesh.vertex_first_index[vx as usize] = ix as u32;
        }
    }
}

pub(crate) fn finalize_mesh(mesh: &mut Mesh) -> Res<()> {
    if mesh.vertices.is_empty() {
        mesh.vertices = mesh.vertex_position.values.clone();
    }
    if mesh.vertex_indices.is_empty() {
        mesh.vertex_indices = mesh.vertex_position.indices.clone();
    }

    mesh.num_vertices = mesh.vertices.len();
    mesh.num_indices = mesh.vertex_indices.len();
    mesh.num_faces = mesh.faces.len();

    if mesh.num_triangles == 0 || mesh.max_face_triangles == 0 {
        let mut num_triangles = 0;
        let mut max_face_triangles = 0;
        let mut num_bad_faces = [0usize; 3];
        for face in &mesh.faces {
            if face.num_indices >= 3 {
                let tris = face.num_indices as usize - 2;
                num_triangles += tris;
                max_face_triangles = max_face_triangles.max(tris);
            } else {
                num_bad_faces[face.num_indices as usize] += 1;
            }
        }
        mesh.num_triangles = num_triangles;
        mesh.max_face_triangles = max_face_triangles;
        mesh.num_empty_faces = num_bad_faces[0];
        mesh.num_point_faces = num_bad_faces[1];
        mesh.num_line_faces = num_bad_faces[2];
    }

    if !mesh.skinned_position.exists {
        mesh.skinned_is_local = true;
        mesh.skinned_position = mesh.vertex_position.clone();
        mesh.skinned_normal = mesh.vertex_normal.clone();
    }

    if mesh.vertex_first_index.is_empty() {
        mesh.vertex_first_index = vec![NO_INDEX; mesh.num_vertices];
        update_vertex_first_index(mesh);
    }

    if mesh.uv_sets.is_empty() && mesh.vertex_uv.exists {
        mesh.uv_sets.push(UvSet {
            vertex_uv: mesh.vertex_uv.clone(),
            vertex_tangent: mesh.vertex_tangent.clone(),
            vertex_bitangent: mesh.vertex_bitangent.clone(),
            ..Default::default()
        });
    }

    if mesh.color_sets.is_empty() && mesh.vertex_color.exists {
        mesh.color_sets.push(ColorSet { vertex_color: mesh.vertex_color.clone(), ..Default::default() });
    }

    patch_mesh_reals(mesh);
    Ok(())
}

pub(crate) fn finalize_mesh_material(mesh: &mut Mesh) -> Res<()> {
    let num_materials = mesh.materials.len();
    let num_parts = mesh.material_parts.len();
    let num_faces = mesh.faces.len();
    let has_face_material = !mesh.face_material.is_empty();

    for i in 0..num_faces {
        let face = mesh.faces[i];
        let mut mat_ix = 0;
        if has_face_material {
            mat_ix = mesh.face_material[i];
            if mat_ix as usize >= num_materials {
                mesh.face_material[i] = 0;
                mat_ix = 0;
            }
        }
        if let Some(part) = mesh.material_parts.get_mut(mat_ix as usize) {
            mesh_part_add_face(part, face.num_indices);
        }
    }

    if num_parts > 0 {
        for (index, part) in mesh.material_parts.iter_mut().enumerate() {
            part.index = index as u32;
            part.face_indices = Vec::with_capacity(part.num_faces);
        }
        for i in 0..num_faces {
            let mat_ix = if has_face_material { mesh.face_material[i] } else { 0 };
            if (mat_ix as usize) < num_parts {
                mesh.material_parts[mat_ix as usize].face_indices.push(i as u32);
            }
        }

        let parts = &mesh.material_parts;
        let mut order: Vec<u32> = (0..num_parts as u32).collect();
        // `ufbxi_material_part_usage_less` in ufbx.c
        order.sort_by(|&a, &b| {
            let (pa, pb) = (&parts[a as usize], &parts[b as usize]);
            if pa.face_indices.is_empty() || pb.face_indices.is_empty() {
                if pa.face_indices.len() == pb.face_indices.len() {
                    return a.cmp(&b);
                }
                return pb.face_indices.len().cmp(&pa.face_indices.len());
            }
            pa.face_indices[0].cmp(&pb.face_indices[0])
        });
        mesh.material_part_usage_order = order;
    }
    Ok(())
}

pub(crate) fn patch_mesh_reals(mesh: &mut Mesh) {
    mesh.vertex_position.value_reals = 3;
    mesh.vertex_normal.value_reals = 3;
    mesh.vertex_uv.value_reals = 2;
    mesh.vertex_tangent.value_reals = 3;
    mesh.vertex_bitangent.value_reals = 3;
    mesh.vertex_color.value_reals = 4;
    mesh.vertex_crease.value_reals = 1;
    mesh.skinned_position.value_reals = 3;
    mesh.skinned_normal.value_reals = 3;
    for s in &mut mesh.uv_sets {
        s.vertex_uv.value_reals = 2;
        s.vertex_tangent.value_reals = 3;
        s.vertex_bitangent.value_reals = 3;
    }
    for s in &mut mesh.color_sets {
        s.vertex_color.value_reals = 4;
    }
}

pub(crate) fn mesh_part_add_face(part: &mut MeshPart, num_indices: u32) {
    part.num_faces += 1;
    match num_indices {
        0 => part.num_empty_faces += 1,
        1 => part.num_point_faces += 1,
        2 => part.num_line_faces += 1,
        n => part.num_triangles += n as usize - 2,
    }
}

pub(crate) fn update_face_groups(mesh: &mut Mesh) {
    let num_groups = mesh.face_group_parts.len();
    if num_groups == 0 {
        return;
    }
    let mut parts: Vec<MeshPart> = (0..num_groups).map(|i| MeshPart { index: i as u32, ..Default::default() }).collect();
    for (i, &g) in mesh.face_group.iter().enumerate() {
        let part = &mut parts[g as usize];
        mesh_part_add_face(part, mesh.faces[i].num_indices);
        part.face_indices.push(i as u32);
    }
    mesh.face_group_parts = parts;
}

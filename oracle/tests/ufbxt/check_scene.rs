use load3d::*;

macro_rules! typed_lists {
    ($($element_type:ident => $list:ident),* $(,)?) => {
        fn typed_element(scene: &Scene, element_type: ElementType, index: usize) -> Option<&Element> {
            match element_type {
                $(ElementType::$element_type => scene.$list.get(index).map(|e| &e.element),)*
            }
        }

        fn typed_count(scene: &Scene, element_type: ElementType) -> usize {
            match element_type {
                $(ElementType::$element_type => scene.$list.len(),)*
            }
        }
    };
}

typed_lists! {
    Unknown => unknowns,
    Node => nodes,
    Mesh => meshes,
    Light => lights,
    Camera => cameras,
    Bone => bones,
    Empty => empties,
    LineCurve => line_curves,
    NurbsCurve => nurbs_curves,
    NurbsSurface => nurbs_surfaces,
    NurbsTrimSurface => nurbs_trim_surfaces,
    NurbsTrimBoundary => nurbs_trim_boundaries,
    ProceduralGeometry => procedural_geometries,
    StereoCamera => stereo_cameras,
    CameraSwitcher => camera_switchers,
    Marker => markers,
    LodGroup => lod_groups,
    SkinDeformer => skin_deformers,
    SkinCluster => skin_clusters,
    BlendDeformer => blend_deformers,
    BlendChannel => blend_channels,
    BlendShape => blend_shapes,
    CacheDeformer => cache_deformers,
    CacheFile => cache_files,
    Material => materials,
    Texture => textures,
    Video => videos,
    Shader => shaders,
    ShaderBinding => shader_bindings,
    AnimStack => anim_stacks,
    AnimLayer => anim_layers,
    AnimValue => anim_values,
    AnimCurve => anim_curves,
    DisplayLayer => display_layers,
    SelectionSet => selection_sets,
    SelectionNode => selection_nodes,
    Character => characters,
    Constraint => constraints,
    AudioLayer => audio_layers,
    AudioClip => audio_clips,
    Pose => poses,
    MetadataObject => metadata_objects,
}

fn element_by_id(scene: &Scene, id: ElementId) -> Option<&Element> {
    let r = scene.elements.get(id as usize)?;
    typed_element(scene, r.element_type, r.index)
}

/// C's `a.data == b.data` of two fields that share one array: two copies here, equal bit for bit
fn same_array<T: std::ops::Index<usize, Output = Real>>(a: &[T], b: &[T], reals: usize) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (0..reals).all(|i| x[i].to_bits() == y[i].to_bits()))
}

fn is_utf8(s: &[u8]) -> bool {
    // {min,max} (inclusive) of each byte of each form, after https://www.w3.org/International/questions/qa-forms-utf-8
    const UTF8_FORMS: [[[u8; 2]; 4]; 8] = [
        [[0xc2, 0xdf], [0x80, 0xbf], [0, 0], [0, 0]],
        [[0xe0, 0xe0], [0xa0, 0xbf], [0x80, 0xbf], [0, 0]],
        [[0xe1, 0xec], [0x80, 0xbf], [0x80, 0xbf], [0, 0]],
        [[0xee, 0xef], [0x80, 0xbf], [0x80, 0xbf], [0, 0]],
        [[0xed, 0xed], [0x80, 0x9f], [0x80, 0xbf], [0, 0]],
        [[0xf0, 0xf0], [0x90, 0xbf], [0x80, 0xbf], [0x80, 0xbf]],
        [[0xf1, 0xf3], [0x80, 0xbf], [0x80, 0xbf], [0x80, 0xbf]],
        [[0xf4, 0xf4], [0x80, 0x8f], [0x80, 0xbf], [0x80, 0xbf]],
    ];

    let length = s.len();
    let mut pos = 0;
    while pos < length {
        let mut c = s[pos];
        if (0x01..=0x7f).contains(&c) {
            pos += 1;
            continue;
        }

        let Some(form) = UTF8_FORMS.iter().position(|f| c >= f[0][0] && c <= f[0][1]) else {
            return false;
        };

        let mut len = 1;
        while len < 4 {
            let range = UTF8_FORMS[form][len];
            if range[1] == 0 {
                break;
            }
            if pos + len >= length {
                return false;
            }
            c = s[pos + len];
            if c < range[0] || c > range[1] {
                return false;
            }
            len += 1;
        }

        pos += len;
    }

    true
}

fn float_equal(a: f64, b: f64) -> bool {
    if a.is_nan() { b.is_nan() } else { a == b }
}

pub fn check_string(s: &str) {
    ufbxt_assert!(!s.as_bytes().contains(&0));
    ufbxt_assert!(is_utf8(s.as_bytes()));
}

fn check_element_ptr_any(scene: &Scene, id: Option<ElementId>) {
    let Some(id) = id else { return };
    let r = scene.elements.get(id as usize).copied();
    ufbxt_assert!(r.is_some());
    let r = r.unwrap();
    let element = typed_element(scene, r.element_type, r.index);
    ufbxt_assert!(element.is_some_and(|e| e.element_id == id));
    ufbxt_assert!(element.is_some_and(|e| e.element_type == r.element_type && e.typed_id as usize == r.index));
}

fn check_element_ptr(scene: &Scene, index: Option<usize>, element_type: ElementType) {
    let Some(index) = index else { return };
    let element = typed_element(scene, element_type, index);
    ufbxt_assert!(element.is_some_and(|e| e.element_type == element_type));
    let element = element.unwrap();
    ufbxt_assert!(scene.elements.get(element.element_id as usize) == Some(&ElementRef { element_type, index }));
    ufbxt_assert!(element.typed_id as usize == index);
}

fn check_vertex_element<T>(scene: &Scene, mesh: &Mesh, elem: &VertexAttrib<T>) {
    if !elem.exists {
        ufbxt_assert!(elem.values.is_empty());
        ufbxt_assert!(elem.indices.is_empty());
        return;
    }

    ufbxt_assert!(elem.values.len() <= i32::MAX as usize);
    ufbxt_assert!(elem.indices.len() == mesh.num_indices);
    if mesh.num_indices > 0 {
        ufbxt_assert!(!elem.indices.is_empty());
    }

    for i in 0..mesh.num_indices {
        let ix = elem.indices[i];
        ufbxt_assert!((ix as usize) < elem.values.len() || (ix == NO_INDEX && scene.metadata.may_contain_no_index));
    }

    if !elem.values_w.is_empty() {
        ufbxt_assert!(elem.values_w.len() == elem.values.len());
    }
}

fn check_mesh_list_count(_scene: &Scene, _mesh: &Mesh, count: usize, required_count: usize, optional: bool) {
    if count > 0 || !optional {
        ufbxt_assert!(count == required_count);
    }
}

fn check_mesh_list<T>(scene: &Scene, mesh: &Mesh, list: &[T], required_count: usize, optional: bool) {
    check_mesh_list_count(scene, mesh, list.len(), required_count, optional);
}

fn check_props(scene: &Scene, props: &Props, top: bool) {
    let mut prev: Option<&Prop> = None;
    for prop in &props.props {
        check_string(&prop.name);
        check_string(&prop.value_str);

        if let Some(prev) = prev {
            ufbxt_assert!(prop.internal_key >= prev.internal_key);
            ufbxt_assert!(prop.name.as_bytes() >= prev.name.as_bytes());
        }

        let r = props.find(&prop.name);
        if top {
            ufbxt_assert!(r.is_some_and(|r| std::ptr::eq(r, prop)));
        } else {
            ufbxt_assert!(r.is_some());
        }

        let vec_flag = prop.flags & (prop_flags::VALUE_REAL | prop_flags::VALUE_VEC2 | prop_flags::VALUE_VEC3 | prop_flags::VALUE_VEC4);
        ufbxt_assert!((vec_flag & vec_flag.wrapping_sub(1)) == 0);

        prev = Some(prop);
    }

    if let Some(defaults) = &props.defaults {
        check_props(scene, defaults, false);
    }
}

fn check_element_cast(scene: &Scene, element: Option<&Element>) {
    let id = element.map_or(NO_INDEX, |e| e.element_id);
    let element_type = element.map(|e| e.element_type);
    ufbxt_assert!(scene.as_unknown(id).is_none() || element_type == Some(ElementType::Unknown));
    ufbxt_assert!(scene.as_node(id).is_none() || element_type == Some(ElementType::Node));
    ufbxt_assert!(scene.as_mesh(id).is_none() || element_type == Some(ElementType::Mesh));
    ufbxt_assert!(scene.as_light(id).is_none() || element_type == Some(ElementType::Light));
    ufbxt_assert!(scene.as_camera(id).is_none() || element_type == Some(ElementType::Camera));
    ufbxt_assert!(scene.as_bone(id).is_none() || element_type == Some(ElementType::Bone));
    ufbxt_assert!(scene.as_empty(id).is_none() || element_type == Some(ElementType::Empty));
    ufbxt_assert!(scene.as_line_curve(id).is_none() || element_type == Some(ElementType::LineCurve));
    ufbxt_assert!(scene.as_nurbs_curve(id).is_none() || element_type == Some(ElementType::NurbsCurve));
    ufbxt_assert!(scene.as_nurbs_surface(id).is_none() || element_type == Some(ElementType::NurbsSurface));
    ufbxt_assert!(scene.as_nurbs_trim_surface(id).is_none() || element_type == Some(ElementType::NurbsTrimSurface));
    ufbxt_assert!(scene.as_nurbs_trim_boundary(id).is_none() || element_type == Some(ElementType::NurbsTrimBoundary));
    ufbxt_assert!(scene.as_procedural_geometry(id).is_none() || element_type == Some(ElementType::ProceduralGeometry));
    ufbxt_assert!(scene.as_stereo_camera(id).is_none() || element_type == Some(ElementType::StereoCamera));
    ufbxt_assert!(scene.as_camera_switcher(id).is_none() || element_type == Some(ElementType::CameraSwitcher));
    ufbxt_assert!(scene.as_marker(id).is_none() || element_type == Some(ElementType::Marker));
    ufbxt_assert!(scene.as_lod_group(id).is_none() || element_type == Some(ElementType::LodGroup));
    ufbxt_assert!(scene.as_skin_deformer(id).is_none() || element_type == Some(ElementType::SkinDeformer));
    ufbxt_assert!(scene.as_skin_cluster(id).is_none() || element_type == Some(ElementType::SkinCluster));
    ufbxt_assert!(scene.as_blend_deformer(id).is_none() || element_type == Some(ElementType::BlendDeformer));
    ufbxt_assert!(scene.as_blend_channel(id).is_none() || element_type == Some(ElementType::BlendChannel));
    ufbxt_assert!(scene.as_blend_shape(id).is_none() || element_type == Some(ElementType::BlendShape));
    ufbxt_assert!(scene.as_cache_deformer(id).is_none() || element_type == Some(ElementType::CacheDeformer));
    ufbxt_assert!(scene.as_cache_file(id).is_none() || element_type == Some(ElementType::CacheFile));
    ufbxt_assert!(scene.as_material(id).is_none() || element_type == Some(ElementType::Material));
    ufbxt_assert!(scene.as_texture(id).is_none() || element_type == Some(ElementType::Texture));
    ufbxt_assert!(scene.as_video(id).is_none() || element_type == Some(ElementType::Video));
    ufbxt_assert!(scene.as_shader(id).is_none() || element_type == Some(ElementType::Shader));
    ufbxt_assert!(scene.as_shader_binding(id).is_none() || element_type == Some(ElementType::ShaderBinding));
    ufbxt_assert!(scene.as_anim_stack(id).is_none() || element_type == Some(ElementType::AnimStack));
    ufbxt_assert!(scene.as_anim_layer(id).is_none() || element_type == Some(ElementType::AnimLayer));
    ufbxt_assert!(scene.as_anim_value(id).is_none() || element_type == Some(ElementType::AnimValue));
    ufbxt_assert!(scene.as_anim_curve(id).is_none() || element_type == Some(ElementType::AnimCurve));
    ufbxt_assert!(scene.as_display_layer(id).is_none() || element_type == Some(ElementType::DisplayLayer));
    ufbxt_assert!(scene.as_selection_set(id).is_none() || element_type == Some(ElementType::SelectionSet));
    ufbxt_assert!(scene.as_selection_node(id).is_none() || element_type == Some(ElementType::SelectionNode));
    ufbxt_assert!(scene.as_character(id).is_none() || element_type == Some(ElementType::Character));
    ufbxt_assert!(scene.as_constraint(id).is_none() || element_type == Some(ElementType::Constraint));
    ufbxt_assert!(scene.as_audio_layer(id).is_none() || element_type == Some(ElementType::AudioLayer));
    ufbxt_assert!(scene.as_audio_clip(id).is_none() || element_type == Some(ElementType::AudioClip));
    ufbxt_assert!(scene.as_pose(id).is_none() || element_type == Some(ElementType::Pose));
    ufbxt_assert!(scene.as_metadata_object(id).is_none() || element_type == Some(ElementType::MetadataObject));
}

fn check_element(scene: &Scene, element: &Element) {
    if scene.dom_root.is_some() && scene.metadata.file_format == FileFormat::Fbx {
        let mut requires_dom = true;
        let version = scene.metadata.version;

        if element.element_type == ElementType::Node && scene.nodes.get(element.typed_id as usize).is_some_and(|n| n.is_root) {
            requires_dom = false;
        } else if version < 6000 && element.element_type == ElementType::AnimStack {
            requires_dom = false;
        } else if version < 6000 && element.element_type == ElementType::AnimLayer {
            requires_dom = false;
        } else if version < 6000 && element.element_type == ElementType::SkinDeformer {
            requires_dom = false;
        }

        if requires_dom {
            ufbxt_assert!(element.dom_node.is_some());
        }
    }

    check_element_cast(scene, Some(element));
    check_props(scene, &element.props, true);
    check_string(&element.name);
    ufbxt_assert!(element_by_id(scene, element.element_id).is_some_and(|e| std::ptr::eq(e, element)));

    ufbxt_assert!(element_by_id(scene, element.element_id).is_some_and(|e| std::ptr::eq(e, element)));
    ufbxt_assert!(typed_element(scene, element.element_type, element.typed_id as usize).is_some_and(|e| std::ptr::eq(e, element)));

    for (i, c) in element.connections_src.iter().enumerate() {
        check_string(&c.src_prop);
        check_string(&c.dst_prop);
        ufbxt_assert!(c.src == element.element_id);
        check_element_ptr_any(scene, Some(c.dst));
        if i > 0 {
            let prev = &element.connections_src[i - 1];
            let cmp = prev.src_prop.as_bytes().cmp(c.src_prop.as_bytes());
            ufbxt_assert!(cmp.is_le());
            if cmp.is_eq() {
                ufbxt_assert!(prev.dst_prop.as_bytes() <= c.dst_prop.as_bytes());
            }
        }
    }

    for (i, c) in element.connections_dst.iter().enumerate() {
        check_string(&c.src_prop);
        check_string(&c.dst_prop);
        ufbxt_assert!(c.dst == element.element_id);
        check_element_ptr_any(scene, Some(c.src));
        if i > 0 {
            let prev = &element.connections_dst[i - 1];
            let cmp = prev.dst_prop.as_bytes().cmp(c.dst_prop.as_bytes());
            ufbxt_assert!(cmp.is_le());
            if cmp.is_eq() {
                ufbxt_assert!(prev.src_prop.as_bytes() <= c.src_prop.as_bytes());
            }
        }
    }

    if element.element_type >= ElementType::FIRST_ATTRIB && element.element_type <= ElementType::LAST_ATTRIB {
        for &node in &element.instances {
            check_element_ptr(scene, Some(node), ElementType::Node);
            ufbxt_assert!(scene.nodes[node].all_attribs.contains(&element.element_id));
        }
    } else {
        ufbxt_assert!(element.instances.is_empty());
    }
}

fn check_unknown(_scene: &Scene, unknown: &Unknown) {
    check_string(&unknown.type_);
    check_string(&unknown.sub_type);
    check_string(&unknown.super_type);
}

fn check_node(scene: &Scene, index: usize) {
    let node = &scene.nodes[index];
    check_element_ptr(scene, node.parent, ElementType::Node);
    if let Some(parent) = node.parent {
        ufbxt_assert!(scene.nodes[parent].children.contains(&index));
    }

    for &child in &node.children {
        ufbxt_assert!(scene.nodes[child].parent == Some(index));
    }

    for &attrib in &node.all_attribs {
        check_element_ptr_any(scene, Some(attrib));
        ufbxt_assert!(element_by_id(scene, attrib).is_some_and(|a| a.instances.contains(&index)));
    }

    if !node.all_attribs.is_empty() {
        ufbxt_assert!(node.attrib == Some(node.all_attribs[0]));
        if node.all_attribs.len() == 1 {
            ufbxt_assert!(element_by_id(scene, node.all_attribs[0]).is_some_and(|a| node.attrib_type == a.element_type));
        }
    }

    check_element_ptr(scene, node.mesh, ElementType::Mesh);
    check_element_ptr(scene, node.light, ElementType::Light);
    check_element_ptr(scene, node.camera, ElementType::Camera);
    check_element_ptr(scene, node.bone, ElementType::Bone);
    check_element_ptr(scene, node.inherit_scale_node, ElementType::Node);
    check_element_ptr(scene, node.scale_helper, ElementType::Node);
    check_element_ptr(scene, node.geometry_transform_helper, ElementType::Node);

    match node.attrib_type {
        ElementType::Mesh => ufbxt_assert!(node.mesh.is_some()),
        ElementType::Light => ufbxt_assert!(node.light.is_some()),
        ElementType::Camera => ufbxt_assert!(node.camera.is_some()),
        ElementType::Bone => ufbxt_assert!(node.bone.is_some()),
        _ => {}
    }

    for &material in &node.materials {
        ufbxt_assert!(material.is_some());
        check_element_ptr(scene, material, ElementType::Material);
    }

    if node.is_root {
        ufbxt_assert!(!node.has_geometry_transform);
    }

    let duplicate_ids = scene.metadata.has_warning[WarningType::DuplicateObjectId as usize];

    if let Some(helper) = node.geometry_transform_helper {
        ufbxt_assert!(scene.nodes[helper].is_geometry_transform_helper);

        if duplicate_ids {
            // in broken files a node may have several geometry transform helpers
            ufbxt_assert!(!node.children.is_empty());
            ufbxt_assert!(scene.nodes[node.children[0]].is_geometry_transform_helper);
        } else {
            ufbxt_assert!(node.children.first() == Some(&helper));
        }
    }

    if node.is_geometry_transform_helper && !duplicate_ids {
        ufbxt_assert!(node.parent.is_some());
        ufbxt_assert!(node.parent.is_some_and(|p| scene.nodes[p].geometry_transform_helper == Some(index)));
    }

    check_element_ptr(scene, node.bind_pose, ElementType::Pose);
    if let Some(bind_pose) = node.bind_pose {
        let pose = get_bone_pose(&scene.poses[bind_pose], index);
        ufbxt_assert!(pose.is_some());
        ufbxt_assert!(pose.is_some_and(|p| p.bone_node == index));
    }
}

fn check_mesh_part(_scene: &Scene, mesh: &Mesh, part: &MeshPart, index: u32, source_list: Option<&[u32]>) {
    ufbxt_assert!(part.index == index);
    ufbxt_assert!(part.num_faces <= mesh.num_faces);
    ufbxt_assert!(part.face_indices.len() == part.num_faces);

    let mut mat_bad_faces = [0usize; 3];
    let mut mat_tris = 0;
    for &ix in &part.face_indices[..part.num_faces] {
        let face = mesh.faces[ix as usize];
        if face.num_indices >= 3 {
            mat_tris += face.num_indices as usize - 2;
        } else {
            mat_bad_faces[face.num_indices as usize] += 1;
        }
        if let Some(source_list) = source_list {
            ufbxt_assert!(source_list[ix as usize] == index);
        }
    }

    ufbxt_assert!(part.num_triangles == mat_tris);
    ufbxt_assert!(part.num_empty_faces == mat_bad_faces[0]);
    ufbxt_assert!(part.num_point_faces == mat_bad_faces[1]);
    ufbxt_assert!(part.num_line_faces == mat_bad_faces[2]);
}

fn list_data(list: &[u32]) -> Option<&[u32]> {
    if list.is_empty() { None } else { Some(list) }
}

pub fn check_mesh(scene: &Scene, mesh: &Mesh) {
    ufbxt_assert!(same_array(&mesh.vertices, &mesh.vertex_position.values, 3));
    ufbxt_assert!(mesh.vertices.len() == mesh.num_vertices);
    ufbxt_assert!(mesh.vertex_indices == mesh.vertex_position.indices);
    ufbxt_assert!(mesh.vertex_indices.len() == mesh.num_indices);
    ufbxt_assert!(mesh.vertex_first_index.len() == mesh.num_vertices);

    for ii in 0..mesh.num_indices {
        let vi = mesh.vertex_indices[ii];
        if vi != NO_INDEX {
            ufbxt_assert!((vi as usize) < mesh.num_vertices);
            if !mesh.reversed_winding {
                ufbxt_assert!(mesh.vertex_first_index[vi as usize] as usize <= ii);
            }
        }
    }

    for vi in 0..mesh.num_vertices {
        let ii = mesh.vertex_first_index[vi] as i32;
        if ii >= 0 {
            ufbxt_assert!(mesh.vertex_indices[ii as usize] as usize == vi);
        } else {
            ufbxt_assert!(ii as u32 == NO_INDEX);
        }
    }

    check_vertex_element(scene, mesh, &mesh.vertex_position);
    check_vertex_element(scene, mesh, &mesh.vertex_normal);
    check_vertex_element(scene, mesh, &mesh.vertex_tangent);
    check_vertex_element(scene, mesh, &mesh.vertex_bitangent);
    check_vertex_element(scene, mesh, &mesh.vertex_uv);
    check_vertex_element(scene, mesh, &mesh.vertex_color);
    check_vertex_element(scene, mesh, &mesh.skinned_position);
    check_vertex_element(scene, mesh, &mesh.skinned_normal);

    ufbxt_assert!(mesh.num_vertices == mesh.vertex_position.values.len());
    ufbxt_assert!(mesh.num_triangles <= mesh.num_indices);

    ufbxt_assert!(mesh.vertex_position.value_reals == 3);
    ufbxt_assert!(mesh.vertex_normal.value_reals == 3);
    ufbxt_assert!(mesh.vertex_tangent.value_reals == 3);
    ufbxt_assert!(mesh.vertex_bitangent.value_reals == 3);
    ufbxt_assert!(mesh.vertex_uv.value_reals == 2);
    ufbxt_assert!(mesh.vertex_color.value_reals == 4);
    ufbxt_assert!(mesh.vertex_crease.value_reals == 1);

    if !scene.metadata.may_contain_missing_vertex_position {
        ufbxt_assert!(mesh.vertex_position.exists);
    }

    if mesh.vertex_position.exists {
        ufbxt_assert!(mesh.vertex_position.unique_per_vertex);
    }

    check_mesh_list(scene, mesh, &mesh.edges, mesh.num_edges, false);
    check_mesh_list(scene, mesh, &mesh.edge_crease, mesh.num_edges, true);
    check_mesh_list(scene, mesh, &mesh.edge_smoothing, mesh.num_edges, true);
    check_mesh_list(scene, mesh, &mesh.edge_visibility, mesh.num_edges, true);

    check_mesh_list(scene, mesh, &mesh.face_material, mesh.num_faces, true);
    check_mesh_list(scene, mesh, &mesh.face_smoothing, mesh.num_faces, true);
    check_mesh_list(scene, mesh, &mesh.face_group, mesh.num_faces, true);
    check_mesh_list(scene, mesh, &mesh.face_hole, mesh.num_faces, true);

    let mut num_triangles = 0;
    let mut max_face_triangles = 0;
    let mut num_bad_faces = [0usize; 3];

    let mut prev_end = 0;
    ufbxt_assert!(mesh.faces.len() == mesh.num_faces);
    for face in &mesh.faces[..mesh.num_faces] {
        ufbxt_assert!(face.index_begin == prev_end);
        prev_end = face.index_begin + face.num_indices;
        ufbxt_assert!(prev_end as usize <= mesh.num_indices);

        if face.num_indices >= 3 {
            let tris = face.num_indices as usize - 2;
            num_triangles += tris;
            if tris > max_face_triangles {
                max_face_triangles = tris;
            }
        } else {
            num_bad_faces[face.num_indices as usize] += 1;
        }
    }

    ufbxt_assert!(mesh.num_triangles == num_triangles);
    ufbxt_assert!(mesh.max_face_triangles == max_face_triangles);
    ufbxt_assert!(mesh.num_empty_faces == num_bad_faces[0]);
    ufbxt_assert!(mesh.num_point_faces == num_bad_faces[1]);
    ufbxt_assert!(mesh.num_line_faces == num_bad_faces[2]);

    if !mesh.from_tessellated_nurbs && !mesh.subdivision_evaluated {
        ufbxt_assert!(scene.metadata.max_face_triangles >= max_face_triangles);
    }

    ufbxt_assert!(mesh.edges.len() == mesh.num_edges);
    for edge in &mesh.edges[..mesh.num_edges] {
        ufbxt_assert!((edge.a as usize) < mesh.num_indices);
        ufbxt_assert!((edge.b as usize) < mesh.num_indices);
    }

    for (i, set) in mesh.uv_sets.iter().enumerate() {
        ufbxt_assert!(set.vertex_uv.value_reals == 2);
        ufbxt_assert!(set.vertex_tangent.value_reals == 3);
        ufbxt_assert!(set.vertex_bitangent.value_reals == 3);

        if i == 0 {
            ufbxt_assert!(same_array(&mesh.vertex_uv.values, &set.vertex_uv.values, 2));
            ufbxt_assert!(mesh.vertex_uv.values.len() == set.vertex_uv.values.len());
            ufbxt_assert!(mesh.vertex_uv.indices == set.vertex_uv.indices);
            ufbxt_assert!(mesh.vertex_uv.indices.len() == set.vertex_uv.indices.len());
        }
        check_string(&set.name);
        check_vertex_element(scene, mesh, &set.vertex_uv);
        check_vertex_element(scene, mesh, &set.vertex_tangent);
        check_vertex_element(scene, mesh, &set.vertex_bitangent);
    }

    for (i, set) in mesh.color_sets.iter().enumerate() {
        ufbxt_assert!(set.vertex_color.value_reals == 4);

        if i == 0 {
            ufbxt_assert!(same_array(&mesh.vertex_color.values, &set.vertex_color.values, 4));
            ufbxt_assert!(mesh.vertex_color.values.len() == set.vertex_color.values.len());
            ufbxt_assert!(mesh.vertex_color.indices == set.vertex_color.indices);
            ufbxt_assert!(mesh.vertex_color.indices.len() == set.vertex_color.indices.len());
        }
        check_string(&set.name);
        check_vertex_element(scene, mesh, &set.vertex_color);
    }

    for edge in &mesh.edges[..mesh.num_edges] {
        ufbxt_assert!((edge.a as usize) < mesh.num_indices);
        ufbxt_assert!((edge.b as usize) < mesh.num_indices);
    }

    if !mesh.face_material.is_empty() {
        ufbxt_assert!(mesh.face_material.len() == mesh.num_faces);
        for &material in &mesh.face_material[..mesh.num_faces] {
            let material = material as i32;
            ufbxt_assert!(material >= 0 && (material as usize) < mesh.materials.len());
        }
    } else {
        ufbxt_assert!(mesh.face_material.is_empty());
    }

    let mut total_material_faces = 0;
    let mut total_material_tris = 0;
    if !mesh.materials.is_empty() {
        ufbxt_assert!(mesh.material_parts.is_empty() || mesh.material_parts.len() == mesh.materials.len());

        for (i, &mat) in mesh.materials.iter().enumerate() {
            check_element_ptr(scene, mat, ElementType::Material);

            if !mesh.material_parts.is_empty() {
                check_mesh_part(scene, mesh, &mesh.material_parts[i], i as u32, list_data(&mesh.face_material));
                total_material_faces += mesh.material_parts[i].num_faces;
                total_material_tris += mesh.material_parts[i].num_triangles;
            }
        }
    } else if !mesh.material_parts.is_empty() {
        ufbxt_assert!(mesh.material_parts.len() == 1);
        check_mesh_part(scene, mesh, &mesh.material_parts[0], 0, None);
        total_material_faces += mesh.material_parts[0].num_faces;
        total_material_tris += mesh.material_parts[0].num_triangles;
    }
    if !mesh.material_parts.is_empty() {
        ufbxt_assert!(total_material_faces == mesh.num_faces);
        ufbxt_assert!(total_material_tris == mesh.num_triangles);
    }

    let usage_order = &mesh.material_part_usage_order;
    for i in 0..usage_order.len() {
        ufbxt_assert!((usage_order[i] as usize) < mesh.material_parts.len());
        if i > 0 {
            let pa = &mesh.material_parts[usage_order[i - 1] as usize];
            let pb = &mesh.material_parts[usage_order[i] as usize];
            if !pa.face_indices.is_empty() && !pb.face_indices.is_empty() {
                ufbxt_assert!(pa.face_indices[0] < pb.face_indices[0]);
            } else if pa.face_indices.len() == pb.face_indices.len() {
                ufbxt_assert!(usage_order[i - 1] < usage_order[i]);
            } else {
                ufbxt_assert!(pa.face_indices.len() > pb.face_indices.len());
            }
        }
    }
    ufbxt_assert!(usage_order.len() == mesh.material_parts.len());

    if !mesh.face_group.is_empty() {
        ufbxt_assert!(mesh.face_group.len() == mesh.num_faces);
        for &group in &mesh.face_group[..mesh.num_faces] {
            ufbxt_assert!((group as usize) < mesh.face_groups.len());
        }
    } else {
        ufbxt_assert!(mesh.face_groups.is_empty());
    }

    ufbxt_assert!(mesh.face_group_parts.is_empty() || mesh.face_group_parts.len() == mesh.face_groups.len());

    if !mesh.face_groups.is_empty() {
        let mut total_group_faces = 0;
        let mut total_group_tris = 0;
        for (i, group) in mesh.face_groups.iter().enumerate() {
            if !mesh.face_group_parts.is_empty() {
                check_mesh_part(scene, mesh, &mesh.face_group_parts[i], i as u32, list_data(&mesh.face_group));
                total_group_faces += mesh.face_group_parts[i].num_faces;
                total_group_tris += mesh.face_group_parts[i].num_triangles;
            }

            if i > 0 {
                ufbxt_assert!(group.id >= mesh.face_groups[i - 1].id);
            }

            check_string(&group.name);
        }

        if !mesh.face_group_parts.is_empty() {
            ufbxt_assert!(total_group_faces == mesh.num_faces);
            ufbxt_assert!(total_group_tris == mesh.num_triangles);
        }
    }

    for &deformer in &mesh.skin_deformers {
        ufbxt_assert!(scene.skin_deformers[deformer].vertices.len() >= mesh.num_vertices);
        check_element_ptr(scene, Some(deformer), ElementType::SkinDeformer);
    }
    for &deformer in &mesh.blend_deformers {
        check_element_ptr(scene, Some(deformer), ElementType::BlendDeformer);
    }
    for &deformer in &mesh.cache_deformers {
        check_element_ptr(scene, Some(deformer), ElementType::CacheDeformer);
    }
    for &deformer in &mesh.all_deformers {
        check_element_ptr_any(scene, Some(deformer));
    }

    for &instance in &mesh.element.instances {
        ufbxt_assert!(scene.nodes[instance].materials.len() >= mesh.materials.len());
    }

    if mesh.vertex_uv.exists {
        ufbxt_assert!(!mesh.uv_sets.is_empty());
    }
    if mesh.vertex_color.exists {
        ufbxt_assert!(!mesh.color_sets.is_empty());
    }
}

fn check_light(_scene: &Scene, _light: &Light) {
}

fn check_camera(_scene: &Scene, camera: &Camera) {
    match camera.projection_mode {
        ProjectionMode::Perspective => {
            ufbxt_assert!(float_equal(camera.projection_plane.x, camera.field_of_view_tan.x));
            ufbxt_assert!(float_equal(camera.projection_plane.y, camera.field_of_view_tan.y));
        }
        ProjectionMode::Orthographic => {
            ufbxt_assert!(float_equal(camera.projection_plane.x, camera.orthographic_size.x));
            ufbxt_assert!(float_equal(camera.projection_plane.y, camera.orthographic_size.y));
        }
    }
}

fn check_bone(_scene: &Scene, _bone: &Bone) {}

fn check_empty(_scene: &Scene, _empty: &Empty) {}

fn check_line_curve(scene: &Scene, line: &LineCurve) {
    for &ix in &line.point_indices {
        if scene.metadata.may_contain_no_index && ix == NO_INDEX {
            continue;
        }
        ufbxt_assert!((ix as usize) < line.control_points.len());
    }

    for seg in &line.segments {
        ufbxt_assert!(seg.num_indices as usize <= line.point_indices.len());
        ufbxt_assert!(seg.index_begin as usize <= line.point_indices.len() - seg.num_indices as usize);
    }
}

fn check_nurbs_basis(_scene: &Scene, basis: &NurbsBasis) {
    if basis.valid {
        ufbxt_assert!(basis.order > 1);
        ufbxt_assert!(basis.knot_vector.len() >= 2 * (basis.order as usize - 1) + 1);
    }
}

fn check_nurbs_curve(scene: &Scene, curve: &NurbsCurve) {
    check_nurbs_basis(scene, &curve.basis);
}

fn check_nurbs_surface(scene: &Scene, surface: &NurbsSurface) {
    check_nurbs_basis(scene, &surface.basis_u);
    check_nurbs_basis(scene, &surface.basis_v);

    if surface.num_control_points_u > 0 && surface.num_control_points_v > 0 {
        ufbxt_assert!(surface.num_control_points_u <= u32::MAX as usize / surface.num_control_points_v);
        if !scene.metadata.geometry_ignored {
            ufbxt_assert!(surface.control_points.len() == surface.num_control_points_u * surface.num_control_points_v);
        }
    } else {
        ufbxt_assert!(surface.control_points.is_empty());
    }
}

fn check_nurbs_trim_surface(_scene: &Scene, _surface: &NurbsTrimSurface) {}

fn check_nurbs_trim_boundary(_scene: &Scene, _boundary: &NurbsTrimBoundary) {}

fn check_procedural_geometry(_scene: &Scene, _geometry: &ProceduralGeometry) {}

fn check_stereo_camera(scene: &Scene, camera: &StereoCamera) {
    check_element_ptr(scene, camera.left, ElementType::Camera);
    check_element_ptr(scene, camera.right, ElementType::Camera);
}

fn check_camera_switcher(_scene: &Scene, _switcher: &CameraSwitcher) {}

fn check_marker(_scene: &Scene, _marker: &Marker) {
}

fn check_lod_level(_scene: &Scene, _lod_group: &LodGroup, _level: &LodLevel) {
}

fn check_lod_group(scene: &Scene, lod_group: &LodGroup) {
    for level in &lod_group.lod_levels {
        check_lod_level(scene, lod_group, level);
    }
}

fn check_skin_deformer(scene: &Scene, deformer: &SkinDeformer) {
    for &cluster in &deformer.clusters {
        check_element_ptr(scene, Some(cluster), ElementType::SkinCluster);
        if !scene.metadata.may_contain_broken_elements {
            ufbxt_assert!(scene.skin_clusters[cluster].bone_node.is_some());
        }
    }
    for vertex in &deformer.vertices {
        ufbxt_assert!(vertex.weight_begin as usize <= deformer.weights.len());
        ufbxt_assert!(deformer.weights.len() - vertex.weight_begin as usize >= vertex.num_weights as usize);

        for i in 1..vertex.num_weights as usize {
            let ix = vertex.weight_begin as usize + i;
            if !deformer.weights[ix - 1].weight.is_nan() && !deformer.weights[ix].weight.is_nan() {
                ufbxt_assert!(deformer.weights[ix - 1].weight >= deformer.weights[ix].weight);
            }
        }
    }
    for weight in &deformer.weights {
        ufbxt_assert!((weight.cluster_index as usize) < deformer.clusters.len());
    }
}

fn check_skin_cluster(scene: &Scene, cluster: &SkinCluster) {
    check_element_ptr(scene, cluster.bone_node, ElementType::Node);
}

fn check_blend_deformer(scene: &Scene, deformer: &BlendDeformer) {
    for &channel in &deformer.channels {
        check_element_ptr(scene, Some(channel), ElementType::BlendChannel);
    }
}

fn check_blend_channel(scene: &Scene, channel: &BlendChannel) {
    let keyframes = &channel.keyframes;
    for i in 0..keyframes.len() {
        if i > 0 && !keyframes[i - 1].target_weight.is_nan() && !keyframes[i].target_weight.is_nan() {
            ufbxt_assert!(keyframes[i - 1].target_weight <= keyframes[i].target_weight);
        }
        check_element_ptr(scene, Some(keyframes[i].shape), ElementType::BlendShape);
    }
    check_element_ptr(scene, channel.target_shape, ElementType::BlendShape);

    if let Some(last) = keyframes.last() {
        ufbxt_assert!(channel.target_shape == Some(last.shape));
    } else {
        ufbxt_assert!(channel.target_shape.is_none());
    }
}

fn check_blend_shape(_scene: &Scene, shape: &BlendShape) {
    ufbxt_assert!(shape.offset_vertices.len() == shape.position_offsets.len());
    if !shape.normal_offsets.is_empty() {
        ufbxt_assert!(shape.normal_offsets.len() == shape.offset_vertices.len());
    }
}

fn check_cache_deformer(scene: &Scene, deformer: &CacheDeformer) {
    check_string(&deformer.channel);
    check_element_ptr(scene, deformer.file, ElementType::CacheFile);
}

fn check_cache_file(_scene: &Scene, file: &CacheFile) {
    check_string(&file.filename);
    check_string(&file.absolute_filename);
    check_string(&file.relative_filename);
}

fn check_material_map(scene: &Scene, _material: &Material, map: &MaterialMap) {
    check_element_ptr(scene, map.texture, ElementType::Texture);
    ufbxt_assert!(map.value_components <= 4);
}

fn check_material(scene: &Scene, material: &Material) {
    for texture in &material.textures {
        check_string(&texture.material_prop);
        check_string(&texture.shader_prop);
        check_element_ptr(scene, Some(texture.texture), ElementType::Texture);
    }

    for map in &material.fbx.maps {
        check_material_map(scene, material, map);
    }

    for map in &material.pbr.maps {
        check_material_map(scene, material, map);
    }

    check_string(&material.shader_prop_prefix);
    check_element_ptr(scene, material.shader, ElementType::Shader);
}

fn check_shader_texture_input(scene: &Scene, texture: &Texture, _shader: &ShaderTexture, input: &ShaderTextureInput) {
    check_string(&input.name);

    check_string(&input.value_str);

    ufbxt_assert!(texture.element.props.find(&input.prop).is_some());

    if let Some(prop) = &input.texture_prop {
        ufbxt_assert!(texture.element.props.find(prop).is_some());
    }

    if let Some(prop) = &input.texture_enabled_prop {
        ufbxt_assert!(texture.element.props.find(prop).is_some());
    }

    check_element_ptr(scene, input.texture, ElementType::Texture);
}

fn check_shader_texture(scene: &Scene, texture: &Texture, shader: &ShaderTexture) {
    check_string(&shader.shader_name);
    check_string(&shader.shader_source);

    check_element_ptr(scene, shader.main_texture, ElementType::Texture);

    for input in &shader.inputs {
        check_shader_texture_input(scene, texture, shader, input);
    }
}

fn check_texture(scene: &Scene, index: usize) {
    let texture = &scene.textures[index];
    check_string(&texture.filename);
    check_string(&texture.relative_filename);
    check_string(&texture.absolute_filename);

    check_element_ptr(scene, texture.video, ElementType::Video);
    for layer in &texture.layers {
        check_element_ptr(scene, Some(layer.texture), ElementType::Texture);
    }

    if let Some(shader) = &texture.shader {
        check_shader_texture(scene, texture, shader);
    }

    for &file in &texture.file_textures {
        check_element_ptr(scene, Some(file), ElementType::Texture);
        ufbxt_assert!(scene.textures[file].texture_type == TextureType::File);
    }

    if texture.texture_type == TextureType::File {
        ufbxt_assert!(!texture.file_textures.is_empty());
        ufbxt_assert!(texture.file_textures.first() == Some(&index));
    } else if texture.texture_type == TextureType::Shader {
        ufbxt_assert!(texture.shader.is_some());
    }

    if !texture.raw_absolute_filename.is_empty() || !texture.raw_relative_filename.is_empty() {
        ufbxt_assert!(texture.has_file);
    }

    ufbxt_assert!((texture.file_index as usize) < scene.texture_files.len() || texture.file_index == NO_INDEX);
    if texture.has_file {
        ufbxt_assert!((texture.file_index as usize) < scene.texture_files.len());
        let file = &scene.texture_files[texture.file_index as usize];
        ufbxt_assert!(file.absolute_filename == texture.absolute_filename || file.relative_filename == texture.relative_filename);
    } else {
        ufbxt_assert!(texture.file_index == NO_INDEX);
    }
}

fn check_video(_scene: &Scene, video: &Video) {
    check_string(&video.filename);
    check_string(&video.relative_filename);
    check_string(&video.absolute_filename);
}

fn check_shader(scene: &Scene, shader: &Shader) {
    for &binding in &shader.bindings {
        check_element_ptr(scene, Some(binding), ElementType::ShaderBinding);
    }
}

fn check_shader_binding(_scene: &Scene, binding: &ShaderBinding) {
    for prop in &binding.prop_bindings {
        check_string(&prop.material_prop);
        check_string(&prop.shader_prop);
    }
}

pub fn check_anim(scene: &Scene, anim: &Anim) {
    for &layer in &anim.layers {
        check_element_ptr(scene, Some(layer), ElementType::AnimLayer);
    }
    for over in &anim.prop_overrides {
        check_string(&over.prop_name);
        check_string(&over.value_str);
    }
    if !anim.custom {
        ufbxt_assert!(anim.prop_overrides.is_empty());
        ufbxt_assert!(anim.transform_overrides.is_empty());
        ufbxt_assert!(anim.override_layer_weights.is_empty());
    }
}

fn check_anim_stack(scene: &Scene, anim_stack: &AnimStack) {
    for &layer in &anim_stack.layers {
        check_element_ptr(scene, Some(layer), ElementType::AnimLayer);
    }
}

fn check_anim_layer(scene: &Scene, anim_layer: &AnimLayer) {
    check_string(&anim_layer.element.name);

    for &value in &anim_layer.anim_values {
        check_element_ptr(scene, Some(value), ElementType::AnimValue);
    }

    let mut props_left = 0;
    let mut prev_element: Option<ElementId> = None;

    for anim_prop in &anim_layer.anim_props {
        check_string(&anim_prop.prop_name);
        check_element_ptr_any(scene, Some(anim_prop.element));
        check_element_ptr(scene, Some(anim_prop.anim_value), ElementType::AnimValue);

        let r = find_anim_prop(anim_layer, anim_prop.element, &anim_prop.prop_name);
        ufbxt_assert!(r.is_some());
        ufbxt_assert!(r.is_some_and(|r| r.prop_name == anim_prop.prop_name));

        if props_left > 0 {
            ufbxt_assert!(Some(anim_prop.element) == prev_element);
            props_left -= 1;
        } else {
            let list = find_anim_props(anim_layer, anim_prop.element);
            ufbxt_assert!(!list.is_empty());
            prev_element = Some(anim_prop.element);
            props_left = list.len() - 1;
        }
    }

    ufbxt_assert!(props_left == 0);
}

fn check_anim_value(scene: &Scene, anim_value: &AnimValue) {
    check_element_ptr(scene, anim_value.curves[0], ElementType::AnimCurve);
    check_element_ptr(scene, anim_value.curves[1], ElementType::AnimCurve);
    check_element_ptr(scene, anim_value.curves[2], ElementType::AnimCurve);
}

fn check_anim_curve(_scene: &Scene, _anim_curve: &AnimCurve) {
}

fn check_display_layer(scene: &Scene, layer: &DisplayLayer) {
    for &node in &layer.nodes {
        check_element_ptr(scene, Some(node), ElementType::Node);
    }
}

fn check_selection_set(scene: &Scene, set: &SelectionSet) {
    for &node in &set.nodes {
        check_element_ptr(scene, Some(node), ElementType::SelectionNode);
    }
}

fn check_selection_node(scene: &Scene, node: &SelectionNode) {
    check_element_ptr(scene, node.target_node, ElementType::Node);
    check_element_ptr(scene, node.target_mesh, ElementType::Mesh);
}

fn check_character(_scene: &Scene, _character: &Character) {}

fn check_constraint(scene: &Scene, constraint: &Constraint) {
    check_element_ptr(scene, constraint.node, ElementType::Node);
    check_element_ptr(scene, constraint.aim_up_node, ElementType::Node);
    check_element_ptr(scene, constraint.ik_effector, ElementType::Node);
    check_element_ptr(scene, constraint.ik_end_node, ElementType::Node);
    for target in &constraint.targets {
        check_element_ptr(scene, Some(target.node), ElementType::Node);
    }
}

fn check_audio_layer(scene: &Scene, layer: &AudioLayer) {
    for &clip in &layer.clips {
        check_element_ptr(scene, Some(clip), ElementType::AudioClip);
    }
}

fn check_audio_clip(_scene: &Scene, clip: &AudioClip) {
    check_string(&clip.filename);
    check_string(&clip.absolute_filename);
    check_string(&clip.relative_filename);
}

fn check_pose(scene: &Scene, pose: &Pose) {
    for (i, bone_pose) in pose.bone_poses.iter().enumerate() {
        check_element_ptr(scene, Some(bone_pose.bone_node), ElementType::Node);

        if i > 0 {
            ufbxt_assert!(scene.nodes[pose.bone_poses[i - 1].bone_node].element.typed_id <= scene.nodes[bone_pose.bone_node].element.typed_id);
        }

        let pose_ref = get_bone_pose(pose, bone_pose.bone_node);
        ufbxt_assert!(pose_ref.is_some());
        ufbxt_assert!(pose_ref.is_some_and(|r| r.bone_node == bone_pose.bone_node));
    }
}

fn check_metadata_object(_scene: &Scene, _metadata_object: &MetadataObject) {}

fn check_texture_file(_scene: &Scene, texture_file: &TextureFile) {
    check_string(&texture_file.filename);
    check_string(&texture_file.absolute_filename);
    check_string(&texture_file.relative_filename);
}

fn check_application(_scene: &Scene, application: &Application) {
    check_string(&application.name);
    check_string(&application.vendor);
    check_string(&application.version);
}

fn check_thumbnail(_scene: &Scene, _thumbnail: &Thumbnail) {
}

fn check_metadata(scene: &Scene, metadata: &Metadata) {
    check_string(&metadata.creator);
    check_string(&metadata.filename);
    check_string(&metadata.relative_root);
    check_application(scene, &metadata.latest_application);
    check_application(scene, &metadata.original_application);

    match metadata.file_format {
        FileFormat::Fbx => {}
        FileFormat::Obj => ufbxt_assert!(metadata.ascii),
        FileFormat::Mtl => ufbxt_assert!(metadata.ascii),
        other => {
            ufbxt_hintf!("Invalid file format {other:?}");
            ufbxt_assert!(false);
        }
    }

    for warning in &metadata.warnings {
        check_string(&warning.description);
        ufbxt_assert!(metadata.has_warning[warning.kind as usize]);
        if warning.element_id != NO_INDEX {
            ufbxt_assert!((warning.element_id as usize) < scene.elements.len());
        }
    }

    check_thumbnail(scene, &metadata.thumbnail);
}

fn check_dom_value(_scene: &Scene, value: &DomValue) {
    check_string(&value.value_str);
    let array_stride = match value.value_type {
        DomValueType::Number => 0,
        DomValueType::String => 0,
        DomValueType::Blob => 1,
        DomValueType::ArrayI32 => 4,
        DomValueType::ArrayI64 => 8,
        DomValueType::ArrayF32 => 4,
        DomValueType::ArrayF64 => 8,
        DomValueType::ArrayBlob => 0,
        DomValueType::ArrayIgnored => 0,
    };

    if array_stride != 0 {
        ufbxt_assert!(value.value_blob.len() == value.value_int as usize * array_stride);
    }

    if value.value_type == DomValueType::ArrayBlob {
        ufbxt_assert!(matches!(&value.array, Some(DomArray::Blob(blobs)) if blobs.len() == value.value_int as usize));
    }
}

fn check_dom_node(scene: &Scene, node: &DomNode) {
    check_string(&node.name);
    for child in &node.children {
        check_dom_node(scene, child);
    }
    for value in &node.values {
        check_dom_value(scene, value);
    }
}

pub fn check_scene(scene: &Scene) {
    if scene.metadata.is_unsafe {
        return;
    }

    check_element_cast(scene, None);

    check_metadata(scene, &scene.metadata);

    for (i, r) in scene.elements.iter().enumerate() {
        let element = typed_element(scene, r.element_type, r.index);
        ufbxt_assert!(element.is_some_and(|e| e.element_id as usize == i));
        check_element(scene, element.unwrap());
    }

    for unknown in &scene.unknowns {
        check_unknown(scene, unknown);
    }

    for i in 0..scene.nodes.len() {
        check_node(scene, i);
    }

    for mesh in &scene.meshes {
        check_mesh(scene, mesh);
    }

    for light in &scene.lights {
        check_light(scene, light);
    }

    for camera in &scene.cameras {
        check_camera(scene, camera);
    }

    for bone in &scene.bones {
        check_bone(scene, bone);
    }

    for empty in &scene.empties {
        check_empty(scene, empty);
    }

    for line in &scene.line_curves {
        check_line_curve(scene, line);
    }

    for curve in &scene.nurbs_curves {
        check_nurbs_curve(scene, curve);
    }

    for surface in &scene.nurbs_surfaces {
        check_nurbs_surface(scene, surface);
    }

    for surface in &scene.nurbs_trim_surfaces {
        check_nurbs_trim_surface(scene, surface);
    }

    for boundary in &scene.nurbs_trim_boundaries {
        check_nurbs_trim_boundary(scene, boundary);
    }

    for geometry in &scene.procedural_geometries {
        check_procedural_geometry(scene, geometry);
    }

    for camera in &scene.stereo_cameras {
        check_stereo_camera(scene, camera);
    }

    for switcher in &scene.camera_switchers {
        check_camera_switcher(scene, switcher);
    }

    for marker in &scene.markers {
        check_marker(scene, marker);
    }

    for lod_group in &scene.lod_groups {
        check_lod_group(scene, lod_group);
    }

    for deformer in &scene.skin_deformers {
        check_skin_deformer(scene, deformer);
    }

    for cluster in &scene.skin_clusters {
        check_skin_cluster(scene, cluster);
    }

    for deformer in &scene.blend_deformers {
        check_blend_deformer(scene, deformer);
    }

    for channel in &scene.blend_channels {
        check_blend_channel(scene, channel);
    }

    for shape in &scene.blend_shapes {
        check_blend_shape(scene, shape);
    }

    for deformer in &scene.cache_deformers {
        check_cache_deformer(scene, deformer);
    }

    for file in &scene.cache_files {
        check_cache_file(scene, file);
    }

    for material in &scene.materials {
        check_material(scene, material);
    }

    for i in 0..scene.textures.len() {
        check_texture(scene, i);
    }

    for video in &scene.videos {
        check_video(scene, video);
    }

    for shader in &scene.shaders {
        check_shader(scene, shader);
    }

    for binding in &scene.shader_bindings {
        check_shader_binding(scene, binding);
    }

    for anim_stack in &scene.anim_stacks {
        check_anim_stack(scene, anim_stack);
    }

    for anim_layer in &scene.anim_layers {
        check_anim_layer(scene, anim_layer);
    }

    for anim_value in &scene.anim_values {
        check_anim_value(scene, anim_value);
    }

    for anim_curve in &scene.anim_curves {
        check_anim_curve(scene, anim_curve);
    }

    for layer in &scene.display_layers {
        check_display_layer(scene, layer);
    }

    for set in &scene.selection_sets {
        check_selection_set(scene, set);
    }

    for node in &scene.selection_nodes {
        check_selection_node(scene, node);
    }

    for character in &scene.characters {
        check_character(scene, character);
    }

    for constraint in &scene.constraints {
        check_constraint(scene, constraint);
    }

    for layer in &scene.audio_layers {
        check_audio_layer(scene, layer);
    }

    for clip in &scene.audio_clips {
        check_audio_clip(scene, clip);
    }

    for pose in &scene.poses {
        check_pose(scene, pose);
    }

    for metadata_object in &scene.metadata_objects {
        check_metadata_object(scene, metadata_object);
    }

    for (i, texture_file) in scene.texture_files.iter().enumerate() {
        ufbxt_assert!(texture_file.index as usize == i);
        check_texture_file(scene, texture_file);
    }

    if let Some(dom_root) = &scene.dom_root {
        check_dom_node(scene, dom_root);
    }

    for type_ix in 0..ElementType::COUNT {
        let element_type = ElementType::from_index(type_ix);
        for i in 0..typed_count(scene, element_type) {
            let element = typed_element(scene, element_type, i).unwrap();
            ufbxt_assert!(element.element_type == element_type);
            ufbxt_assert!(element.typed_id as usize == i);
        }
    }

    check_element_ptr(scene, Some(scene.root_node), ElementType::Node);
    ufbxt_assert!(scene.nodes[scene.root_node].is_root);
}

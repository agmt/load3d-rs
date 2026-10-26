use std::collections::HashMap;

use crate::error::{Error, Res};
use crate::math::{EPSILON, add_weighted_mat, add_weighted_quat, add_weighted_vec3, matrix_for_normals, matrix_mul, mul_quat};
use crate::math::{quat_dot, transform_to_matrix};
use crate::props::name_key;
use crate::search::{lower_bound_eq, upper_bound_eq};
use crate::types::*;

fn str_less(a: &str, b: &str) -> bool {
    a.as_bytes() < b.as_bytes()
}

/// Deduplicates the vertices in place, to the front of each stream in order of first use; returns their count
pub fn generate_indices(streams: &mut [VertexStream], indices: &mut [u32]) -> Res<usize> {
    let num_indices = indices.len();

    let mut packed_size = 0usize;
    for (i, stream) in streams.iter().enumerate() {
        let needed = stream.vertex_size.checked_mul(num_indices);
        if stream.vertex_count < num_indices || needed.is_none_or(|n| stream.data.len() < n) {
            return Err(Error::new("Truncated vertex stream").set_err_info(i.to_string()));
        }
        packed_size = packed_size.saturating_add(stream.vertex_size);
    }
    if packed_size == 0 {
        return Err(Error::new("Zero vertex size"));
    }

    let mut map: HashMap<Vec<u8>, u32> = HashMap::with_capacity(num_indices);
    let mut first_use: Vec<usize> = Vec::new();
    let mut packed = Vec::with_capacity(packed_size);
    for (i, index) in indices.iter_mut().enumerate() {
        packed.clear();
        for stream in streams.iter() {
            let size = stream.vertex_size;
            packed.extend_from_slice(&stream.data[i * size..(i + 1) * size]);
        }
        *index = match map.get(&packed) {
            Some(&ix) => ix,
            None => {
                let ix = first_use.len() as u32;
                map.insert(packed.clone(), ix);
                first_use.push(i);
                ix
            }
        };
    }

    // every vertex is first used at or after its new position, so copying forward overwrites nothing still needed
    for stream in streams.iter_mut() {
        let size = stream.vertex_size;
        for (k, &first) in first_use.iter().enumerate() {
            if first != k {
                stream.data.copy_within(first * size..(first + 1) * size, k * size);
            }
        }
    }

    Ok(first_use.len())
}

fn cmp_name_element_less_ref(a: &NameElement, name: &str, element_type: ElementType, key: u32) -> bool {
    if a.internal_key != key {
        return a.internal_key < key;
    }
    match a.name.as_bytes().cmp(name.as_bytes()) {
        std::cmp::Ordering::Equal => a.element_type < element_type,
        cmp => cmp.is_lt(),
    }
}

pub fn find_element(scene: &Scene, element_type: ElementType, name: &str) -> Option<ElementId> {
    let key = name_key(name.as_bytes());
    let list = &scene.elements_by_name;
    let index = lower_bound_eq(
        0,
        list.len(),
        16,
        |i| cmp_name_element_less_ref(&list[i], name, element_type, key),
        |i| list[i].name == name && list[i].element_type == element_type,
    )?;
    Some(list[index].element)
}

fn find_typed(scene: &Scene, element_type: ElementType, name: &str) -> Option<usize> {
    let id = find_element(scene, element_type, name)?;
    scene.elements.get(id as usize).map(|r| r.index)
}

/// An index of `Scene::nodes`
pub fn find_node(scene: &Scene, name: &str) -> Option<usize> {
    find_typed(scene, ElementType::Node, name)
}

/// An index of `Scene::anim_stacks`
pub fn find_anim_stack(scene: &Scene, name: &str) -> Option<usize> {
    find_typed(scene, ElementType::AnimStack, name)
}

/// An index of `Scene::materials`
pub fn find_material(scene: &Scene, name: &str) -> Option<usize> {
    find_typed(scene, ElementType::Material, name)
}

pub fn get_prop_element(scene: &Scene, element: &Element, prop: &Prop, element_type: ElementType) -> Option<ElementId> {
    crate::finalize::fetch_dst_element(scene, element.element_id, false, &prop.name, element_type)
}

pub fn find_prop_element(scene: &Scene, element: &Element, name: &str, element_type: ElementType) -> Option<ElementId> {
    let prop = element.props.find(name)?;
    get_prop_element(scene, element, prop, element_type)
}

pub fn find_anim_prop<'a>(layer: &'a AnimLayer, element: ElementId, prop: &str) -> Option<&'a AnimProp> {
    let props = &layer.anim_props;
    let index = lower_bound_eq(
        0,
        props.len(),
        16,
        |i| if props[i].element != element { props[i].element < element } else { str_less(&props[i].prop_name, prop) },
        |i| props[i].element == element && props[i].prop_name == prop,
    )?;
    Some(&props[index])
}

pub fn find_anim_props(layer: &AnimLayer, element: ElementId) -> &[AnimProp] {
    let props = &layer.anim_props;
    let count = props.len();
    let begin = lower_bound_eq(0, count, 16, |i| props[i].element < element, |i| props[i].element == element).unwrap_or(count);
    let end = upper_bound_eq(begin, count, 16, |i| props[i].element == element);
    &props[begin..end]
}

pub fn get_compatible_matrix_for_normals(node: &Node) -> Matrix {
    let geom_rot = Transform { rotation: node.geometry_transform.rotation, ..Transform::IDENTITY };
    let geom_rot_mat = transform_to_matrix(&geom_rot);

    let norm_mat = matrix_mul(&node.node_to_world, &geom_rot_mat);
    matrix_for_normals(&norm_mat)
}

/// `node` is an index of `Scene::nodes`
pub fn get_bone_pose(pose: &Pose, node: usize) -> Option<&BonePose> {
    let poses = &pose.bone_poses;
    let index = lower_bound_eq(0, poses.len(), 8, |i| poses[i].bone_node < node, |i| poses[i].bone_node == node)?;
    Some(&poses[index])
}

/// An index of `Scene::textures`
pub fn find_prop_texture(material: &Material, name: &str) -> Option<usize> {
    let textures = &material.textures;
    let index = lower_bound_eq(0, textures.len(), 4, |i| str_less(&textures[i].material_prop, name), |i| textures[i].material_prop == name)?;
    Some(textures[index].texture)
}

pub fn find_shader_prop<'a>(scene: &'a Scene, shader: &Shader, name: &str) -> &'a str {
    match find_shader_prop_bindings(scene, shader, name).first() {
        Some(binding) => &binding.material_prop,
        None => "",
    }
}

/// Only the first of the shader's bindings that names the property
pub fn find_shader_prop_bindings<'a>(scene: &'a Scene, shader: &Shader, name: &str) -> &'a [ShaderPropBinding] {
    for &bind_ix in &shader.bindings {
        let Some(bind) = scene.shader_bindings.get(bind_ix) else { continue };
        let props = &bind.prop_bindings;
        let count = props.len();
        if let Some(begin) = lower_bound_eq(0, count, 4, |i| str_less(&props[i].shader_prop, name), |i| props[i].shader_prop == name) {
            let end = upper_bound_eq(begin, count, 4, |i| props[i].shader_prop == name);
            return &props[begin..end];
        }
    }
    &[]
}

pub fn find_shader_texture_input<'a>(shader: &'a ShaderTexture, name: &str) -> Option<&'a ShaderTextureInput> {
    let inputs = &shader.inputs;
    let index = lower_bound_eq(0, inputs.len(), 4, |i| str_less(&inputs[i].name, name), |i| inputs[i].name == name)?;
    Some(&inputs[index])
}

/// The identity for a vertex out of range, as ufbx's release build gives it
pub fn get_skin_vertex_matrix(scene: &Scene, skin: &SkinDeformer, vertex: usize, fallback: Option<&Matrix>) -> Matrix {
    let Some(&skin_vertex) = skin.vertices.get(vertex) else { return Matrix::IDENTITY };

    let mut mat = Matrix::ZERO;
    let zero_quat = Quat::new(0.0, 0.0, 0.0, 0.0);
    let (mut q0, mut qe, mut first_q0) = (zero_quat, zero_quat, zero_quat);
    let mut qs = Vec3::ZERO;
    let mut total_weight: Real = 0.0;

    for i in 0..skin_vertex.num_weights {
        let weight = skin.weights[skin_vertex.weight_begin as usize + i as usize];
        let cluster = &scene.skin_clusters[skin.clusters[weight.cluster_index as usize]];
        if cluster.bone_node.is_none() {
            continue;
        }

        total_weight += weight.weight;
        if skin_vertex.dq_weight > 0.0 {
            let t = cluster.geometry_to_world_transform;
            let mut vq0 = t.rotation;
            if i == 0 {
                first_q0 = vq0;
            }

            if quat_dot(first_q0, vq0) < 0.0 {
                vq0.x = -vq0.x;
                vq0.y = -vq0.y;
                vq0.z = -vq0.z;
                vq0.w = -vq0.w;
            }

            let vqt = Quat::new(0.5 * t.translation.x, 0.5 * t.translation.y, 0.5 * t.translation.z, 0.0);
            let vqe = mul_quat(vqt, vq0);
            add_weighted_quat(&mut q0, vq0, weight.weight);
            add_weighted_quat(&mut qe, vqe, weight.weight);
            add_weighted_vec3(&mut qs, t.scale, weight.weight);
        }

        if skin_vertex.dq_weight < 1.0 {
            add_weighted_mat(&mut mat, &cluster.geometry_to_world, (1.0 - skin_vertex.dq_weight) * weight.weight);
        }
    }

    if total_weight <= 0.0 {
        return fallback.copied().unwrap_or(Matrix::IDENTITY);
    }

    if (total_weight - 1.0).abs() > EPSILON {
        let rcp_weight = if total_weight.abs() > EPSILON { 1.0 / total_weight } else { 0.0 };
        if skin_vertex.dq_weight > 0.0 {
            for q in [&mut q0, &mut qe] {
                q.x *= rcp_weight;
                q.y *= rcp_weight;
                q.z *= rcp_weight;
                q.w *= rcp_weight;
            }
            qs.x *= rcp_weight;
            qs.y *= rcp_weight;
            qs.z *= rcp_weight;
        }
        if skin_vertex.dq_weight < 1.0 {
            mat = Matrix::from_v(mat.v().map(|m| m * rcp_weight));
        }
    }

    if skin_vertex.dq_weight > 0.0 {
        let rcp_len = 1.0 / (q0.x * q0.x + q0.y * q0.y + q0.z * q0.z + q0.w * q0.w).sqrt();
        let rcp_len2x2 = 2.0 * rcp_len * rcp_len;
        let dqt = Transform {
            rotation: Quat::new(q0.x * rcp_len, q0.y * rcp_len, q0.z * rcp_len, q0.w * rcp_len),
            scale: qs,
            translation: Vec3::new(
                rcp_len2x2 * (-qe.w * q0.x + qe.x * q0.w - qe.y * q0.z + qe.z * q0.y),
                rcp_len2x2 * (-qe.w * q0.y + qe.x * q0.z + qe.y * q0.w - qe.z * q0.x),
                rcp_len2x2 * (-qe.w * q0.z - qe.x * q0.y + qe.y * q0.x + qe.z * q0.w),
            ),
        };
        let dqm = transform_to_matrix(&dqt);
        if skin_vertex.dq_weight < 1.0 {
            add_weighted_mat(&mut mat, &dqm, skin_vertex.dq_weight);
        } else {
            mat = dqm;
        }
    }

    mat
}

/// `NO_INDEX` where the shape does not move the vertex
pub fn get_blend_shape_offset_index(shape: &BlendShape, vertex: usize) -> u32 {
    let vertex_ix = vertex as u32;
    let offsets = &shape.offset_vertices;
    let count = shape.num_offsets.min(offsets.len());
    match lower_bound_eq(0, count, 16, |i| offsets[i] < vertex_ix, |i| offsets[i] == vertex_ix) {
        Some(index) if index < u32::MAX as usize => index as u32,
        _ => NO_INDEX,
    }
}

pub fn get_blend_shape_vertex_offset(shape: &BlendShape, vertex: usize) -> Vec3 {
    let index = get_blend_shape_offset_index(shape, vertex);
    if index == NO_INDEX {
        return Vec3::ZERO;
    }
    shape.position_offsets.get(index as usize).copied().unwrap_or(Vec3::ZERO)
}

fn blend_keyframes<'a>(scene: &'a Scene, blend: &'a BlendDeformer) -> impl Iterator<Item = &'a BlendKeyframe> {
    blend.channels.iter().filter_map(|&c| scene.blend_channels.get(c)).flat_map(|chan| chan.keyframes.iter())
}

pub fn get_blend_vertex_offset(scene: &Scene, blend: &BlendDeformer, vertex: usize) -> Vec3 {
    let mut offset = Vec3::ZERO;
    for key in blend_keyframes(scene, blend) {
        if key.effective_weight == 0.0 {
            continue;
        }
        let key_offset = get_blend_shape_vertex_offset(&scene.blend_shapes[key.shape], vertex);
        add_weighted_vec3(&mut offset, key_offset, key.effective_weight);
    }
    offset
}

/// Offsets of vertices past the end of `vertices` are left out
pub fn add_blend_shape_vertex_offsets(shape: &BlendShape, vertices: &mut [Vec3], weight: Real) {
    if weight == 0.0 {
        return;
    }

    let weights = &shape.offset_weights;
    for i in 0..shape.num_offsets {
        let (Some(&index), Some(&offset)) = (shape.offset_vertices.get(i), shape.position_offsets.get(i)) else { break };
        if let Some(vertex) = vertices.get_mut(index as usize) {
            let mut vertex_weight = weight;
            if i < weights.len() {
                vertex_weight *= weights[i];
            }
            add_weighted_vec3(vertex, offset, vertex_weight);
        }
    }
}

pub fn add_blend_vertex_offsets(scene: &Scene, blend: &BlendDeformer, vertices: &mut [Vec3], weight: Real) {
    for key in blend_keyframes(scene, blend) {
        if key.effective_weight == 0.0 {
            continue;
        }
        add_blend_shape_vertex_offsets(&scene.blend_shapes[key.shape], vertices, weight * key.effective_weight);
    }
}

impl<T: Copy + Default> VertexAttrib<T> {
    /// `ufbx_catch_get_vertex_real`, `ufbx_catch_get_vertex_vec2`, `ufbx_catch_get_vertex_vec3` and `ufbx_catch_get_vertex_vec4` in ufbx.c:
    /// An error where ufbx would panic
    pub fn catch_get(&self, index: usize) -> Res<T> {
        let ix = check_vertex_index(&self.indices, self.values.len(), index)?;
        Ok(if ix == NO_INDEX { T::default() } else { self.values[ix as usize] })
    }
}

impl VertexVec3 {
    /// `ufbx_catch_get_vertex_w_vec3` in ufbx.c
    pub fn catch_get_w(&self, index: usize) -> Res<Real> {
        if index >= self.indices.len() {
            return Err(Error::new(format!("index ({}) out of range ({})", index, self.indices.len())));
        }
        if self.values_w.is_empty() {
            return Ok(0.0);
        }
        let ix = check_vertex_index(&self.indices, self.values.len(), index)?;
        Ok(if ix == NO_INDEX { 0.0 } else { self.values_w.get(ix as usize).copied().unwrap_or(0.0) })
    }
}

fn check_vertex_index(indices: &[u32], num_values: usize, index: usize) -> Res<u32> {
    let Some(&ix) = indices.get(index) else {
        return Err(Error::new(format!("index ({}) out of range ({})", index, indices.len())));
    };
    if (ix as usize) < num_values || ix == NO_INDEX { Ok(ix) } else { Err(Error::new(format!("Corrupted or missing vertex attribute ({}) at {}", ix, index))) }
}

macro_rules! impl_as_element {
    ($($f:ident => $variant:ident, $ty:ident, $list:ident;)*) => {
        impl Scene {
            $(
                pub fn $f(&self, id: ElementId) -> Option<&$ty> {
                    let r = self.elements.get(id as usize)?;
                    if r.element_type == ElementType::$variant { self.$list.get(r.index) } else { None }
                }
            )*
        }
    };
}

impl_as_element! {
    as_unknown => Unknown, Unknown, unknowns;
    as_node => Node, Node, nodes;
    as_mesh => Mesh, Mesh, meshes;
    as_light => Light, Light, lights;
    as_camera => Camera, Camera, cameras;
    as_bone => Bone, Bone, bones;
    as_empty => Empty, Empty, empties;
    as_line_curve => LineCurve, LineCurve, line_curves;
    as_nurbs_curve => NurbsCurve, NurbsCurve, nurbs_curves;
    as_nurbs_surface => NurbsSurface, NurbsSurface, nurbs_surfaces;
    as_nurbs_trim_surface => NurbsTrimSurface, NurbsTrimSurface, nurbs_trim_surfaces;
    as_nurbs_trim_boundary => NurbsTrimBoundary, NurbsTrimBoundary, nurbs_trim_boundaries;
    as_procedural_geometry => ProceduralGeometry, ProceduralGeometry, procedural_geometries;
    as_stereo_camera => StereoCamera, StereoCamera, stereo_cameras;
    as_camera_switcher => CameraSwitcher, CameraSwitcher, camera_switchers;
    as_marker => Marker, Marker, markers;
    as_lod_group => LodGroup, LodGroup, lod_groups;
    as_skin_deformer => SkinDeformer, SkinDeformer, skin_deformers;
    as_skin_cluster => SkinCluster, SkinCluster, skin_clusters;
    as_blend_deformer => BlendDeformer, BlendDeformer, blend_deformers;
    as_blend_channel => BlendChannel, BlendChannel, blend_channels;
    as_blend_shape => BlendShape, BlendShape, blend_shapes;
    as_cache_deformer => CacheDeformer, CacheDeformer, cache_deformers;
    as_cache_file => CacheFile, CacheFile, cache_files;
    as_material => Material, Material, materials;
    as_texture => Texture, Texture, textures;
    as_video => Video, Video, videos;
    as_shader => Shader, Shader, shaders;
    as_shader_binding => ShaderBinding, ShaderBinding, shader_bindings;
    as_anim_stack => AnimStack, AnimStack, anim_stacks;
    as_anim_layer => AnimLayer, AnimLayer, anim_layers;
    as_anim_value => AnimValue, AnimValue, anim_values;
    as_anim_curve => AnimCurve, AnimCurve, anim_curves;
    as_display_layer => DisplayLayer, DisplayLayer, display_layers;
    as_selection_set => SelectionSet, SelectionSet, selection_sets;
    as_selection_node => SelectionNode, SelectionNode, selection_nodes;
    as_character => Character, Character, characters;
    as_constraint => Constraint, Constraint, constraints;
    as_audio_layer => AudioLayer, AudioLayer, audio_layers;
    as_audio_clip => AudioClip, AudioClip, audio_clips;
    as_pose => Pose, Pose, poses;
    as_metadata_object => MetadataObject, MetadataObject, metadata_objects;
}

impl Scene {
    /// `path` is an element's `dom_node`, its place under `dom_root`
    pub fn dom_node(&self, path: &[u32]) -> Option<&DomNode> {
        path.iter().try_fold(self.dom_root.as_ref()?, |node, &i| node.children.get(i as usize))
    }
}

impl DomNode {
    /// `ufbx_dom_find` in ufbx.c
    pub fn find(&self, name: &str) -> Option<&DomNode> {
        self.children.iter().find(|c| c.name == name)
    }

    /// `ufbx_dom_is_array` in ufbx.c
    pub fn is_array(&self) -> bool {
        match &self.values[..] {
            [v] => matches!(
                v.value_type,
                DomValueType::ArrayI32 | DomValueType::ArrayI64 | DomValueType::ArrayF32 | DomValueType::ArrayF64 | DomValueType::ArrayBlob
            ),
            _ => false,
        }
    }

    /// `ufbx_dom_array_size` in ufbx.c
    pub fn array_size(&self) -> usize {
        if self.is_array() { self.values[0].value_int as usize } else { 0 }
    }

    fn single_array(&self, value_type: DomValueType) -> Option<&DomArray> {
        match &self.values[..] {
            [v] if v.value_type == value_type => v.array.as_ref(),
            _ => None,
        }
    }

    /// `ufbx_dom_as_int32_list` in ufbx.c
    pub fn as_int32_list(&self) -> &[i32] {
        match self.single_array(DomValueType::ArrayI32) {
            Some(DomArray::I32(a)) => a,
            _ => &[],
        }
    }

    /// `ufbx_dom_as_int64_list` in ufbx.c
    pub fn as_int64_list(&self) -> &[i64] {
        match self.single_array(DomValueType::ArrayI64) {
            Some(DomArray::I64(a)) => a,
            _ => &[],
        }
    }

    /// `ufbx_dom_as_float_list` in ufbx.c
    pub fn as_float_list(&self) -> &[f32] {
        match self.single_array(DomValueType::ArrayF32) {
            Some(DomArray::F32(a)) => a,
            _ => &[],
        }
    }

    /// `ufbx_dom_as_double_list` in ufbx.c
    pub fn as_double_list(&self) -> &[f64] {
        match self.single_array(DomValueType::ArrayF64) {
            Some(DomArray::F64(a)) => a,
            _ => &[],
        }
    }

    /// `ufbx_dom_as_real_list` in ufbx.c
    pub fn as_real_list(&self) -> &[Real] {
        self.as_double_list()
    }

    /// `ufbx_dom_as_blob_list` in ufbx.c
    pub fn as_blob_list(&self) -> &[Vec<u8>] {
        match self.single_array(DomValueType::ArrayBlob) {
            Some(DomArray::Blob(a)) => a,
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorType;
    use crate::math::{matrix_invert, transform_position};

    fn named(name: &str, element_id: ElementId, typed_id: usize, element_type: ElementType) -> Element {
        Element { name: name.into(), element_id, typed_id: typed_id as u32, element_type, ..Default::default() }
    }

    fn scene_with_names(entries: &[(&str, ElementType)]) -> Scene {
        let mut scene = Scene::default();
        for (id, &(name, t)) in entries.iter().enumerate() {
            let index = scene.count_of(t);
            let element = named(name, id as ElementId, index, t);
            match t {
                ElementType::Node => scene.nodes.push(Node { element, ..Default::default() }),
                ElementType::Material => scene.materials.push(Material { element, ..Default::default() }),
                ElementType::AnimStack => scene.anim_stacks.push(AnimStack { element, ..Default::default() }),
                ElementType::Mesh => scene.meshes.push(Mesh { element, ..Default::default() }),
                _ => unreachable!(),
            }
            scene.elements.push(ElementRef { element_type: t, index });
            scene.elements_by_name.push(NameElement { name: name.into(), element_type: t, internal_key: name_key(name.as_bytes()), element: id as ElementId });
        }
        scene.elements_by_name.sort_by(|a, b| (a.internal_key, a.name.as_bytes(), a.element_type).cmp(&(b.internal_key, b.name.as_bytes(), b.element_type)));
        scene
    }

    #[test]
    fn bounds_match_a_plain_search() {
        let mut seed = 12345u32;
        for len in 0..80 {
            let mut data: Vec<u32> = (0..len)
                .map(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    seed >> 28
                })
                .collect();
            data.sort();
            for linear in [2, 4, 8, 16, 32] {
                for x in 0..17 {
                    let lo = lower_bound_eq(0, len, linear, |i| data[i] < x, |i| data[i] == x);
                    assert_eq!(lo, data.iter().position(|&d| d == x), "{data:?} {x}");
                    if let Some(lo) = lo {
                        let hi = upper_bound_eq(lo, len, linear, |i| data[i] == x);
                        assert_eq!(hi, lo + data.iter().filter(|&&d| d == x).count());
                    }
                }
            }
        }
    }

    #[test]
    fn generate_indices_dedups_across_streams() {
        let mut positions: Vec<u8> =
            [[0.0f64, 1.0], [2.0, 3.0], [0.0, 1.0], [2.0, 3.0], [0.0, 1.0]].iter().flat_map(|v| v.iter().flat_map(|x| x.to_le_bytes())).collect();
        let mut tags: Vec<u8> = vec![7, 7, 7, 9, 7];
        let mut indices = [0u32; 5];
        let mut streams =
            [VertexStream { data: &mut positions, vertex_count: 5, vertex_size: 16 }, VertexStream { data: &mut tags, vertex_count: 5, vertex_size: 1 }];
        assert_eq!(generate_indices(&mut streams, &mut indices).unwrap(), 3);
        assert_eq!(indices, [0, 1, 0, 2, 0]);
        assert_eq!(&tags[..3], [7, 7, 9]);
        assert_eq!(&positions[32..48], &positions[16..32]);
        assert_eq!(f64::from_le_bytes(positions[16..24].try_into().unwrap()), 2.0);

        let mut short = vec![0u8; 4];
        let mut streams = [VertexStream { data: &mut short, vertex_count: 2, vertex_size: 2 }];
        let err = generate_indices(&mut streams, &mut [0u32; 3]).unwrap_err();
        assert_eq!((err.kind, err.info.as_str()), (ErrorType::TruncatedVertexStream, "0"));

        let mut empty: [u8; 0] = [];
        let mut streams = [VertexStream { data: &mut empty, vertex_count: 3, vertex_size: 0 }];
        assert_eq!(generate_indices(&mut streams, &mut [0u32; 3]).unwrap_err().kind, ErrorType::ZeroVertexSize);
        assert_eq!(generate_indices(&mut [], &mut []).unwrap_err().kind, ErrorType::ZeroVertexSize);
    }

    #[test]
    fn find_by_name_and_type() {
        let scene = scene_with_names(&[
            ("Root", ElementType::Node),
            ("Cube", ElementType::Node),
            ("Cube", ElementType::Mesh),
            ("Cube", ElementType::Material),
            ("Take 001", ElementType::AnimStack),
            ("Ab", ElementType::Node),
            ("Abc", ElementType::Node),
        ]);
        assert_eq!(find_element(&scene, ElementType::Mesh, "Cube"), Some(2));
        assert_eq!(find_element(&scene, ElementType::Node, "Ab"), Some(5));
        assert_eq!(find_element(&scene, ElementType::Node, "Abc"), Some(6));
        assert_eq!(find_element(&scene, ElementType::Node, "Abcd"), None);
        assert_eq!(find_element(&scene, ElementType::Light, "Cube"), None);
        assert_eq!(find_node(&scene, "Cube"), Some(1));
        assert_eq!(find_material(&scene, "Cube"), Some(0));
        assert_eq!(find_anim_stack(&scene, "Take 001"), Some(0));
        assert_eq!(find_anim_stack(&scene, "Take 002"), None);
        assert!(scene.as_node(1).is_some() && scene.as_mesh(1).is_none() && scene.as_mesh(2).is_some());
        assert!(scene.as_node(99).is_none());
    }

    #[test]
    fn anim_props_by_element_then_name() {
        let prop = |element: ElementId, name: &str, anim_value: usize| AnimProp {
            element,
            internal_key: name_key(name.as_bytes()),
            prop_name: name.into(),
            anim_value,
        };
        let layer = AnimLayer {
            anim_props: vec![
                prop(1, "Lcl Rotation", 0),
                prop(1, "Lcl Translation", 1),
                prop(3, "DeformPercent", 2),
                prop(3, "Lcl Scaling", 3),
                prop(3, "Visibility", 4),
                prop(7, "Intensity", 5),
            ],
            ..Default::default()
        };
        assert_eq!(find_anim_props(&layer, 3).iter().map(|p| p.anim_value).collect::<Vec<_>>(), [2, 3, 4]);
        assert_eq!(find_anim_props(&layer, 7).len(), 1);
        assert!(find_anim_props(&layer, 2).is_empty() && find_anim_props(&layer, 9).is_empty());
        assert_eq!(find_anim_prop(&layer, 1, "Lcl Translation").map(|p| p.anim_value), Some(1));
        assert_eq!(find_anim_prop(&layer, 3, "Visibility").map(|p| p.anim_value), Some(4));
        assert!(find_anim_prop(&layer, 3, "Lcl Rotation").is_none());
    }

    #[test]
    fn bone_poses_textures_and_shader_bindings() {
        let pose = Pose { bone_poses: [2, 5, 9].iter().map(|&n| BonePose { bone_node: n, ..Default::default() }).collect(), ..Default::default() };
        assert_eq!(get_bone_pose(&pose, 5).map(|p| p.bone_node), Some(5));
        assert!(get_bone_pose(&pose, 4).is_none());

        let tex = |prop: &str, texture: usize| MaterialTexture { material_prop: prop.into(), shader_prop: String::new(), texture };
        let material = Material { textures: vec![tex("DiffuseColor", 4), tex("NormalMap", 1)], ..Default::default() };
        assert_eq!(find_prop_texture(&material, "NormalMap"), Some(1));
        assert_eq!(find_prop_texture(&material, "Bump"), None);

        let bind = |s: &str, m: &str| ShaderPropBinding { shader_prop: s.into(), material_prop: m.into() };
        let mut scene = Scene::default();
        scene.shader_bindings.push(ShaderBinding { prop_bindings: vec![bind("base", "Maya|base")], ..Default::default() });
        scene.shader_bindings.push(ShaderBinding {
            prop_bindings: vec![bind("color", "Maya|baseColor"), bind("color", "Maya|tint"), bind("rough", "Maya|r")],
            ..Default::default()
        });
        let shader = Shader { bindings: vec![0, 1], ..Default::default() };
        assert_eq!(find_shader_prop_bindings(&scene, &shader, "color").len(), 2);
        assert_eq!(find_shader_prop(&scene, &shader, "color"), "Maya|baseColor");
        assert_eq!(find_shader_prop(&scene, &shader, "base"), "Maya|base");
        assert_eq!(find_shader_prop(&scene, &shader, "metal"), "");

        let input = |name: &str| ShaderTextureInput { name: name.into(), ..Default::default() };
        let texture = ShaderTexture { inputs: vec![input("color"), input("image"), input("uv")], ..Default::default() };
        assert_eq!(find_shader_texture_input(&texture, "image").map(|i| i.name.as_str()), Some("image"));
        assert!(find_shader_texture_input(&texture, "alpha").is_none());
    }

    fn skin_scene() -> (Scene, SkinDeformer) {
        let mut scene = Scene::default();
        let cluster = |t: Transform, bone: Option<usize>| SkinCluster {
            bone_node: bone,
            geometry_to_world: transform_to_matrix(&t),
            geometry_to_world_transform: t,
            ..Default::default()
        };
        let moved = |x: Real| Transform { translation: Vec3::new(x, 0.0, 0.0), ..Transform::IDENTITY };
        scene.skin_clusters = vec![cluster(moved(2.0), Some(0)), cluster(moved(4.0), Some(1)), cluster(moved(100.0), None)];
        let weight = |cluster_index: u32, weight: Real| SkinWeight { cluster_index, weight };
        let skin = SkinDeformer {
            clusters: vec![0, 1, 2],
            vertices: vec![
                SkinVertex { weight_begin: 0, num_weights: 2, dq_weight: 0.0 },
                SkinVertex { weight_begin: 2, num_weights: 1, dq_weight: 0.0 },
                SkinVertex { weight_begin: 3, num_weights: 2, dq_weight: 1.0 },
                SkinVertex { weight_begin: 5, num_weights: 1, dq_weight: 0.0 },
            ],
            weights: vec![weight(0, 0.5), weight(1, 0.5), weight(0, 0.25), weight(0, 1.0), weight(1, 1.0), weight(2, 1.0)],
            ..Default::default()
        };
        (scene, skin)
    }

    #[test]
    fn skin_vertex_matrices() {
        let (scene, skin) = skin_scene();
        let at = |m: Matrix| transform_position(&m, Vec3::ZERO);
        assert_eq!(at(get_skin_vertex_matrix(&scene, &skin, 0, None)), Vec3::new(3.0, 0.0, 0.0));
        assert_eq!(at(get_skin_vertex_matrix(&scene, &skin, 1, None)), Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(at(get_skin_vertex_matrix(&scene, &skin, 2, None)), Vec3::new(3.0, 0.0, 0.0));
        let fallback = transform_to_matrix(&Transform { scale: Vec3::new(2.0, 2.0, 2.0), ..Transform::IDENTITY });
        assert_eq!(get_skin_vertex_matrix(&scene, &skin, 3, Some(&fallback)), fallback);
        assert_eq!(get_skin_vertex_matrix(&scene, &skin, 3, None), Matrix::IDENTITY);
        assert_eq!(get_skin_vertex_matrix(&scene, &skin, 4, Some(&fallback)), Matrix::IDENTITY);
    }

    #[test]
    fn blend_offsets() {
        let mut scene = Scene::default();
        scene.blend_shapes.push(BlendShape {
            num_offsets: 2,
            offset_vertices: vec![1, 3],
            position_offsets: vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0)],
            offset_weights: vec![1.0, 0.5],
            ..Default::default()
        });
        scene.blend_shapes.push(BlendShape {
            num_offsets: 1,
            offset_vertices: vec![9],
            position_offsets: vec![Vec3::new(0.0, 0.0, 1.0)],
            ..Default::default()
        });
        let key = |shape: usize, effective_weight: Real| BlendKeyframe { shape, target_weight: 1.0, effective_weight };
        scene.blend_channels.push(BlendChannel { keyframes: vec![key(0, 0.5), key(1, 0.0)], ..Default::default() });
        scene.blend_channels.push(BlendChannel { keyframes: vec![key(1, 1.0)], ..Default::default() });
        let blend = BlendDeformer { channels: vec![0, 1], ..Default::default() };

        let shape = &scene.blend_shapes[0];
        assert_eq!(get_blend_shape_offset_index(shape, 3), 1);
        assert_eq!(get_blend_shape_offset_index(shape, 2), NO_INDEX);
        assert_eq!(get_blend_shape_vertex_offset(shape, 3), Vec3::new(0.0, 2.0, 0.0));
        assert_eq!(get_blend_vertex_offset(&scene, &blend, 1), Vec3::new(0.5, 0.0, 0.0));
        assert_eq!(get_blend_vertex_offset(&scene, &blend, 9), Vec3::new(0.0, 0.0, 1.0));

        let mut vertices = vec![Vec3::ZERO; 4];
        add_blend_vertex_offsets(&scene, &blend, &mut vertices, 2.0);
        assert_eq!(vertices, [Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)]);
        add_blend_shape_vertex_offsets(shape, &mut vertices, 0.0);
        assert_eq!(vertices[1], Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn vertex_attribute_access() {
        let attrib = VertexVec3 {
            exists: true,
            values: vec![Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)],
            indices: vec![1, NO_INDEX, 0, 5],
            values_w: vec![0.25, 0.75],
            ..Default::default()
        };
        assert_eq!(attrib.catch_get(0).unwrap(), Vec3::new(4.0, 5.0, 6.0));
        assert_eq!(attrib.catch_get(1).unwrap(), Vec3::ZERO);
        assert_eq!(attrib.catch_get(3).unwrap_err().description, "Corrupted or missing vertex attribute (5) at 3");
        assert_eq!(attrib.catch_get(4).unwrap_err().description, "index (4) out of range (4)");
        assert_eq!(attrib.catch_get_w(0).unwrap(), 0.75);
        assert_eq!(VertexVec3 { values_w: Vec::new(), ..attrib.clone() }.catch_get_w(3).unwrap(), 0.0);
        assert!(attrib.catch_get_w(3).is_err());
    }

    #[test]
    fn dom_arrays() {
        let array = |value_type: DomValueType, array: DomArray, n: i64| DomValue { value_type, value_int: n, array: Some(array), ..Default::default() };
        let node = DomNode {
            name: "Geometry".into(),
            children: vec![
                DomNode { name: "Vertices".into(), values: vec![array(DomValueType::ArrayF64, DomArray::F64(vec![1.0, 2.0]), 2)], ..Default::default() },
                DomNode { name: "PolygonVertexIndex".into(), values: vec![array(DomValueType::ArrayI32, DomArray::I32(vec![0, -2]), 2)], ..Default::default() },
                DomNode { name: "Version".into(), values: vec![DomValue { value_int: 124, ..Default::default() }], ..Default::default() },
            ],
            ..Default::default()
        };
        let vertices = node.find("Vertices").unwrap();
        assert!(vertices.is_array());
        assert_eq!(vertices.array_size(), 2);
        assert_eq!(vertices.as_real_list(), [1.0, 2.0]);
        assert!(vertices.as_int32_list().is_empty() && vertices.as_float_list().is_empty());
        assert_eq!(node.find("PolygonVertexIndex").unwrap().as_int32_list(), [0, -2]);
        let version = node.find("Version").unwrap();
        assert!(!version.is_array() && version.array_size() == 0);
        assert!(node.find("Edges").is_none());
    }

    #[cfg(feature = "format-fbx")]
    #[test]
    fn an_elements_dom_node_is_found_by_its_place() {
        let text =
            b"; FBX 7.4.0 project file\nObjects:  {\n\tModel: 1, \"Model::hull\", \"Null\" {\n\t\tVersion: 232\n\t}\n}\nConnections:  {\n\tC: \"OO\",1,0\n}\n";
        let scene = crate::load::load_memory(text, &LoadOpts { retain_dom: true, ..Default::default() }).unwrap();
        let hull = &scene.nodes[find_node(&scene, "hull").unwrap()];
        let dom = scene.dom_node(hull.element.dom_node.as_deref().unwrap()).unwrap();
        assert_eq!(dom.name, "Model");
        assert_eq!(dom.find("Version").unwrap().values[0].value_int, 232);
        assert!(scene.dom_node(&[99]).is_none());
        let unkept = crate::load::load_memory(text, &LoadOpts::default()).unwrap();
        assert!(unkept.dom_node(&[]).is_none());
    }

    #[test]
    fn normal_matrix_takes_only_the_geometry_rotation() {
        let rotation = crate::math::euler_to_quat(Vec3::new(0.0, 90.0, 0.0), RotationOrder::Xyz);
        let node = Node {
            geometry_transform: Transform { translation: Vec3::new(5.0, 0.0, 0.0), rotation, scale: Vec3::new(3.0, 3.0, 3.0) },
            node_to_world: transform_to_matrix(&Transform { scale: Vec3::new(2.0, 1.0, 1.0), ..Transform::IDENTITY }),
            ..Default::default()
        };
        let geom_rot = transform_to_matrix(&Transform { rotation, ..Transform::IDENTITY });
        let expected = matrix_for_normals(&matrix_mul(&node.node_to_world, &geom_rot));
        assert_eq!(get_compatible_matrix_for_normals(&node), expected);
        let inv = matrix_invert(&expected);
        assert!(inv.m03 == 0.0 && inv.m13 == 0.0 && inv.m23 == 0.0);
    }
}

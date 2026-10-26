use crate::error::{Res, check};
use crate::math::*;
use crate::mesh::{finalize_mesh, finalize_mesh_material};
use crate::read::{patch_mesh_reals, update_face_groups};
use crate::search::unstable_sort;
use crate::topology::{compute_normals, compute_topology, generate_normal_mapping, topo_next_vertex_edge, topo_prev_vertex_edge};
use crate::types::*;

type VertexWeights = Vec<SubdivisionWeight>;

struct LayerInput<'a, T> {
    values: &'a [T],
    indices: &'a [u32],
    same_data: Option<fn(&T, &T) -> bool>,
    ignore_indices: bool,
    boundary: SubdivisionBoundary,
    zero: T,
}

impl<T> LayerInput<'_, T> {
    fn index(&self, ix: u32) -> u32 {
        self.indices.get(ix as usize).copied().unwrap_or(NO_INDEX)
    }

    fn value(&self, value_ix: u32) -> &T {
        self.values.get(value_ix as usize).unwrap_or(&self.zero)
    }

    fn at(&self, ix: u32) -> &T {
        self.value(self.index(ix))
    }
}

struct LayerOutput<T> {
    values: Vec<T>,
    indices: Vec<u32>,
}

trait AttribValue: Copy + Default {
    /// `ufbxi_subdivide_sum_vec2`, `ufbxi_subdivide_sum_vec3` and `ufbxi_subdivide_sum_vec4` in ufbx.c
    fn sum(inputs: &[(&Self, Real)]) -> Self;
    fn same_bytes(a: &Self, b: &Self) -> bool;
}

impl AttribValue for Vec2 {
    fn sum(inputs: &[(&Self, Real)]) -> Self {
        let mut dst = Vec2::ZERO;
        for &(src, weight) in inputs {
            dst.x += src.x * weight;
            dst.y += src.y * weight;
        }
        dst
    }
    fn same_bytes(a: &Self, b: &Self) -> bool {
        a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits()
    }
}

impl AttribValue for Vec3 {
    fn sum(inputs: &[(&Self, Real)]) -> Self {
        let mut dst = Vec3::ZERO;
        for &(src, weight) in inputs {
            dst.x += src.x * weight;
            dst.y += src.y * weight;
            dst.z += src.z * weight;
        }
        dst
    }
    fn same_bytes(a: &Self, b: &Self) -> bool {
        a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits() && a.z.to_bits() == b.z.to_bits()
    }
}

impl AttribValue for Vec4 {
    fn sum(inputs: &[(&Self, Real)]) -> Self {
        let mut dst = Vec4::ZERO;
        for &(src, weight) in inputs {
            dst.x += src.x * weight;
            dst.y += src.y * weight;
            dst.z += src.z * weight;
            dst.w += src.w * weight;
        }
        dst
    }
    fn same_bytes(a: &Self, b: &Self) -> bool {
        a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits() && a.z.to_bits() == b.z.to_bits() && a.w.to_bits() == b.w.to_bits()
    }
}

/// The state of `ufbxi_subdivide_sum_vertex_weights`
struct WeightSum {
    vertex_weights: Vec<Real>,
    tmp_weights: Vec<SubdivisionWeight>,
    total_weights: usize,
    max_vertex_weights: usize,
}

fn subdivision_weight_less(a: &SubdivisionWeight, b: &SubdivisionWeight) -> bool {
    if a.weight != b.weight {
        return a.weight > b.weight;
    }
    a.index < b.index
}

impl WeightSum {
    fn subdivide_sum_vertex_weights(&mut self, inputs: &[(&VertexWeights, Real)]) -> Res<VertexWeights> {
        self.tmp_weights.clear();
        for &(src, input_weight) in inputs {
            for w in src {
                let weight = input_weight * w.weight;
                if weight < f32::MIN_POSITIVE as Real {
                    continue;
                }
                let vx = w.index as usize;
                check!(vx < self.vertex_weights.len());
                let prev = self.vertex_weights[vx];
                self.vertex_weights[vx] = prev + weight;
                if prev == 0.0 {
                    self.tmp_weights.push(SubdivisionWeight { weight: 0.0, index: w.index });
                }
            }
        }

        for t in &mut self.tmp_weights {
            let vx = t.index as usize;
            t.weight = self.vertex_weights[vx];
            self.vertex_weights[vx] = 0.0;
        }

        unstable_sort(&mut self.tmp_weights, subdivision_weight_less);

        let mut num_weights = self.tmp_weights.len();
        if self.max_vertex_weights != usize::MAX {
            num_weights = self.max_vertex_weights.min(num_weights);
            let mut prefix_weight = 0.0;
            for t in &self.tmp_weights[..num_weights] {
                prefix_weight += t.weight;
            }
            for t in &mut self.tmp_weights[..num_weights] {
                t.weight /= prefix_weight;
            }
        }

        self.total_weights += num_weights;
        Ok(self.tmp_weights[..num_weights].to_vec())
    }
}

fn is_edge_split<T>(input: &LayerInput<T>, topo: &[TopoEdge], index: u32) -> bool {
    let twin = topo[index as usize].twin;
    if twin != NO_INDEX {
        let a0 = input.index(index);
        let a1 = input.index(topo[index as usize].next);
        let b0 = input.index(topo[twin as usize].next);
        let b1 = input.index(twin);
        if a0 == b0 && a1 == b1 {
            return false;
        }
        let Some(same) = input.same_data else { return true };
        if same(input.value(a0), input.value(b0)) && same(input.value(a1), input.value(b1)) {
            return false;
        }
        return true;
    }
    false
}

fn edge_crease_value(mesh: &Mesh, edge: u32) -> Option<Real> {
    if edge == NO_INDEX {
        return None;
    }
    mesh.edge_crease.get(edge as usize).map(|&c| c * 10.0)
}

fn edge_crease(mesh: &Mesh, split: bool, topo: &[TopoEdge], index: u32) -> Real {
    let te = &topo[index as usize];
    if te.twin == NO_INDEX {
        return 1.0;
    }
    if split {
        return 1.0;
    }
    edge_crease_value(mesh, te.edge).unwrap_or(0.0)
}

/// The new values are the face points, then the edge points, then the vertex points
fn subdivide_layer<T: Clone + Default>(
    mesh: &Mesh,
    topo: &[TopoEdge],
    input: &LayerInput<T>,
    sum: &mut dyn FnMut(&[(&T, Real)]) -> Res<T>,
) -> Res<LayerOutput<T>> {
    let num_indices = mesh.num_indices;

    let mut edge_indices = vec![0u32; num_indices];
    let mut num_edge_values = 0u32;
    for ix in 0..num_indices as u32 {
        let twin = topo[ix as usize].twin;
        if twin < ix && !is_edge_split(input, topo, ix) {
            edge_indices[ix as usize] = edge_indices[twin as usize];
        } else {
            edge_indices[ix as usize] = num_edge_values;
            num_edge_values += 1;
        }
    }

    let mut vertex_indices = vec![NO_INDEX; num_indices];

    let (sharp_corners, sharp_splits, sharp_all) = match input.boundary {
        SubdivisionBoundary::Default | SubdivisionBoundary::SharpNone | SubdivisionBoundary::Legacy => (false, false, false),
        SubdivisionBoundary::SharpCorners => (true, false, false),
        SubdivisionBoundary::SharpBoundary => (true, true, false),
        SubdivisionBoundary::SharpInterior => (false, false, true),
    };

    let mut face_values: Vec<T> = Vec::with_capacity(mesh.num_faces);
    {
        let mut inputs: Vec<(&T, Real)> = Vec::new();
        for fi in 0..mesh.num_faces {
            let face = mesh.faces[fi];
            let weight = 1.0 / face.num_indices as Real;
            inputs.clear();
            for ci in 0..face.num_indices {
                inputs.push((input.at(face.index_begin + ci), weight));
            }
            face_values.push(sum(&inputs)?);
        }
    }

    let mut edge_values: Vec<T> = vec![T::default(); num_edge_values as usize];
    let mut vertex_values: Vec<T> = Vec::new();
    {
        let face_values = &face_values;
        let mut inputs: Vec<(&T, Real)> = Vec::with_capacity(32);

        for ix in 0..num_indices as u32 {
            let te = topo[ix as usize];
            let dst = edge_indices[ix as usize] as usize;

            let twin = te.twin;
            let split = is_edge_split(input, topo, ix);

            let mut crease = 0.0;
            if split || twin == NO_INDEX {
                crease = 1.0;
            } else if let Some(c) = edge_crease_value(mesh, te.edge) {
                crease = c;
            }
            if sharp_all {
                crease = 1.0;
            }

            let v0 = input.at(ix);
            let v1 = input.at(te.next);

            if twin < ix && !split {
                // Already computed from the twin
            } else if crease <= 0.0 {
                let f0 = &face_values[te.face as usize];
                let f1 = &face_values[topo[twin as usize].face as usize];
                edge_values[dst] = sum(&[(v0, 0.25), (v1, 0.25), (f0, 0.25), (f1, 0.25)])?;
            } else if crease >= 1.0 {
                edge_values[dst] = sum(&[(v0, 0.5), (v1, 0.5)])?;
            } else if crease < 1.0 {
                let f0 = &face_values[te.face as usize];
                let f1 = &face_values[topo[twin as usize].face as usize];
                let w0 = 0.25 + 0.25 * crease;
                let w1 = 0.25 - 0.25 * crease;
                edge_values[dst] = sum(&[(v0, w0), (v1, w0), (f0, w1), (f1, w1)])?;
            }
        }

        for vi in 0..mesh.num_vertices {
            let mut original_start = mesh.vertex_first_index[vi];
            if original_start == NO_INDEX {
                continue;
            }

            // Start from a topological boundary, else from a split edge, else anywhere on the loop
            let mut start = original_start;
            let mut cur = start;
            loop {
                let prev = topo_prev_vertex_edge(topo, cur);
                if prev == NO_INDEX {
                    start = cur;
                    break;
                }
                if is_edge_split(input, topo, prev) {
                    start = cur;
                }
                if prev == original_start {
                    break;
                }
                cur = prev;
            }

            original_start = start;
            while start != NO_INDEX {
                let value_index = vertex_values.len() as u32;

                let mut total_crease: Real = 0.0;
                let mut num_crease = 0usize;
                let mut num_split = 0usize;
                let mut on_boundary = false;
                let mut non_manifold = false;
                let mut crease_input_indices = [0usize; 2];

                let st = topo[start as usize];
                let start_prev = st.prev;
                let end_edge = topo[start_prev as usize].twin;
                let mut valence = 2usize;

                non_manifold |= st.flags & TOPO_NON_MANIFOLD != 0;
                non_manifold |= topo[start_prev as usize].flags & TOPO_NON_MANIFOLD != 0;

                let v0 = input.at(start);
                inputs.clear();
                inputs.push((v0, 0.0));
                inputs.push((input.at(st.next), 0.0));
                inputs.push((input.at(start_prev), 0.0));
                inputs.push((&face_values[st.face as usize], 0.0));

                let start_split = is_edge_split(input, topo, start);
                let prev_split = end_edge != NO_INDEX && is_edge_split(input, topo, end_edge);

                let start_crease = edge_crease(mesh, start_split, topo, start);
                if start_crease > 0.0 {
                    total_crease += start_crease;
                    crease_input_indices[num_crease] = 1;
                    num_crease += 1;
                }
                let prev_crease = edge_crease(mesh, prev_split, topo, start_prev);
                if prev_crease > 0.0 {
                    total_crease += prev_crease;
                    crease_input_indices[num_crease] = 2;
                    num_crease += 1;
                }

                if end_edge != NO_INDEX {
                    if prev_split {
                        num_split += 1;
                    }
                } else {
                    on_boundary = true;
                }

                check!(vertex_indices[start as usize] == NO_INDEX);
                vertex_indices[start as usize] = value_index;

                if start_split {
                    start = topo_next_vertex_edge(topo, start);
                    num_split += 1;
                } else {
                    let mut cur = start;
                    loop {
                        cur = topo_next_vertex_edge(topo, cur);

                        if cur == NO_INDEX {
                            on_boundary = true;
                            start = NO_INDEX;
                            break;
                        }

                        let ct = topo[cur as usize];
                        non_manifold |= ct.flags & TOPO_NON_MANIFOLD != 0;
                        check!(vertex_indices[cur as usize] == NO_INDEX);
                        vertex_indices[cur as usize] = value_index;

                        let split = is_edge_split(input, topo, cur);

                        // Looped back to `start_prev`: its face is still to add
                        if cur == end_edge && !split {
                            inputs.push((&face_values[ct.face as usize], 0.0));
                            start = NO_INDEX;
                            break;
                        }

                        let cur_crease = edge_crease(mesh, split, topo, cur);
                        if cur_crease > 0.0 {
                            total_crease += cur_crease;
                            if num_crease < 2 {
                                crease_input_indices[num_crease] = inputs.len();
                            }
                            num_crease += 1;
                        }

                        inputs.push((input.at(ct.next), 0.0));
                        inputs.push((&face_values[ct.face as usize], 0.0));
                        valence += 1;

                        if split {
                            start = topo_next_vertex_edge(topo, cur);
                            num_split += 1;
                            break;
                        }
                    }
                }

                if start == original_start {
                    start = NO_INDEX;
                }

                let fe_weight = 1.0 / (valence * valence) as Real;
                let v_weight = (valence - 2) as Real / valence as Real;

                if num_crease > 2
                    || (sharp_corners && valence == 2 && (num_split > 0 || on_boundary))
                    || (sharp_splits && (num_split > 0 || on_boundary))
                    || sharp_all
                    || non_manifold
                {
                    inputs.truncate(1);
                    inputs[0] = (v0, 1.0);
                } else if num_crease == 2 {
                    total_crease = (total_crease * 0.5).clamp(0.0, 1.0);

                    inputs[0].1 = v_weight * (1.0 - total_crease) + 0.75 * total_crease;
                    let few = fe_weight * (1.0 - total_crease);
                    for inp in &mut inputs[1..] {
                        inp.1 = few;
                    }

                    inputs[crease_input_indices[0]].1 += 0.125 * total_crease;
                    inputs[crease_input_indices[1]].1 += 0.125 * total_crease;
                } else {
                    inputs[0].1 = v_weight;
                    for inp in &mut inputs[1..] {
                        inp.1 = fe_weight;
                    }
                }

                if mesh.vertex_crease.exists {
                    let mut v = mesh.vertex_crease.get(original_start as usize);
                    v *= 10.0;
                    if v > 0.0 {
                        if v > 1.0 {
                            v = 1.0;
                        }
                        let iv = 1.0 - v;
                        inputs[0].1 = v + inputs[0].1 * iv;
                        for inp in &mut inputs[1..] {
                            inp.1 *= iv;
                        }
                    }
                }

                vertex_values.push(sum(&inputs)?);
            }
        }

        // Corners no vertex loop reached, at non-manifold edges, keep their values
        for old_ix in 0..num_indices {
            if vertex_indices[old_ix] == NO_INDEX {
                vertex_indices[old_ix] = vertex_values.len() as u32;
                vertex_values.push(sum(&[(input.at(old_ix as u32), 1.0)])?);
            }
        }
    }

    let indices = if !input.ignore_indices {
        let face_start = 0u32;
        let edge_start = face_start + mesh.num_faces as u32;
        let vert_start = edge_start + num_edge_values;
        let mut indices = Vec::with_capacity(num_indices * 4);
        for ix in 0..num_indices {
            let te = &topo[ix];
            indices.push(vert_start + vertex_indices[ix]);
            indices.push(edge_start + edge_indices[ix]);
            indices.push(face_start + te.face);
            indices.push(edge_start + edge_indices[te.prev as usize]);
        }
        indices
    } else {
        Vec::new()
    };

    let mut values = face_values;
    values.append(&mut edge_values);
    values.append(&mut vertex_values);
    Ok(LayerOutput { values, indices })
}

/// As ufbx, it leaves `unique_per_vertex` and `values_w` as they were
fn subdivide_attrib<T: AttribValue>(
    mesh: &Mesh,
    topo: &[TopoEdge],
    attrib: &mut VertexAttrib<T>,
    boundary: SubdivisionBoundary,
    check_split_data: bool,
) -> Res<()> {
    if !attrib.exists {
        return Ok(());
    }
    let output = {
        let input = LayerInput {
            values: &attrib.values,
            indices: &attrib.indices,
            same_data: if check_split_data { Some(T::same_bytes as fn(&T, &T) -> bool) } else { None },
            ignore_indices: false,
            boundary,
            zero: T::default(),
        };
        subdivide_layer(mesh, topo, &input, &mut |inputs| Ok(T::sum(inputs)))?
    };
    attrib.values = output.values;
    attrib.indices = output.indices;
    Ok(())
}

fn subdivision_copy_weights(ranges: &[SubdivisionWeightRange], weights: &[SubdivisionWeight]) -> Res<Vec<VertexWeights>> {
    ranges
        .iter()
        .map(|range| {
            let begin = range.weight_begin as usize;
            let end = begin + range.num_weights as usize;
            check!(end <= weights.len());
            Ok(weights[begin..end].to_vec())
        })
        .collect()
}

fn init_source_vertex_weights(num_vertices: usize) -> Vec<VertexWeights> {
    (0..num_vertices).map(|i| vec![SubdivisionWeight { index: i as u32, weight: 1.0 }]).collect()
}

fn init_skin_weights(num_vertices: usize, skin: &SkinDeformer, max_vertex_weights: usize) -> Res<Vec<VertexWeights>> {
    let mut dst = Vec::with_capacity(num_vertices);
    for i in 0..num_vertices {
        check!(i < skin.vertices.len());
        let vertex = skin.vertices[i];
        let num_weights = max_vertex_weights.min(vertex.num_weights as usize);
        let begin = vertex.weight_begin as usize;
        check!(begin + num_weights <= skin.weights.len());

        let mut weights = Vec::with_capacity(num_weights);
        for sw in &skin.weights[begin..begin + num_weights] {
            check!(sw.cluster_index <= i32::MAX as u32);
            weights.push(SubdivisionWeight { index: sw.cluster_index, weight: sw.weight });
        }
        dst.push(weights);
    }
    Ok(dst)
}

fn subdivide_weights(
    mesh: &Mesh,
    topo: &[TopoEdge],
    boundary: SubdivisionBoundary,
    ws: &mut WeightSum,
    src: &[VertexWeights],
) -> Res<(Vec<SubdivisionWeightRange>, Vec<SubdivisionWeight>)> {
    let input = LayerInput { values: src, indices: &mesh.vertex_indices, same_data: None, ignore_indices: true, boundary, zero: Vec::new() };

    ws.total_weights = 0;
    let output = subdivide_layer(mesh, topo, &input, &mut |inputs| ws.subdivide_sum_vertex_weights(inputs))?;

    let num_vertices = output.values.len();
    let mut ranges = Vec::with_capacity(num_vertices);
    let mut weights = Vec::with_capacity(ws.total_weights);
    let mut weight_offset = 0usize;
    for vertex_weights in &output.values {
        check!(u32::MAX as usize - weight_offset >= vertex_weights.len());
        ranges.push(SubdivisionWeightRange { weight_begin: weight_offset as u32, num_weights: vertex_weights.len() as u32 });
        weights.extend_from_slice(vertex_weights);
        weight_offset += vertex_weights.len();
    }
    Ok((ranges, weights))
}

/// Each level lowers the crease by 0.1 and keeps it only at the original corner
fn subdivide_vertex_crease(dst: &mut VertexReal, src: &VertexReal) {
    let src_values = src.values.len();

    let mut values = Vec::with_capacity(src_values + 1);
    for &value in &src.values {
        let mut crease = value;
        if crease < 0.999f32 as Real {
            crease -= 0.1f32 as Real;
        }
        if crease < 0.0 {
            crease = 0.0;
        }
        values.push(crease);
    }
    values.push(0.0);

    let zero_index = src_values as u32;
    let mut indices = Vec::with_capacity(src.indices.len() * 4);
    for &ix in &src.indices {
        indices.extend_from_slice(&[ix, zero_index, zero_index, zero_index]);
    }

    dst.values = values;
    dst.indices = indices;
}

fn subdivide_mesh_level(scene: &Scene, mesh: &Mesh, opts: &SubdivideOpts) -> Res<Mesh> {
    let mut result = mesh.clone();

    let mut topo = vec![TopoEdge::default(); mesh.num_indices];
    compute_topology(mesh, &mut topo);

    subdivide_attrib(mesh, &topo, &mut result.vertex_position, opts.boundary, false)?;

    result.vertex_uv = VertexVec2::default();
    result.vertex_tangent = VertexVec3::default();
    result.vertex_bitangent = VertexVec3::default();
    result.vertex_color = VertexVec4::default();

    for set in &mut result.uv_sets {
        subdivide_attrib(mesh, &topo, &mut set.vertex_uv, opts.uv_boundary, true)?;
        if opts.interpolate_tangents {
            subdivide_attrib(mesh, &topo, &mut set.vertex_tangent, opts.uv_boundary, true)?;
            subdivide_attrib(mesh, &topo, &mut set.vertex_bitangent, opts.uv_boundary, true)?;
        } else {
            set.vertex_tangent = VertexVec3::default();
            set.vertex_bitangent = VertexVec3::default();
        }
    }

    for set in &mut result.color_sets {
        subdivide_attrib(mesh, &topo, &mut set.vertex_color, opts.uv_boundary, true)?;
    }

    if let Some(set) = result.uv_sets.first() {
        result.vertex_uv = set.vertex_uv.clone();
        result.vertex_bitangent = set.vertex_bitangent.clone();
        result.vertex_tangent = set.vertex_tangent.clone();
    }
    if let Some(set) = result.color_sets.first() {
        result.vertex_color = set.vertex_color.clone();
    }

    // ufbx tests whether the skinned and the plain attribute share their values; equal ones subdivide alike
    if opts.interpolate_normals && !opts.ignore_normals {
        subdivide_attrib(mesh, &topo, &mut result.vertex_normal, opts.boundary, true)?;
        for normal in &mut result.vertex_normal.values {
            *normal = normalize3(*normal);
        }
        if mesh.skinned_normal == mesh.vertex_normal {
            result.skinned_normal = result.vertex_normal.clone();
        } else {
            subdivide_attrib(mesh, &topo, &mut result.skinned_normal, opts.boundary, true)?;
            for normal in &mut result.skinned_normal.values {
                *normal = normalize3(*normal);
            }
        }
    }

    if result.vertex_crease.exists {
        subdivide_vertex_crease(&mut result.vertex_crease, &mesh.vertex_crease);
    }

    if mesh.skinned_position == mesh.vertex_position {
        result.skinned_position = result.vertex_position.clone();
    } else {
        subdivide_attrib(mesh, &topo, &mut result.skinned_position, opts.boundary, false)?;
    }

    let mut result_sub = SubdivisionResult::default();

    if opts.evaluate_source_vertices || opts.evaluate_skin_weights {
        let mesh_sub = mesh.subdivision_result.as_deref();

        let mut skin = None;
        if opts.evaluate_skin_weights && !mesh.skin_deformers.is_empty() {
            check!(opts.skin_deformer_index < mesh.skin_deformers.len());
            skin = scene.skin_deformers.get(mesh.skin_deformers[opts.skin_deformer_index]);
            check!(skin.is_some());
        }

        let mut max_weights = 0;
        if opts.evaluate_source_vertices {
            max_weights = max_weights.max(mesh.num_vertices);
        }
        if let Some(skin) = skin {
            max_weights = max_weights.max(skin.clusters.len());
        }

        // ufbx sizes this by the vertices alone, so cluster indices past them would write out of bounds
        let mut ws = WeightSum {
            vertex_weights: vec![0.0; mesh.num_vertices.max(max_weights)],
            tmp_weights: Vec::with_capacity(max_weights),
            total_weights: 0,
            max_vertex_weights: usize::MAX,
        };

        if opts.evaluate_source_vertices {
            ws.max_vertex_weights = if opts.max_source_vertices != 0 { opts.max_source_vertices } else { usize::MAX };

            let weights = match mesh_sub {
                Some(sub) if !sub.source_vertex_ranges.is_empty() => subdivision_copy_weights(&sub.source_vertex_ranges, &sub.source_vertex_weights)?,
                _ => init_source_vertex_weights(mesh.num_vertices),
            };

            let (ranges, weights) = subdivide_weights(mesh, &topo, opts.boundary, &mut ws, &weights)?;
            result_sub.source_vertex_ranges = ranges;
            result_sub.source_vertex_weights = weights;
        }

        if let Some(skin) = skin {
            ws.max_vertex_weights = if opts.max_skin_weights != 0 { opts.max_skin_weights } else { usize::MAX };

            // ufbx tests the source vertex ranges here too, not the skin cluster ones
            let weights = match mesh_sub {
                Some(sub) if !sub.source_vertex_ranges.is_empty() => subdivision_copy_weights(&sub.skin_cluster_ranges, &sub.skin_cluster_weights)?,
                _ => init_skin_weights(mesh.num_vertices, skin, ws.max_vertex_weights)?,
            };

            let (ranges, weights) = subdivide_weights(mesh, &topo, opts.boundary, &mut ws, &weights)?;
            result_sub.skin_cluster_ranges = ranges;
            result_sub.skin_cluster_weights = weights;
        }
    }

    result.subdivision_result = Some(Box::new(result_sub));

    result.num_vertices = result.vertex_position.values.len();
    result.num_indices = mesh.num_indices * 4;
    result.num_faces = mesh.num_indices;
    result.num_triangles = mesh.num_indices * 2;

    result.vertex_indices = result.vertex_position.indices.clone();
    result.vertices = result.vertex_position.values.clone();

    result.faces = (0..result.num_faces).map(|i| Face { index_begin: (i * 4) as u32, num_indices: 4 }).collect();

    if !mesh.edges.is_empty() || mesh.has_edges {
        result.has_edges = true;
        result.num_edges = mesh.num_edges * 2 + result.num_faces;
        let num_edges = result.num_edges;

        let has_crease = !mesh.edge_crease.is_empty();
        let has_smoothing = !mesh.edge_smoothing.is_empty() || mesh.has_edge_smoothing;
        let has_visibility = !mesh.edge_visibility.is_empty();

        let mut edges = Vec::with_capacity(num_edges);
        let mut edge_crease = Vec::with_capacity(if has_crease { num_edges } else { 0 });
        let mut edge_smoothing = Vec::with_capacity(if has_smoothing { num_edges } else { 0 });
        let mut edge_visibility = Vec::with_capacity(if has_visibility { num_edges } else { 0 });

        for i in 0..mesh.num_edges {
            let edge = mesh.edges[i];
            let face_ix = topo[edge.a as usize].face;
            let face = mesh.faces[face_ix as usize];
            let offset = edge.a - face.index_begin;
            let next = (offset + 1) % face.num_indices;

            let a = (face.index_begin + offset).wrapping_mul(4);
            let b = (face.index_begin + next).wrapping_mul(4);

            edges.push(Edge { a, b: a + 1 });
            edges.push(Edge { a: b + 3, b });

            if has_crease {
                let mut crease = mesh.edge_crease[i];
                if crease < 0.999f32 as Real {
                    crease -= 0.1;
                }
                if crease < 0.0 {
                    crease = 0.0;
                }
                edge_crease.extend_from_slice(&[crease, crease]);
            }
            if has_smoothing {
                edge_smoothing.extend_from_slice(&[mesh.edge_smoothing[i]; 2]);
            }
            if has_visibility {
                edge_visibility.extend_from_slice(&[mesh.edge_visibility[i]; 2]);
            }
        }

        for fi in 0..result.num_faces {
            edges.push(Edge { a: (fi * 4 + 1) as u32, b: (fi * 4 + 2) as u32 });
            if has_crease {
                edge_crease.push(0.0);
            }
            if has_smoothing {
                edge_smoothing.push(true);
            }
            if has_visibility {
                edge_visibility.push(false);
            }
        }

        result.edges = edges;
        if has_crease {
            result.edge_crease = edge_crease;
        }
        if has_smoothing {
            result.edge_smoothing = edge_smoothing;
            result.has_edge_smoothing = true;
        }
        if has_visibility {
            result.edge_visibility = edge_visibility;
        }
    }

    let num_faces = result.num_faces;
    if !mesh.face_material.is_empty() {
        result.face_material = vec![0; num_faces];
    }
    if !mesh.face_smoothing.is_empty() {
        result.face_smoothing = vec![false; num_faces];
    }
    if !mesh.face_group.is_empty() {
        result.face_group = vec![0; num_faces];
    }
    if !mesh.face_hole.is_empty() {
        result.face_hole = vec![false; num_faces];
    }

    if !result.material_parts.is_empty() {
        result.material_parts = vec![MeshPart::default(); result.material_parts.len()];
    }

    let mut index_offset = 0;
    for i in 0..mesh.num_faces {
        let face = mesh.faces[i];
        let corners = index_offset..index_offset + face.num_indices as usize;
        if !mesh.face_material.is_empty() {
            result.face_material[corners.clone()].fill(mesh.face_material[i]);
        }
        if !mesh.face_smoothing.is_empty() {
            result.face_smoothing[corners.clone()].fill(mesh.face_smoothing[i]);
        }
        if !mesh.face_group.is_empty() {
            result.face_group[corners.clone()].fill(mesh.face_group[i]);
        }
        if !mesh.face_hole.is_empty() {
            result.face_hole[corners].fill(mesh.face_hole[i]);
        }
        index_offset += face.num_indices as usize;
    }

    result.vertex_first_index = Vec::new();

    finalize_mesh_material(&mut result)?;
    finalize_mesh(&mut result)?;
    update_face_groups(&mut result);

    Ok(result)
}

fn subdivide_mesh_imp(scene: &Scene, mesh: &Mesh, level: usize, user_opts: &SubdivideOpts) -> Res<Mesh> {
    let mut opts = user_opts.clone();
    if opts.boundary == SubdivisionBoundary::Default {
        opts.boundary = mesh.subdivision_boundary;
    }
    if opts.uv_boundary == SubdivisionBoundary::Default {
        opts.uv_boundary = mesh.subdivision_uv_boundary;
    }

    let mut src: Option<Mesh> = None;
    for _ in 1..level {
        let dst = subdivide_mesh_level(scene, src.as_ref().unwrap_or(mesh), &opts)?;
        src = Some(dst);
    }
    let mut mesh = subdivide_mesh_level(scene, src.as_ref().unwrap_or(mesh), &opts)?;
    drop(src);

    mesh.max_face_triangles = 2;
    mesh.num_empty_faces = 0;
    mesh.num_point_faces = 0;
    mesh.num_line_faces = 0;

    if !opts.interpolate_normals {
        mesh.vertex_normal = VertexVec3::default();
        mesh.skinned_normal = VertexVec3::default();
    }

    if !opts.interpolate_normals && !opts.ignore_normals {
        let mut topo = vec![TopoEdge::default(); mesh.num_indices];
        compute_topology(&mesh, &mut topo);

        let mut normal_indices = vec![0u32; mesh.num_indices];
        let num_normals = generate_normal_mapping(&mesh, &topo, &mut normal_indices, true);

        let mut normals = vec![Vec3::ZERO; num_normals];
        compute_normals(&mesh, &mesh.skinned_position, &normal_indices, &mut normals);

        mesh.generated_normals = true;
        mesh.vertex_normal.exists = true;
        mesh.vertex_normal.values = normals;
        mesh.vertex_normal.indices = normal_indices;

        mesh.skinned_normal = mesh.vertex_normal.clone();
    }

    patch_mesh_reals(&mut mesh);
    mesh.subdivision_evaluated = true;
    Ok(mesh)
}

/// `scene` holds the skin deformers its weights come from
pub fn subdivide_mesh(scene: &Scene, mesh: &Mesh, level: usize, opts: &SubdivideOpts) -> Res<Mesh> {
    if level == 0 {
        return Ok(mesh.clone());
    }
    subdivide_mesh_imp(scene, mesh, level, opts).map_err(|e| e.fix_error_type("Failed to subdivide"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{compute_topology, topo_next_vertex_edge};

    fn cube() -> Mesh {
        let p = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        let faces: [&[u32]; 6] = [&[0, 3, 2, 1], &[4, 5, 6, 7], &[0, 1, 5, 4], &[1, 2, 6, 5], &[2, 3, 7, 6], &[3, 0, 4, 7]];
        let mut mesh = Mesh::default();
        for (i, f) in faces.iter().enumerate() {
            mesh.faces.push(Face { index_begin: i as u32 * 4, num_indices: 4 });
            mesh.vertex_indices.extend_from_slice(f);
        }
        mesh.vertex_position.exists = true;
        mesh.vertex_position.values = p.iter().map(|&v| Vec3::from_array(v)).collect();
        mesh.vertex_position.indices = mesh.vertex_indices.clone();
        finalize_mesh(&mut mesh).unwrap();
        mesh
    }

    fn with_edges(mut mesh: Mesh, crease: Real) -> Mesh {
        let mut topo = vec![TopoEdge::default(); mesh.num_indices];
        compute_topology(&mesh, &mut topo);
        for t in &topo {
            if t.twin == NO_INDEX || t.twin > t.index {
                mesh.edges.push(Edge { a: t.index, b: t.next });
            }
        }
        mesh.num_edges = mesh.edges.len();
        mesh.edge_crease = vec![crease; mesh.num_edges];
        mesh
    }

    fn close(a: Vec3, b: [Real; 3]) -> bool {
        (a.x - b[0]).abs() < 1e-14 && (a.y - b[1]).abs() < 1e-14 && (a.z - b[2]).abs() < 1e-14
    }

    fn assert_closed_quads(mesh: &Mesh) {
        assert!(mesh.faces.iter().enumerate().all(|(i, f)| f.index_begin == i as u32 * 4 && f.num_indices == 4));
        let mut topo = vec![TopoEdge::default(); mesh.num_indices];
        compute_topology(mesh, &mut topo);
        assert!(topo.iter().all(|t| t.twin != NO_INDEX && t.flags == 0));
        for start in 0..mesh.num_indices as u32 {
            let (mut n, mut cur) = (0, start);
            loop {
                cur = topo_next_vertex_edge(&topo, cur);
                n += 1;
                if cur == start {
                    break;
                }
            }
            assert!(n == 3 || n == 4);
        }
    }

    #[test]
    fn cube_level_1() {
        let cube = cube();
        let sub = subdivide_mesh(&Scene::default(), &cube, 1, &SubdivideOpts::default()).unwrap();
        assert_eq!((sub.num_vertices, sub.num_faces, sub.num_indices, sub.num_triangles), (26, 24, 96, 48));
        assert_closed_quads(&sub);

        assert!(close(sub.vertices[0], [0.0, 0.0, -1.0]));
        assert!(close(sub.vertices[1], [0.0, 0.0, 1.0]));
        let edge_points = &sub.vertices[6..18];
        assert!(edge_points.iter().all(|v| {
            let a = [v.x.abs(), v.y.abs(), v.z.abs()];
            let mut s = a;
            s.sort_by(|x, y| x.partial_cmp(y).unwrap());
            (s[0] - 0.0).abs() < 1e-15 && (s[1] - 0.75).abs() < 1e-15 && (s[2] - 0.75).abs() < 1e-15
        }));
        let corner = -5.0 / 9.0;
        assert!(close(sub.vertices[18], [corner, corner, corner]));
        for (i, &vi) in cube.vertex_indices.iter().enumerate() {
            let quad = &sub.vertex_indices[i * 4..i * 4 + 4];
            let p = cube.vertices[vi as usize];
            assert!(close(sub.vertices[quad[0] as usize], [p.x * 5.0 / 9.0, p.y * 5.0 / 9.0, p.z * 5.0 / 9.0]));
            assert_eq!(quad[2], i as u32 / 4);
        }

        assert!(sub.generated_normals && sub.vertex_normal.exists);
        assert_eq!(sub.vertex_normal.values.len(), 26);
        for i in 0..sub.num_indices {
            let (p, n) = (sub.vertex_position.get(i), sub.vertex_normal.get(i));
            assert!((length3(n) - 1.0).abs() < 1e-14 && dot3(p, n) > 0.0);
        }
        assert_eq!(sub.vertex_first_index.len(), 26);
        assert!(sub.subdivision_evaluated);
    }

    #[test]
    fn cube_level_2_is_level_1_twice() {
        let cube = cube();
        let opts = SubdivideOpts::default();
        let scene = Scene::default();
        let two = subdivide_mesh(&scene, &cube, 2, &opts).unwrap();
        assert_eq!((two.num_vertices, two.num_faces, two.num_indices), (98, 96, 384));
        assert_closed_quads(&two);

        let twice = subdivide_mesh(&scene, &subdivide_mesh(&scene, &cube, 1, &opts).unwrap(), 1, &opts).unwrap();
        assert_eq!(two.vertex_position, twice.vertex_position);
        assert_eq!(two.vertex_normal, twice.vertex_normal);
        assert_eq!(two.faces, twice.faces);
        for v in &two.vertices {
            assert!(v.x.abs() < 1.0 && v.y.abs() < 1.0 && v.z.abs() < 1.0);
        }
    }

    #[test]
    fn fully_creased_cube_stays_a_cube() {
        let cube = with_edges(cube(), 1.0);
        let opts = SubdivideOpts { evaluate_source_vertices: true, ..Default::default() };
        let sub = subdivide_mesh(&Scene::default(), &cube, 1, &opts).unwrap();
        assert_eq!(sub.num_edges, 12 * 2 + 24);
        assert_eq!(sub.edge_crease.len(), sub.num_edges);
        assert!(sub.edge_crease[..24].iter().all(|&c| c == 1.0) && sub.edge_crease[24..].iter().all(|&c| c == 0.0));
        for v in &sub.vertices {
            assert_eq!(v.x.abs().max(v.y.abs()).max(v.z.abs()), 1.0);
        }
        let res = sub.subdivision_result.as_deref().unwrap();
        assert_eq!(res.source_vertex_ranges.len(), 26);
        for (i, &vi) in cube.vertex_indices.iter().enumerate() {
            let v = sub.vertex_indices[i * 4];
            assert_eq!(sub.vertices[v as usize], cube.vertices[vi as usize]);
            let range = res.source_vertex_ranges[v as usize];
            assert_eq!(range.num_weights, 1);
            let w = res.source_vertex_weights[range.weight_begin as usize];
            assert_eq!((w.index, w.weight), (vi, 1.0));
        }
        let face_range = res.source_vertex_ranges[0];
        assert_eq!(face_range.num_weights, 4);
        let ws = &res.source_vertex_weights[face_range.weight_begin as usize..][..4];
        assert!(ws.iter().all(|w| w.weight == 0.25));
        assert_eq!(ws.iter().map(|w| w.index).collect::<Vec<_>>(), [0, 1, 2, 3]);
    }

    #[test]
    fn level_0_returns_the_mesh() {
        let cube = cube();
        assert_eq!(subdivide_mesh(&Scene::default(), &cube, 0, &SubdivideOpts::default()).unwrap(), cube);
    }
}

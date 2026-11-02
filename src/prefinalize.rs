use crate::context::Context;
use crate::error::Res;
use crate::math::{add3, is_vec3_zero, neg3, sub3};
use crate::props::{deduplicate_properties, sort_properties};
use crate::types::*;

const NONE: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, Default)]
struct PreNode {
    has_constant_scale: bool,
    has_recursive_scale_helper: bool,
    has_skin_deformer: bool,
    constant_scale: Vec3,
    element_id: u32,
    first_child: u32,
    next_child: u32,
    parent: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct PreAnimValue {
    has_constant_value: bool,
    constant_value: Vec3,
}

fn pivot_nonzero(offset: Vec3) -> bool {
    let e = 0.0009765625;
    offset.x.abs() >= e || offset.y.abs() >= e || offset.z.abs() >= e
}

fn pivot_div(offset: Real, initial_scale: Real) -> Real {
    if initial_scale.abs() >= 0.0078125 { offset / initial_scale } else { offset }
}

impl<'a> Context<'a> {
    pub(crate) fn pre_finalize_scene(&mut self) -> Res<()> {
        let o = &self.opts;
        let required = matches!(o.geometry_transform_handling, GeometryTransformHandling::HelperNodes | GeometryTransformHandling::ModifyGeometry)
            || matches!(
                o.inherit_mode_handling,
                InheritModeHandling::HelperNodes | InheritModeHandling::Compensate | InheritModeHandling::CompensateNoFallback
            )
            || matches!(o.pivot_handling, PivotHandling::AdjustToPivot | PivotHandling::AdjustToRotationPivot);
        if !required {
            return Ok(());
        }

        let num_elements = self.scene.elements.len();
        let num_nodes = self.scene.nodes.len();
        let num_meshes = self.scene.meshes.len();
        let num_values = self.scene.anim_values.len();
        let mut instance_counts = vec![0u32; num_elements];
        let mut modify_not_supported = vec![false; num_elements];
        let mut node_attrib_type = vec![ElementType::Unknown; num_nodes];
        let mut has_unscaled_children = vec![false; num_nodes];
        let mut pre_nodes = vec![PreNode::default(); num_nodes];
        let mut pre_mesh_skin = vec![false; num_meshes];
        let mut pre_values = vec![PreAnimValue::default(); num_values];
        let fbx_ids = self.element_fbx_ids.clone();

        let scale_epsilon: Real = 0.001f32 as Real;
        let pivot_epsilon: Real = 0.001f32 as Real;
        let compensate_epsilon: Real = 0.01f32 as Real;

        for i in 0..num_elements {
            let r = self.scene.elements[i];
            match r.element_type {
                ElementType::Node => {
                    let n = &self.scene.nodes[r.index];
                    pre_nodes[r.index] = PreNode {
                        has_constant_scale: true,
                        constant_scale: n.element.props.vec3("Lcl Scaling", Vec3::ONE),
                        element_id: n.element.element_id,
                        first_child: NONE,
                        next_child: NONE,
                        parent: NONE,
                        ..Default::default()
                    };
                }
                ElementType::AnimValue => {
                    let p = &self.scene.anim_values[r.index].element.props;
                    let mut c = Vec3::ZERO;
                    c.x = p.real("d|X", p.real("X", Real::NAN));
                    c.y = p.real("d|Y", p.real("Y", Real::NAN));
                    c.z = p.real("d|Z", p.real("Z", Real::NAN));
                    pre_values[r.index] = PreAnimValue { has_constant_value: true, constant_value: c };
                }
                _ => {}
            }
        }

        let resolve = |ctx: &Context, id: u64| ctx.find_fbx_id(id).map(|e| e.element_id as usize);
        type Conn = (Option<usize>, Option<usize>, bool, bool, String);
        let conns: Vec<Conn> = self
            .tmp_connections
            .iter()
            .map(|c| (resolve(self, c.src), resolve(self, c.dst), c.src_prop.is_empty(), c.dst_prop.is_empty(), c.dst_prop.clone()))
            .collect();

        for (src, dst, src_empty, dst_empty, dst_prop) in &conns {
            let (Some(src), Some(dst)) = (*src, *dst) else { continue };
            let sr = self.scene.elements[src];
            let dr = self.scene.elements[dst];
            if *src_empty && *dst_empty {
                if dr.element_type == ElementType::Node {
                    if sr.element_type.is_attrib() {
                        instance_counts[src] += 1;
                        let count = instance_counts[src];
                        node_attrib_type[dr.index] = if count == 1 { sr.element_type } else { ElementType::Unknown };
                        if !matches!(sr.element_type, ElementType::Mesh | ElementType::LineCurve | ElementType::NurbsCurve | ElementType::NurbsSurface) {
                            modify_not_supported[dst] = true;
                        }
                    }
                    if sr.element_type == ElementType::Node {
                        let (si, di) = (sr.index, dr.index);
                        if pre_nodes[si].parent == NONE {
                            pre_nodes[si].parent = di as u32;
                            pre_nodes[si].next_child = pre_nodes[di].first_child;
                            pre_nodes[di].first_child = si as u32;
                        }
                        if self.opts.inherit_mode_handling != InheritModeHandling::Preserve
                            && !self.scene.nodes[di].is_root
                            && self.scene.nodes[si].original_inherit_mode != InheritMode::Normal
                        {
                            has_unscaled_children[di] = true;
                        }
                    }
                } else if dr.element_type == ElementType::Mesh && sr.element_type == ElementType::SkinDeformer {
                    pre_mesh_skin[dr.index] = true;
                }
            } else if *src_empty && !*dst_empty && dr.element_type == ElementType::AnimValue && sr.element_type == ElementType::AnimCurve {
                let curve = &self.scene.anim_curves[sr.index];
                let index = match dst_prop.as_str() {
                    "Y" | "d|Y" => 1,
                    "Z" | "d|Z" => 2,
                    _ => 0,
                };
                let pv = &mut pre_values[dr.index];
                if curve.max_value - curve.min_value >= scale_epsilon {
                    pv.has_constant_value = false;
                } else {
                    let constant = (curve.min_value + curve.max_value) * 0.5;
                    if pv.constant_value[index].is_nan() {
                        pv.constant_value[index] = constant;
                    }
                    if (pv.constant_value[index] - constant).abs() > scale_epsilon {
                        pv.has_constant_value = false;
                    }
                }
            }
        }

        for (src, dst, src_empty, dst_empty, dst_prop) in &conns {
            let (Some(src), Some(dst)) = (*src, *dst) else { continue };
            let sr = self.scene.elements[src];
            let dr = self.scene.elements[dst];
            if *src_empty && *dst_empty {
                if dr.element_type == ElementType::Node {
                    if sr.element_type.is_attrib() {
                        instance_counts[dst] = instance_counts[dst].max(instance_counts[src]);
                        if sr.element_type == ElementType::Mesh && pre_mesh_skin[sr.index] {
                            pre_nodes[dr.index].has_skin_deformer = true;
                        }
                    } else if sr.element_type == ElementType::SkinDeformer {
                        pre_nodes[dr.index].has_skin_deformer = true;
                    }
                }
            } else if *src_empty
                && !*dst_empty
                && dr.element_type == ElementType::Node
                && sr.element_type == ElementType::AnimValue
                && dst_prop == "Lcl Scaling"
            {
                let pn = &mut pre_nodes[dr.index];
                if pn.has_constant_scale {
                    let pv = &pre_values[sr.index];
                    if !pv.has_constant_value {
                        pn.has_constant_scale = false;
                    } else {
                        let mut error: Real = 0.0;
                        error += (pv.constant_value.x - pn.constant_scale.x).abs();
                        error += (pv.constant_value.y - pn.constant_scale.y).abs();
                        error += (pv.constant_value.z - pn.constant_scale.z).abs();
                        if error >= scale_epsilon {
                            pn.has_constant_scale = false;
                        }
                    }
                }
            }
        }

        if matches!(self.opts.pivot_handling, PivotHandling::AdjustToPivot | PivotHandling::AdjustToRotationPivot) {
            for i in 0..num_nodes {
                let pn = pre_nodes[i];
                let node_ix = self.scene.elements[pn.element_id as usize].index;
                let props = &self.scene.nodes[node_ix].element.props;
                let rotation_pivot = props.vec3("RotationPivot", Vec3::ZERO);
                let scaling_pivot = props.vec3("ScalingPivot", Vec3::ZERO);
                let scaling_offset = props.vec3("ScalingOffset", Vec3::ZERO);
                let should_modify = match self.opts.pivot_handling {
                    PivotHandling::AdjustToPivot => !is_vec3_zero(rotation_pivot),
                    _ => pivot_nonzero(rotation_pivot) || pivot_nonzero(scaling_pivot) || pivot_nonzero(scaling_offset),
                };
                if !should_modify {
                    continue;
                }
                let mut skip_geometry_transform = false;
                let mut can_modify_geometry_transform = true;
                if self.opts.pivot_handling == PivotHandling::AdjustToRotationPivot && node_attrib_type[node_ix] == ElementType::Empty {
                    if !self.opts.pivot_handling_retain_empties {
                        skip_geometry_transform = true;
                    } else {
                        can_modify_geometry_transform = false;
                    }
                }
                let eid = pn.element_id as usize;
                if self.opts.geometry_transform_handling == GeometryTransformHandling::ModifyGeometryNoFallback
                    && (instance_counts[eid] > 1 || modify_not_supported[eid])
                {
                    can_modify_geometry_transform = false;
                }
                // The geometry transform would break skinning
                if pn.has_skin_deformer {
                    can_modify_geometry_transform = false;
                }
                let mut can_modify_pivot = true;
                if self.opts.pivot_handling == PivotHandling::AdjustToPivot {
                    let mut err: Real = 0.0;
                    err += (rotation_pivot.x - scaling_pivot.x).abs();
                    err += (rotation_pivot.y - scaling_pivot.y).abs();
                    err += (rotation_pivot.z - scaling_pivot.z).abs();
                    if err > pivot_epsilon {
                        can_modify_pivot = false;
                    }
                }
                if !(can_modify_pivot && (can_modify_geometry_transform || skip_geometry_transform)) {
                    continue;
                }
                let mut geometric_translation = props.vec3("GeometricTranslation", Vec3::ZERO);
                let mut new_props = props.props.clone();
                let child_offset;
                if self.opts.pivot_handling == PivotHandling::AdjustToPivot {
                    child_offset = neg3(rotation_pivot);
                    geometric_translation = add3(geometric_translation, child_offset);
                    new_props.push(Self::init_synthetic_vec3_prop("RotationPivot", Vec3::ZERO, PropType::Vector));
                    new_props.push(Self::init_synthetic_vec3_prop("ScalingPivot", Vec3::ZERO, PropType::Vector));
                    new_props.push(Self::init_synthetic_vec3_prop("GeometricTranslation", geometric_translation, PropType::Vector));
                } else {
                    // The translation after the rotation moves to the geometry and the children: with Z the scale as
                    // the file has it, (Rp-1+Soff+Sp) + S*(Sp-1) = scaled_offset + S*unscaled_offset, which leaves no
                    // translation in the parent while S equals Z
                    let initial_scale = props.vec3("Lcl Scaling", Vec3::ONE);
                    let scaled_offset = sub3(add3(scaling_offset, scaling_pivot), rotation_pivot);
                    let unscaled_offset = Vec3::new(
                        pivot_div(scaled_offset.x, initial_scale.x),
                        pivot_div(scaled_offset.y, initial_scale.y),
                        pivot_div(scaled_offset.z, initial_scale.z),
                    );
                    let new_scaling_pivot = unscaled_offset;
                    let new_scaling_offset = sub3(scaled_offset, new_scaling_pivot);
                    child_offset = sub3(unscaled_offset, scaling_pivot);
                    new_props.push(Self::init_synthetic_vec3_prop("RotationPivot", Vec3::ZERO, PropType::Vector));
                    new_props.push(Self::init_synthetic_vec3_prop("ScalingPivot", new_scaling_pivot, PropType::Vector));
                    new_props.push(Self::init_synthetic_vec3_prop("ScalingOffset", new_scaling_offset, PropType::Vector));
                    if !skip_geometry_transform {
                        geometric_translation = add3(geometric_translation, child_offset);
                        new_props.push(Self::init_synthetic_vec3_prop("GeometricTranslation", geometric_translation, PropType::Vector));
                    }
                }
                sort_properties(&mut new_props);
                deduplicate_properties(&mut new_props);
                let node = &mut self.scene.nodes[node_ix];
                node.element.props.props = new_props;
                node.adjust_pre_translation = add3(node.adjust_pre_translation, rotation_pivot);
                node.has_adjust_transform = true;
                let mut ix = pn.first_child;
                while ix != NONE {
                    let pc = pre_nodes[ix as usize];
                    let ci = self.scene.elements[pc.element_id as usize].index;
                    let child = &mut self.scene.nodes[ci];
                    child.adjust_pre_translation = add3(child.adjust_pre_translation, child_offset);
                    child.has_adjust_transform = true;
                    ix = pc.next_child;
                }
            }
        }

        for i in 0..num_elements {
            let r = self.scene.elements[i];
            if r.element_type != ElementType::Node {
                continue;
            }
            let requires_helper = match self.opts.geometry_transform_handling {
                GeometryTransformHandling::HelperNodes => true,
                GeometryTransformHandling::ModifyGeometry => instance_counts[i] > 1 || modify_not_supported[i],
                _ => false,
            };
            if requires_helper {
                self.setup_geometry_transform_helper(r.index, fbx_ids[i])?;
            }
        }

        for i in 0..num_elements {
            let r = self.scene.elements[i];
            if r.element_type != ElementType::Node {
                continue;
            }
            let node_ix = r.index;
            if !(has_unscaled_children[node_ix] && self.scene.nodes[node_ix].scale_helper.is_none()) {
                continue;
            }
            let pn = pre_nodes[node_ix];
            let reference = if self.opts.inherit_mode_handling == InheritModeHandling::Compensate { pn.constant_scale.x } else { 1.0 };
            let scale = pn.constant_scale;
            let dx = (scale.x - reference).abs();
            let dy = (scale.y - reference).abs();
            let dz = (scale.z - reference).abs();
            if (dx + dy + dz >= scale_epsilon || !pn.has_constant_scale || scale.x.abs() <= compensate_epsilon)
                && self.opts.inherit_mode_handling != InheritModeHandling::CompensateNoFallback
            {
                self.setup_scale_helper(node_ix, fbx_ids[i])?;
                // Children that inherit componentwise get helpers of their own, down the whole subtree
                let mut ix = pn.first_child;
                while ix != NONE && ix != node_ix as u32 {
                    let mut pc_ix = ix as usize;
                    let pc = pre_nodes[pc_ix];
                    let child_ix = self.scene.elements[pc.element_id as usize].index;
                    let child_mode = self.scene.nodes[child_ix].original_inherit_mode;
                    if (pc.parent != node_ix as u32 || child_mode == InheritMode::ComponentwiseScale)
                        && !pc.has_recursive_scale_helper
                        && child_mode != InheritMode::Normal
                    {
                        pre_nodes[pc_ix].has_recursive_scale_helper = true;
                        let child_fbx_id = fbx_ids[pc.element_id as usize];
                        self.setup_scale_helper(child_ix, child_fbx_id)?;
                        self.scene.nodes[child_ix].is_scale_compensate_parent = false;
                        if pc.first_child != NONE {
                            ix = pc.first_child;
                            continue;
                        }
                    }
                    while pre_nodes[pc_ix].next_child == NONE {
                        ix = pre_nodes[pc_ix].parent;
                        if ix == node_ix as u32 {
                            break;
                        }
                        pc_ix = ix as usize;
                    }
                    if ix != node_ix as u32 {
                        ix = pre_nodes[pc_ix].next_child;
                    }
                }
            } else if matches!(self.opts.inherit_mode_handling, InheritModeHandling::Compensate | InheritModeHandling::CompensateNoFallback)
                && (scale.x - 1.0).abs() >= scale_epsilon
            {
                self.scene.nodes[node_ix].is_scale_compensate_parent = true;
            }
        }
        Ok(())
    }
}

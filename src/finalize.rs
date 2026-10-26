use std::sync::Arc;

use crate::context::Context;
use crate::error::{Res, WarningType, Warnings, check};
use crate::math::*;
use crate::props::name_key;
use crate::read::{is_node_property_name, pack_version};
use crate::search::{lower_bound_eq, stable_sort, upper_bound_eq};
use crate::types::*;

pub(crate) fn type_of(scene: &Scene, id: ElementId) -> ElementType {
    scene.elements[id as usize].element_type
}

pub(crate) fn typed_index(scene: &Scene, id: ElementId) -> usize {
    scene.elements[id as usize].index
}

/// `""` as `prop` means the element itself
pub(crate) fn find_dst_connections<'a>(element: &'a Element, prop: &str) -> &'a [Connection] {
    let c = &element.connections_dst;
    let n = c.len();
    let Some(begin) = lower_bound_eq(0, n, 32, |i| c[i].dst_prop.as_bytes() < prop.as_bytes(), |i| c[i].dst_prop == prop && c[i].src_prop.is_empty()) else {
        return &[];
    };
    let end = upper_bound_eq(begin, n, 32, |i| c[i].dst_prop == prop && c[i].src_prop.is_empty());
    &c[begin..end]
}

pub(crate) fn find_src_connections<'a>(element: &'a Element, prop: &str) -> &'a [Connection] {
    let c = &element.connections_src;
    let n = c.len();
    let Some(begin) = lower_bound_eq(0, n, 32, |i| c[i].src_prop.as_bytes() < prop.as_bytes(), |i| c[i].src_prop == prop && c[i].dst_prop.is_empty()) else {
        return &[];
    };
    let end = upper_bound_eq(begin, n, 32, |i| c[i].src_prop == prop && c[i].dst_prop.is_empty());
    &c[begin..end]
}

pub(crate) fn find_prop_connection<'a>(element: &'a Element, prop: &str) -> Option<&'a Connection> {
    let c = &element.connections_dst;
    // `ufbxi_prop_connection_less` in ufbx.c
    let less = |i: usize| match c[i].dst_prop.as_bytes().cmp(prop.as_bytes()) {
        std::cmp::Ordering::Equal => c[i].src_prop.is_empty(),
        o => o == std::cmp::Ordering::Less,
    };
    lower_bound_eq(0, c.len(), 32, less, |i| c[i].dst_prop == prop && !c[i].src_prop.is_empty()).map(|i| &c[i])
}

fn get_element_node(scene: &Scene, id: ElementId) -> Option<ElementId> {
    let r = scene.elements[id as usize];
    if r.element_type == ElementType::Node {
        let node = &scene.nodes[r.index];
        if node.is_geometry_transform_helper {
            return node.parent.map(|p| scene.nodes[p].element.element_id);
        }
        None
    } else {
        scene.element_at(r).instances.first().map(|&n| scene.nodes[n].element.element_id)
    }
}

pub(crate) struct Fetch<'a> {
    pub scene: &'a Scene,
    pub warnings: &'a mut Warnings,
    pub flags: &'a mut Vec<bool>,
}

impl Fetch<'_> {
    pub fn fetch_dst_elements(
        &mut self,
        mut element: ElementId,
        search_node: bool,
        ignore_duplicates: bool,
        prop: &str,
        src_type: ElementType,
    ) -> Vec<ElementId> {
        let scene = self.scene;
        let mut out = Vec::new();
        loop {
            for conn in find_dst_connections(scene.element(element), prop) {
                if type_of(scene, conn.src) != src_type {
                    continue;
                }
                if ignore_duplicates {
                    let id = conn.src as usize;
                    if self.flags[id] {
                        let owner = scene.element(element).element_id;
                        self.warnings.warn(WarningType::DuplicateConnection, conn.src, format!("Duplicate connection to {owner}"));
                        continue;
                    }
                    self.flags[id] = true;
                }
                out.push(conn.src);
            }
            if !search_node {
                break;
            }
            match get_element_node(scene, element) {
                Some(e) => element = e,
                None => break,
            }
        }
        if ignore_duplicates {
            for &e in &out {
                self.flags[e as usize] = false;
            }
        }
        out
    }

    pub fn fetch_src_elements(
        &mut self,
        mut element: ElementId,
        search_node: bool,
        ignore_duplicates: bool,
        prop: &str,
        dst_type: ElementType,
    ) -> Vec<ElementId> {
        let scene = self.scene;
        let mut out = Vec::new();
        loop {
            for conn in find_src_connections(scene.element(element), prop) {
                if type_of(scene, conn.dst) != dst_type {
                    continue;
                }
                if ignore_duplicates {
                    let id = conn.dst as usize;
                    if self.flags[id] {
                        let owner = scene.element(element).element_id;
                        self.warnings.warn(WarningType::DuplicateConnection, conn.dst, format!("Duplicate connection to {owner}"));
                        continue;
                    }
                    self.flags[id] = true;
                }
                out.push(conn.dst);
            }
            if !search_node {
                break;
            }
            match get_element_node(scene, element) {
                Some(e) => element = e,
                None => break,
            }
        }
        if ignore_duplicates {
            for &e in &out {
                self.flags[e as usize] = false;
            }
        }
        out
    }
}

pub(crate) fn fetch_dst_element(scene: &Scene, mut element: ElementId, search_node: bool, prop: &str, src_type: ElementType) -> Option<ElementId> {
    loop {
        for conn in find_dst_connections(scene.element(element), prop) {
            if type_of(scene, conn.src) == src_type {
                return Some(conn.src);
            }
        }
        if !search_node {
            return None;
        }
        element = get_element_node(scene, element)?;
    }
}

pub(crate) fn fetch_src_element(scene: &Scene, mut element: ElementId, search_node: bool, prop: &str, dst_type: ElementType) -> Option<ElementId> {
    loop {
        for conn in find_src_connections(scene.element(element), prop) {
            if type_of(scene, conn.dst) == dst_type {
                return Some(conn.dst);
            }
        }
        if !search_node {
            return None;
        }
        element = get_element_node(scene, element)?;
    }
}

fn typed(scene: &Scene, ids: &[ElementId]) -> Vec<usize> {
    ids.iter().map(|&id| typed_index(scene, id)).collect()
}

fn typed_opt(scene: &Scene, id: Option<ElementId>) -> Option<usize> {
    id.map(|id| typed_index(scene, id))
}

fn fetch_textures(scene: &Scene, mut element: ElementId, search_node: bool) -> Vec<MaterialTexture> {
    let mut out = Vec::new();
    loop {
        for conn in &scene.element(element).connections_dst {
            if !conn.src_prop.is_empty() {
                continue;
            }
            if type_of(scene, conn.src) == ElementType::Texture {
                out.push(MaterialTexture { material_prop: conn.dst_prop.clone(), shader_prop: conn.dst_prop.clone(), texture: typed_index(scene, conn.src) });
            }
        }
        if !search_node {
            break;
        }
        match get_element_node(scene, element) {
            Some(e) => element = e,
            None => break,
        }
    }
    out
}

fn fetch_mesh_materials(scene: &Scene, mut element: ElementId, search_node: bool) -> Vec<Option<usize>> {
    let mut out = Vec::new();
    loop {
        for conn in find_dst_connections(scene.element(element), "") {
            if type_of(scene, conn.src) == ElementType::Material {
                out.push(Some(typed_index(scene, conn.src)));
            }
        }
        if !out.is_empty() || !search_node {
            break;
        }
        match get_element_node(scene, element) {
            Some(e) => element = e,
            None => break,
        }
    }
    out
}

fn fetch_deformers(scene: &Scene, mut element: ElementId, search_node: bool) -> Vec<ElementId> {
    let mut out = Vec::new();
    loop {
        for conn in &scene.element(element).connections_dst {
            if !conn.src_prop.is_empty() {
                continue;
            }
            if matches!(type_of(scene, conn.src), ElementType::SkinDeformer | ElementType::BlendDeformer | ElementType::CacheDeformer) {
                out.push(conn.src);
            }
        }
        if !search_node {
            break;
        }
        match get_element_node(scene, element) {
            Some(e) => element = e,
            None => break,
        }
    }
    out
}

fn fetch_blend_keyframes(scene: &Scene, element: ElementId) -> Vec<BlendKeyframe> {
    find_dst_connections(scene.element(element), "")
        .iter()
        .filter(|c| type_of(scene, c.src) == ElementType::BlendShape)
        .map(|c| BlendKeyframe { shape: typed_index(scene, c.src), ..Default::default() })
        .collect()
}

pub(crate) fn fetch_texture_layers(scene: &Scene, element: ElementId) -> Vec<TextureLayer> {
    let mut out = Vec::new();
    for conn in find_dst_connections(scene.element(element), "") {
        if type_of(scene, conn.src) != ElementType::Texture {
            continue;
        }
        let texture = typed_index(scene, conn.src);
        let props = &scene.textures[texture].element.props;
        let alpha = props.real("Texture alpha", 1.0);
        let blend_mode =
            BlendMode::from_index(props.find_enum("BlendMode", BlendMode::Replace as i64, BlendMode::Overlay as i64)).unwrap_or(BlendMode::Replace);
        out.push(TextureLayer { texture, blend_mode, alpha });
    }
    out
}

/// `index` 0 sorts by source, 1 by destination
fn cmp_connection_less(a: &Connection, b: &Connection, index: usize) -> bool {
    let (ae, be) = if index == 0 { (a.src, b.src) } else { (a.dst, b.dst) };
    if ae != be {
        return ae < be;
    }
    let (ap, bp, aq, bq) = if index == 0 { (&a.src_prop, &b.src_prop, &a.dst_prop, &b.dst_prop) } else { (&a.dst_prop, &b.dst_prop, &a.src_prop, &b.src_prop) };
    match ap.as_bytes().cmp(bp.as_bytes()) {
        std::cmp::Ordering::Equal => aq.as_bytes() < bq.as_bytes(),
        o => o == std::cmp::Ordering::Less,
    }
}

impl Context<'_> {
    fn find_element_by_fbx_id(&self, fbx_id: u64) -> Option<ElementId> {
        self.find_fbx_id(fbx_id).map(|e| e.element_id)
    }

    fn find_attribute_fbx_id(&self, node_fbx_id: u64) -> u64 {
        self.fbx_attr_map.get(&node_fbx_id).copied().unwrap_or(node_fbx_id)
    }

    fn resolve_connections(&mut self) -> Res<()> {
        let mut tmp = std::mem::take(&mut self.tmp_connections);
        let version = self.version();

        if version > 0 && version < 7000 {
            for c in &mut tmp {
                if !c.src_prop.is_empty() && !is_node_property_name(&c.src_prop) {
                    let src = self.find_element_by_fbx_id(c.src);
                    if src.is_none_or(|s| self.scene.element(s).props.find(&c.src_prop).is_none()) {
                        c.src = self.find_attribute_fbx_id(c.src);
                    }
                }
                if !c.dst_prop.is_empty() && !is_node_property_name(&c.dst_prop) {
                    let dst = self.find_element_by_fbx_id(c.dst);
                    if dst.is_none_or(|d| self.scene.element(d).props.find(&c.dst_prop).is_none()) {
                        c.dst = self.find_attribute_fbx_id(c.dst);
                    }
                }
            }
        }

        let mut conns = Vec::with_capacity(tmp.len());
        for c in tmp {
            let (Some(mut src), Some(mut dst)) = (self.find_element_by_fbx_id(c.src), self.find_element_by_fbx_id(c.dst)) else { continue };
            let src_type = type_of(&self.scene, src);
            let dst_type = type_of(&self.scene, dst);

            if !self.opts.disable_quirks
                && dst_type == ElementType::Node
                && src_type != ElementType::Node
                && self.scene.nodes[typed_index(&self.scene, dst)].is_root
            {
                self.warnf_tag(WarningType::BadElementConnectedToRoot, src, "Non-node element connected to root");
                continue;
            }

            if self.has_geometry_transform_nodes && dst_type == ElementType::Node && src_type.is_attrib() {
                let node = &self.scene.nodes[typed_index(&self.scene, dst)];
                if node.has_geometry_transform {
                    let helper = self.get_element_extra(node.element.element_id).and_then(|e| e.geometry_helper_id);
                    check!(helper.is_some());
                    dst = helper.unwrap_or(dst);
                }
            }

            if self.has_scale_helper_nodes {
                let dst_type = type_of(&self.scene, dst);
                if dst_type == ElementType::Node {
                    let dst_node = &self.scene.nodes[typed_index(&self.scene, dst)];
                    if let Some(helper) = dst_node.scale_helper {
                        let helper_id = self.scene.nodes[helper].element.element_id;
                        let src_type = type_of(&self.scene, src);
                        if src_type == ElementType::Node {
                            let src_node = &self.scene.nodes[typed_index(&self.scene, src)];
                            if !src_node.is_scale_helper && src_node.original_inherit_mode == InheritMode::Normal {
                                dst = helper_id;
                            }
                        } else if src_type == ElementType::AnimValue {
                            if c.dst_prop == "Lcl Scaling" {
                                dst = helper_id;
                            }
                        } else {
                            dst = helper_id;
                        }
                    }
                } else if type_of(&self.scene, src) == ElementType::Node {
                    let src_node = &self.scene.nodes[typed_index(&self.scene, src)];
                    if let Some(helper) = src_node.scale_helper
                        && dst_type == ElementType::SkinCluster
                    {
                        src = self.scene.nodes[helper].element.element_id;
                    }
                }
            }

            if version > 0 && version < 7000 && type_of(&self.scene, dst) == ElementType::Node {
                let src_type = type_of(&self.scene, src);
                if (src_type == ElementType::SkinDeformer || src_type == ElementType::CacheDeformer)
                    && let Some(d) = self.find_element_by_fbx_id(self.find_attribute_fbx_id(c.dst))
                {
                    dst = d;
                }
            }

            conns.push(Connection { src, dst, src_prop: c.src_prop, dst_prop: c.dst_prop });
        }

        let mut by_dst = conns.clone();
        // `ufbxi_sort_connections` in ufbx.c
        stable_sort(&mut conns, 32, |a, b| cmp_connection_less(a, b, 0));
        stable_sort(&mut by_dst, 32, |a, b| cmp_connection_less(a, b, 1));
        self.scene.connections_src = conns;
        self.scene.connections_dst = by_dst;
        Ok(())
    }

    fn add_connections_to_elements(&mut self) {
        let src_all = std::mem::take(&mut self.scene.connections_src);
        let dst_all = std::mem::take(&mut self.scene.connections_dst);
        let version = self.version();
        let (mut cs, mut cd) = (0usize, 0usize);

        for id in 0..self.scene.elements.len() as u32 {
            while cs < src_all.len() && src_all[cs].src < id {
                cs += 1;
            }
            while cd < dst_all.len() && dst_all[cd].dst < id {
                cd += 1;
            }
            let (mut src_end, mut dst_end) = (cs, cd);
            while src_end < src_all.len() && src_all[src_end].src == id {
                src_end += 1;
            }
            while dst_end < dst_all.len() && dst_all[dst_end].dst == id {
                dst_end += 1;
            }

            let mut new_props: Vec<Prop> = Vec::new();
            let mut needs_copy = false;
            let mut num_animated = 0;
            {
                let scene = &self.scene;
                let elem = scene.element(id);
                let props = &elem.props.props;
                let mut prop = 0usize;
                let mut conn = cd;
                loop {
                    while conn < dst_end {
                        let c = &dst_all[conn];
                        if c.dst_prop.is_empty() {
                            conn += 1;
                            continue;
                        }
                        if !c.src_prop.is_empty() || type_of(scene, c.src) == ElementType::AnimValue {
                            break;
                        }
                        conn += 1;
                    }
                    let name = if conn < dst_end { dst_all[conn].dst_prop.clone() } else { String::new() };
                    if name.is_empty() {
                        break;
                    }
                    num_animated += 1;

                    let mut anim_value: Option<usize> = None;
                    let mut flags = 0u32;
                    while conn < dst_end && dst_all[conn].dst_prop == name {
                        let c = &dst_all[conn];
                        if !c.src_prop.is_empty() {
                            flags |= prop_flags::CONNECTED;
                        } else if type_of(scene, c.src) == ElementType::AnimValue {
                            anim_value = Some(typed_index(scene, c.src));
                            flags |= prop_flags::ANIMATED;
                        }
                        conn += 1;
                    }

                    let key = name_key(name.as_bytes());
                    // `ufbxi_name_key_less` in ufbx.c
                    while prop < props.len() && (props[prop].internal_key, props[prop].name.as_bytes()) < (key, name.as_bytes()) {
                        prop += 1;
                    }

                    if prop < props.len() && props[prop].name == name {
                        if !needs_copy {
                            new_props = props.clone();
                        }
                        // flags land on the copy at the same position: synthetic ones are inserted before it
                        let shift = new_props.len() - props.len();
                        new_props[prop + shift].flags |= flags;
                        needs_copy = true;
                    } else {
                        if !needs_copy {
                            new_props = props.clone();
                        }
                        needs_copy = true;
                        let def_prop: Option<Prop> = if let Some(defaults) = &elem.props.defaults {
                            defaults.find_prop(&name).cloned()
                        } else if let Some(av) = anim_value {
                            let mut p = Prop::default();
                            let mut t = PropType::Unknown;
                            if name == "Lcl Translation" {
                                t = PropType::Translation;
                            } else if name == "Lcl Rotation" {
                                t = PropType::Rotation;
                            } else if name == "Lcl Scaling" {
                                t = PropType::Scaling;
                                p.value_real_arr[0] = 1.0;
                                p.value_real_arr[1] = 1.0;
                                p.value_real_arr[2] = 1.0;
                            }
                            if version < 6000 {
                                let d = scene.anim_values[av].default_value;
                                p.value_real_arr[0] = d.x;
                                p.value_real_arr[1] = d.y;
                                p.value_real_arr[2] = d.z;
                            }
                            p.prop_type = t;
                            Some(p)
                        } else {
                            flags |= prop_flags::NO_VALUE;
                            None
                        };
                        let mut new_prop = def_prop.unwrap_or_default();
                        flags |= new_prop.flags;
                        new_prop.flags = prop_flags::ANIMATABLE | prop_flags::SYNTHETIC | flags;
                        new_prop.name = name;
                        new_prop.internal_key = key;
                        new_prop.value_str = String::new();
                        new_prop.value_blob = Vec::new();
                        let shift = new_props.len() - props.len();
                        new_props.insert(prop + shift, new_prop);
                    }
                }
            }

            let elem = self.scene.element_mut(id);
            elem.connections_src = src_all[cs..src_end].to_vec();
            elem.connections_dst = dst_all[cd..dst_end].to_vec();
            if needs_copy {
                elem.props.props = new_props;
            }
            elem.props.num_animated = num_animated;

            cs = src_end;
            cd = dst_end;
        }

        self.scene.connections_src = src_all;
        self.scene.connections_dst = dst_all;
    }

    fn linearize_nodes(&mut self) -> Res<()> {
        let node_ids = std::mem::take(&mut self.tmp_node_ids);
        let num_nodes = node_ids.len();
        check!(num_nodes > 0);
        let nodes: Vec<usize> = node_ids.iter().map(|&id| typed_index(&self.scene, id)).collect();
        let root = nodes[0];
        let version = self.version();

        for &node in &nodes {
            if self.scene.nodes[node].parent.is_none() && !(self.opts.allow_nodes_out_of_root && version >= 6000) && node != root {
                self.scene.nodes[node].parent = Some(root);
            }
            let children: Vec<usize> = self.scene.nodes[node]
                .element
                .connections_dst
                .iter()
                .filter(|c| c.src_prop.is_empty() && c.dst_prop.is_empty() && type_of(&self.scene, c.src) == ElementType::Node)
                .map(|c| typed_index(&self.scene, c.src))
                .collect();
            for child in children {
                self.scene.nodes[child].parent = Some(node);
            }
        }

        for &node in &nodes {
            let mut depth: u32 = 0;
            let mut p = self.scene.nodes[node].parent;
            while let Some(pi) = p {
                let pd = self.scene.nodes[pi].node_depth;
                depth = depth.wrapping_add(pd + 1);
                if pd > 0 {
                    break;
                }
                check!(depth as usize <= num_nodes, "Cyclic node hierarchy");
                p = self.scene.nodes[pi].parent;
            }
            if self.opts.node_depth_limit > 0 {
                check!(depth <= self.opts.node_depth_limit, "Node depth limit exceeded");
            }
            self.scene.nodes[node].node_depth = depth;

            let mut p = self.scene.nodes[node].parent;
            while let Some(pi) = p {
                depth -= 1;
                if depth <= self.scene.nodes[pi].node_depth {
                    break;
                }
                self.scene.nodes[pi].node_depth = depth;
                p = self.scene.nodes[pi].parent;
            }
        }

        let mut order = nodes.clone();
        {
            let n = &self.scene.nodes;
            // `ufbxi_cmp_node_less` and `ufbxi_sort_node_ptrs` in ufbx.c
            stable_sort(&mut order, 32, |&a, &b| {
                let (a, b) = (&n[a], &n[b]);
                if a.node_depth != b.node_depth {
                    return a.node_depth < b.node_depth;
                }
                if let (Some(ap), Some(bp)) = (a.parent, b.parent) {
                    let (ai, bi) = (n[ap].element.element_id, n[bp].element.element_id);
                    if ai != bi {
                        return ai < bi;
                    }
                }
                if a.is_geometry_transform_helper != b.is_geometry_transform_helper {
                    return a.is_geometry_transform_helper;
                }
                if a.is_scale_helper != b.is_scale_helper {
                    return a.is_scale_helper;
                }
                a.element.element_id < b.element.element_id
            });
        }

        // `order[new] = old`; nodes not reached through `tmp_node_ids` keep their places after the ordered ones
        let total = self.scene.nodes.len();
        let mut new_of_old = vec![usize::MAX; total];
        for (new, &old) in order.iter().enumerate() {
            new_of_old[old] = new;
        }
        let mut next = order.len();
        for slot in new_of_old.iter_mut() {
            if *slot == usize::MAX {
                *slot = next;
                next += 1;
            }
        }
        let mut old_nodes: Vec<Option<Node>> = std::mem::take(&mut self.scene.nodes).into_iter().map(Some).collect();
        let mut new_nodes: Vec<Node> = Vec::with_capacity(total);
        let mut old_of_new = vec![0usize; total];
        for (old, &new) in new_of_old.iter().enumerate() {
            old_of_new[new] = old;
        }
        for new in 0..total {
            let mut node = old_nodes[old_of_new[new]].take().unwrap_or_default();
            node.element.typed_id = new as u32;
            let remap = |i: Option<usize>| i.map(|i| new_of_old[i]);
            node.parent = remap(node.parent);
            node.scale_helper = remap(node.scale_helper);
            node.geometry_transform_helper = remap(node.geometry_transform_helper);
            node.inherit_scale_node = remap(node.inherit_scale_node);
            self.scene.elements[node.element.element_id as usize].index = new;
            new_nodes.push(node);
        }
        self.scene.nodes = new_nodes;
        self.scene.root_node = new_of_old[root];
        Ok(())
    }
}

impl Context<'_> {
    fn fetch(&mut self) -> (Fetch<'_>, &LoadOpts) {
        (Fetch { scene: &self.scene, warnings: &mut self.p.warnings, flags: &mut self.tmp_element_flag }, &self.opts)
    }

    pub(crate) fn finalize_scene(&mut self) -> Res<()> {
        let num_elements = self.scene.elements.len();
        self.tmp_element_flag = vec![false; num_elements];

        let scene_props = &self.scene.metadata.scene_props;
        self.scene.metadata.original_file_path = scene_props.find_string("DocumentUrl", "").to_string();
        self.scene.metadata.raw_original_file_path = scene_props.find_blob("DocumentUrl", b"").to_vec();

        self.resolve_connections()?;
        self.add_connections_to_elements();
        self.linearize_nodes()?;

        let mut by_name: Vec<NameElement> = (0..num_elements as u32)
            .map(|id| {
                let e = self.scene.element(id);
                NameElement { name: e.name.clone(), element_type: e.element_type, internal_key: name_key(e.name.as_bytes()), element: id }
            })
            .collect();
        // `ufbxi_cmp_name_element_less` and `ufbxi_sort_name_elements` in ufbx.c
        stable_sort(&mut by_name, 32, |a, b| {
            if a.internal_key != b.internal_key {
                return a.internal_key < b.internal_key;
            }
            match a.name.as_bytes().cmp(b.name.as_bytes()) {
                std::cmp::Ordering::Equal => a.element_type < b.element_type,
                o => o == std::cmp::Ordering::Less,
            }
        });
        self.scene.elements_by_name = by_name;

        self.finalize_nodes()?;
        self.finalize_poses();
        self.finalize_instances();
        let search_node = self.version() < 7000;
        self.finalize_skins()?;
        self.finalize_deformers()?;
        self.finalize_blend_channels();
        self.finalize_meshes(search_node)?;
        self.finalize_curves_and_cameras(search_node)?;
        self.finalize_anim();
        self.finalize_shaders_and_materials();
        self.finalize_legacy_mesh_textures();
        self.resolve_file_content();
        let files = self.finalize_textures();
        crate::textures::propagate_main_textures(&mut self.scene);
        // `ufbxi_pop_texture_files` in ufbx.c
        self.scene.texture_files = files;
        self.finalize_material_maps();
        self.finalize_collections()?;
        self.fetch_file_textures();

        if self.scene.anim_layers.is_empty() {
            // `ufbxi_push_anim` in ufbx.c
            self.scene.anim = Arc::new(Anim::default());
        }

        self.scene.metadata.ktime_second = self.ktime_sec;
        self.scene.metadata.bone_prop_size_unit = if self.version() < 6000 {
            1.0
        } else if self.exporter == Exporter::BlenderBinary {
            33.0
        } else if self.exporter == Exporter::BlenderAscii {
            1.0
        } else {
            100.0 / 3.0
        };
        self.scene.metadata.bone_prop_limb_length_relative = self.exporter != Exporter::BlenderAscii;
        Ok(())
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_nodes(&mut self) -> Res<()> {
        for i in 0..self.scene.nodes.len() {
            if let Some(parent) = self.scene.nodes[i].parent {
                self.scene.nodes[parent].children.push(i);
                if self.scene.nodes[i].is_geometry_transform_helper {
                    self.scene.nodes[parent].geometry_transform_helper = Some(i);
                }
                if self.scene.nodes[parent].is_root
                    && self.opts.space_conversion == SpaceConversion::TransformRoot
                    && self.opts.inherit_mode_handling == InheritModeHandling::Preserve
                {
                    self.scene.nodes[i].original_inherit_mode = InheritMode::Normal;
                    self.scene.nodes[i].inherit_mode = InheritMode::Normal;
                }
                match self.scene.nodes[i].original_inherit_mode {
                    InheritMode::ComponentwiseScale => self.scene.nodes[i].inherit_scale_node = Some(parent),
                    InheritMode::IgnoreParentScale => self.scene.nodes[i].inherit_scale_node = self.scene.nodes[parent].inherit_scale_node,
                    _ => {}
                }
            }

            let attribs: Vec<ElementId> =
                find_dst_connections(&self.scene.nodes[i].element, "").iter().filter(|c| type_of(&self.scene, c.src).is_attrib()).map(|c| c.src).collect();
            for &a in &attribs {
                let r = self.scene.elements[a as usize];
                let node = &mut self.scene.nodes[i];
                if node.all_attribs.is_empty() {
                    node.attrib = Some(a);
                    node.attrib_type = r.element_type;
                }
                node.all_attribs.push(a);
                match r.element_type {
                    ElementType::Mesh => node.mesh = Some(r.index),
                    ElementType::Light => node.light = Some(r.index),
                    ElementType::Camera => node.camera = Some(r.index),
                    ElementType::Bone => node.bone = Some(r.index),
                    _ => {}
                }
            }

            let id = self.scene.nodes[i].element.element_id;
            let (mut f, _) = self.fetch();
            let materials = f.fetch_dst_elements(id, false, false, "", ElementType::Material);
            let materials = typed(&self.scene, &materials).into_iter().map(Some).collect();
            self.scene.nodes[i].materials = materials;
        }
        Ok(())
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_poses(&mut self) {
        for p in 0..self.scene.poses.len() {
            let pose_id = self.scene.poses[p].element.element_id;
            let tmp = self.extra.get_mut(&pose_id).map(|e| std::mem::take(&mut e.bone_poses)).unwrap_or_default();
            let is_bind_pose = self.scene.poses[p].is_bind_pose;
            let mut bones = Vec::new();
            for t in tmp {
                let Some(elem) = self.find_element_by_fbx_id(t.bone_fbx_id) else { continue };
                if type_of(&self.scene, elem) != ElementType::Node {
                    continue;
                }
                let node = typed_index(&self.scene, elem);
                bones.push(BonePose { bone_node: node, bone_to_world: t.bone_to_world, ..Default::default() });
                if is_bind_pose {
                    if self.scene.nodes[node].bind_pose.is_none() {
                        self.scene.nodes[node].bind_pose = Some(p);
                    }
                    let clusters: Vec<usize> = find_src_connections(self.scene.element(elem), "")
                        .iter()
                        .filter(|c| type_of(&self.scene, c.dst) == ElementType::SkinCluster)
                        .map(|c| typed_index(&self.scene, c.dst))
                        .collect();
                    for c in clusters {
                        let cluster = &mut self.scene.skin_clusters[c];
                        if matrix_all_zero(&cluster.bind_to_world) {
                            cluster.bind_to_world = t.bone_to_world;
                        }
                    }
                }
            }
            let nodes = &self.scene.nodes;
            // `ufbxi_bone_pose_less` and `ufbxi_sort_bone_poses` in ufbx.c
            stable_sort(&mut bones, 16, |a, b| nodes[a.bone_node].element.typed_id < nodes[b.bone_node].element.typed_id);
            self.scene.poses[p].bone_poses = bones;
        }
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_instances(&mut self) {
        for t in ALL_ELEMENT_TYPES {
            if !t.is_attrib() {
                continue;
            }
            for id in self.scene.elements_by_type(t) {
                let (mut f, _) = self.fetch();
                let nodes = f.fetch_src_elements(id, false, true, "", ElementType::Node);
                let nodes = typed(&self.scene, &nodes);
                self.scene.element_mut(id).instances = nodes;
            }
        }
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_skins(&mut self) -> Res<()> {
        for c in 0..self.scene.skin_clusters.len() {
            let id = self.scene.skin_clusters[c].element.element_id;
            let bone = fetch_dst_element(&self.scene, id, false, "", ElementType::Node);
            self.scene.skin_clusters[c].bone_node = typed_opt(&self.scene, bone);
        }

        for s in 0..self.scene.skin_deformers.len() {
            let id = self.scene.skin_deformers[s].element.element_id;
            let (mut f, _) = self.fetch();
            let clusters = f.fetch_dst_elements(id, false, true, "", ElementType::SkinCluster);
            let mut clusters = typed(&self.scene, &clusters);
            if !self.opts.connect_broken_elements {
                clusters.retain(|&c| self.scene.skin_clusters[c].bone_node.is_some());
            }

            let mut total_weights = 0usize;
            for &c in &clusters {
                let n = self.scene.skin_clusters[c].num_weights;
                check!(usize::MAX - total_weights > n);
                total_weights += n;
            }

            let mut num_vertices = 0usize;
            for conn in find_src_connections(self.scene.element(id), "") {
                if !conn.dst_prop.is_empty() {
                    continue;
                }
                let r = self.scene.elements[conn.dst as usize];
                let mesh = match r.element_type {
                    ElementType::Mesh => Some(r.index),
                    ElementType::Node => {
                        let mut node = r.index;
                        if let Some(h) = self.scene.nodes[node].geometry_transform_helper {
                            node = h;
                        }
                        self.scene.nodes[node].mesh
                    }
                    _ => None,
                };
                if let Some(m) = mesh {
                    num_vertices = num_vertices.max(self.scene.meshes[m].num_vertices);
                }
            }

            let skip = self.opts.skip_skin_vertices;
            let retain_all = !self.opts.clean_skin_weights;
            let scene = &mut self.scene;
            if !skip {
                let mut vertices = vec![SkinVertex::default(); num_vertices];
                let mut weights = vec![SkinWeight::default(); total_weights];
                for &c in &clusters {
                    let cluster = &scene.skin_clusters[c];
                    for i in 0..cluster.num_weights {
                        let v = cluster.vertices[i];
                        if (v as usize) < num_vertices && (retain_all || cluster.weights[i] > 0.0) {
                            vertices[v as usize].num_weights += 1;
                        }
                    }
                }
                let skin = &scene.skin_deformers[s];
                let default_dq = if skin.skinning_method == SkinningMethod::DualQuaternion { 1.0 } else { 0.0 };
                let mut offset = 0u32;
                let mut max_weights = 0u32;
                for v in vertices.iter_mut() {
                    v.weight_begin = offset;
                    v.dq_weight = default_dq;
                    let n = v.num_weights;
                    offset += n;
                    v.num_weights = 0;
                    max_weights = max_weights.max(n);
                }
                for i in 0..skin.num_dq_weights {
                    let v = skin.dq_vertices[i];
                    if (v as usize) < num_vertices {
                        vertices[v as usize].dq_weight = skin.dq_weights[i];
                    }
                }
                for (cluster_index, &c) in clusters.iter().enumerate() {
                    let cluster = &scene.skin_clusters[c];
                    for i in 0..cluster.num_weights {
                        let v = cluster.vertices[i] as usize;
                        if v < num_vertices && (retain_all || cluster.weights[i] > 0.0) {
                            let local = vertices[v].num_weights;
                            vertices[v].num_weights += 1;
                            let index = (vertices[v].weight_begin + local) as usize;
                            weights[index] = SkinWeight { cluster_index: cluster_index as u32, weight: cluster.weights[i] };
                        }
                    }
                }
                // `ufbxi_sort_skin_weights` in ufbx.c
                for v in &vertices {
                    let range = v.weight_begin as usize..(v.weight_begin + v.num_weights) as usize;
                    stable_sort(&mut weights[range], 32, |a, b| a.weight > b.weight);
                }
                let skin = &mut scene.skin_deformers[s];
                skin.max_weights_per_vertex = max_weights as usize;
                skin.vertices = vertices;
                skin.weights = weights;
            }
            scene.skin_deformers[s].clusters = clusters;
        }
        Ok(())
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_deformers(&mut self) -> Res<()> {
        for b in 0..self.scene.blend_deformers.len() {
            let id = self.scene.blend_deformers[b].element.element_id;
            let (mut f, _) = self.fetch();
            let channels = f.fetch_dst_elements(id, false, true, "", ElementType::BlendChannel);
            self.scene.blend_deformers[b].channels = typed(&self.scene, &channels);
        }

        for d in 0..self.scene.cache_deformers.len() {
            let id = self.scene.cache_deformers[d].element.element_id;
            let file = fetch_dst_element(&self.scene, id, false, "", ElementType::CacheFile);
            let file = typed_opt(&self.scene, file);
            let deformer = &mut self.scene.cache_deformers[d];
            deformer.channel = deformer.element.props.find_string("ChannelName", "").to_string();
            deformer.file = file;
        }

        for c in 0..self.scene.cache_files.len() {
            let cache = &mut self.scene.cache_files[c];
            let props = &cache.element.props;
            cache.absolute_filename = props.find_string("CacheAbsoluteFileName", "").to_string();
            cache.relative_filename = props.find_string("CacheFileName", "").to_string();
            cache.raw_absolute_filename = props.find_blob("CacheAbsoluteFileName", b"").to_vec();
            cache.raw_relative_filename = props.find_blob("CacheFileName", b"").to_vec();
            let t = props.find_int("CacheFileType", 0);
            match t {
                0 => cache.format = CacheFileFormat::Unknown,
                1 => cache.format = CacheFileFormat::Pc2,
                2 => cache.format = CacheFileFormat::Mc,
                _ => {}
            }
            let (f, a, r) = self.resolve_filenames_str(&self.scene.cache_files[c].absolute_filename, &self.scene.cache_files[c].relative_filename);
            let (rf, ra, rr) = self.resolve_filenames_raw(&self.scene.cache_files[c].raw_absolute_filename, &self.scene.cache_files[c].raw_relative_filename);
            let cache = &mut self.scene.cache_files[c];
            (cache.filename, cache.absolute_filename, cache.relative_filename) = (f, a, r);
            (cache.raw_filename, cache.raw_absolute_filename, cache.raw_relative_filename) = (rf, ra, rr);
        }
        Ok(())
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_blend_channels(&mut self) {
        let mut full_weights = std::mem::take(&mut self.tmp_full_weights);
        full_weights.resize(self.scene.blend_channels.len(), Vec::new());
        let blender = self.p.blender_full_weights;
        for c in 0..self.scene.blend_channels.len() {
            let id = self.scene.blend_channels[c].element.element_id;
            let mut keys = fetch_blend_keyframes(&self.scene, id);
            let fw = &mut full_weights[c];
            for i in 0..keys.len() {
                keys[i].target_weight = 1.0;
                if i < fw.len() {
                    if !blender {
                        keys[i].target_weight = fw[i] / 100.0;
                    } else if fw.len() == self.scene.blend_shapes[keys[i].shape].num_offsets {
                        if i == 0 {
                            for w in fw.iter_mut() {
                                *w /= 100.0;
                            }
                        }
                        self.scene.blend_shapes[keys[i].shape].offset_weights = fw.clone();
                    }
                }
            }
            // `ufbxi_blend_keyframe_less` and `ufbxi_sort_blend_keyframes` in ufbx.c
            stable_sort(&mut keys, 32, |a, b| a.target_weight < b.target_weight);
            let channel = &mut self.scene.blend_channels[c];
            channel.target_shape = keys.last().map(|k| k.shape);
            channel.keyframes = keys;
        }
    }
}

impl Context<'_> {
    fn generate_normals(mesh: &mut Mesh) {
        let num_indices = mesh.num_indices;
        mesh.generated_normals = true;
        let mut topo = vec![TopoEdge::default(); num_indices];
        let mut normal_indices = vec![0u32; num_indices];
        crate::topology::compute_topology(mesh, &mut topo);
        let num_normals = crate::topology::generate_normal_mapping(mesh, &topo, &mut normal_indices, false);
        if num_normals == mesh.num_vertices {
            mesh.vertex_normal.unique_per_vertex = true;
        }
        let mut normals = vec![Vec3::ZERO; num_normals];
        crate::topology::compute_normals(mesh, &mesh.vertex_position, &normal_indices, &mut normals);
        mesh.vertex_normal.exists = true;
        mesh.vertex_normal.values = normals;
        mesh.vertex_normal.indices = normal_indices;
        mesh.vertex_normal.value_reals = 3;
        mesh.skinned_normal = mesh.vertex_normal.clone();
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_meshes(&mut self, search_node: bool) -> Res<()> {
        for m in 0..self.scene.meshes.len() {
            if !self.scene.meshes[m].vertex_normal.exists && self.opts.generate_missing_normals {
                Self::generate_normals(&mut self.scene.meshes[m]);
            }
            {
                let mesh = &mut self.scene.meshes[m];
                if let Some(set) = mesh.uv_sets.first() {
                    mesh.vertex_uv = set.vertex_uv.clone();
                    mesh.vertex_bitangent = set.vertex_bitangent.clone();
                    mesh.vertex_tangent = set.vertex_tangent.clone();
                }
                if let Some(set) = mesh.color_sets.first() {
                    mesh.vertex_color = set.vertex_color.clone();
                }
            }

            let id = self.scene.meshes[m].element.element_id;
            let materials = fetch_mesh_materials(&self.scene, id, true);
            if !materials.is_empty() {
                for &n in &self.scene.meshes[m].element.instances.clone() {
                    let node = &mut self.scene.nodes[n];
                    if node.materials.len() < materials.len() && materials[0].is_some() {
                        let have = node.materials.len();
                        node.materials.extend_from_slice(&materials[have..]);
                    }
                }
            }
            let retain_parts = self.retain_mesh_parts;
            {
                let mesh = &mut self.scene.meshes[m];
                mesh.materials = materials;
                if retain_parts {
                    mesh.material_parts = vec![MeshPart::default(); mesh.materials.len().max(1)];
                }
                if mesh.materials.len() <= 1 {
                    if !mesh.material_parts.is_empty() {
                        let (nf, nt, ne, np, nl) = (mesh.num_faces, mesh.num_triangles, mesh.num_empty_faces, mesh.num_point_faces, mesh.num_line_faces);
                        let part = &mut mesh.material_parts[0];
                        part.num_faces = nf;
                        part.num_triangles = nt;
                        part.num_empty_faces = ne;
                        part.num_point_faces = np;
                        part.num_line_faces = nl;
                        part.face_indices = (0..nf as u32).collect();
                        mesh.material_part_usage_order = vec![0];
                    }
                    mesh.face_material = if mesh.materials.len() == 1 { vec![0; mesh.num_faces] } else { Vec::new() };
                } else {
                    crate::mesh::finalize_mesh_material(mesh)?;
                }
            }

            let (mut f, _) = self.fetch();
            let skins = f.fetch_dst_elements(id, search_node, true, "", ElementType::SkinDeformer);
            let blends = f.fetch_dst_elements(id, search_node, true, "", ElementType::BlendDeformer);
            let caches = f.fetch_dst_elements(id, search_node, true, "", ElementType::CacheDeformer);
            let all = fetch_deformers(&self.scene, id, search_node);
            let (skins, blends, caches) = (typed(&self.scene, &skins), typed(&self.scene, &blends), typed(&self.scene, &caches));
            let allow_missing = self.opts.allow_missing_vertex_position;
            let mesh = &mut self.scene.meshes[m];
            mesh.skin_deformers = skins;
            mesh.blend_deformers = blends;
            mesh.cache_deformers = caches;
            mesh.all_deformers = all;

            if !mesh.vertex_position.exists && !allow_missing {
                check!(mesh.num_indices == 0);
                mesh.vertex_position.exists = true;
                mesh.vertex_position.unique_per_vertex = true;
                mesh.skinned_position.exists = true;
                mesh.skinned_position.unique_per_vertex = true;
            }
            let mft = mesh.max_face_triangles;
            if mft > self.scene.metadata.max_face_triangles {
                self.scene.metadata.max_face_triangles = mft;
            }
        }
        Ok(())
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_curves_and_cameras(&mut self, search_node: bool) -> Res<()> {
        for s in 0..self.scene.stereo_cameras.len() {
            let id = self.scene.stereo_cameras[s].element.element_id;
            let left = fetch_dst_element(&self.scene, id, search_node, "LeftCamera", ElementType::Camera);
            let right = fetch_dst_element(&self.scene, id, search_node, "RightCamera", ElementType::Camera);
            let (left, right) = (typed_opt(&self.scene, left), typed_opt(&self.scene, right));
            let stereo = &mut self.scene.stereo_cameras[s];
            stereo.left = left;
            stereo.right = right;
        }
        for c in &mut self.scene.nurbs_curves {
            finalize_nurbs_basis(&mut c.basis)?;
        }
        for s in 0..self.scene.nurbs_surfaces.len() {
            finalize_nurbs_basis(&mut self.scene.nurbs_surfaces[s].basis_u)?;
            finalize_nurbs_basis(&mut self.scene.nurbs_surfaces[s].basis_v)?;
            let id = self.scene.nurbs_surfaces[s].element.element_id;
            let material = fetch_dst_element(&self.scene, id, true, "", ElementType::Material);
            self.scene.nurbs_surfaces[s].material = typed_opt(&self.scene, material);
        }
        Ok(())
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_anim(&mut self) {
        for s in 0..self.scene.anim_stacks.len() {
            let id = self.scene.anim_stacks[s].element.element_id;
            let (mut f, _) = self.fetch();
            let layers = f.fetch_dst_elements(id, false, true, "", ElementType::AnimLayer);
            let layers = typed(&self.scene, &layers);
            let stack = &mut self.scene.anim_stacks[s];
            // `ufbxi_push_anim` in ufbx.c
            stack.anim = Arc::new(Anim { layers: layers.clone(), ..Default::default() });
            stack.layers = layers;
        }

        for l in 0..self.scene.anim_layers.len() {
            let id = self.scene.anim_layers[l].element.element_id;
            let (mut f, _) = self.fetch();
            let values = f.fetch_dst_elements(id, false, true, "", ElementType::AnimValue);
            let values = typed(&self.scene, &values);

            let mut min_id = u32::MAX;
            let mut max_id = 0u32;
            let mut bitmask = [0u32; 4];
            let mut props = Vec::new();
            for &v in &values {
                for ac in &self.scene.anim_values[v].element.connections_src {
                    if ac.src_prop.is_empty() && !ac.dst_prop.is_empty() {
                        let id = ac.dst;
                        min_id = min_id.min(id);
                        max_id = max_id.max(id);
                        bitmask[((id >> 5) & 3) as usize] |= 1u32 << (id & 31);
                        props.push(AnimProp { element: id, internal_key: name_key(ac.dst_prop.as_bytes()), prop_name: ac.dst_prop.clone(), anim_value: v });
                    }
                }
            }
            // `ufbxi_cmp_anim_prop_less` and `ufbxi_sort_anim_props` in ufbx.c
            stable_sort(&mut props, 32, |a, b| {
                if a.element != b.element {
                    return a.element < b.element;
                }
                if a.internal_key != b.internal_key {
                    return a.internal_key < b.internal_key;
                }
                a.prop_name.as_bytes() < b.prop_name.as_bytes()
            });

            let layer = &mut self.scene.anim_layers[l];
            layer.anim_values = values;
            // `ufbxi_push_anim` in ufbx.c
            layer.anim = Arc::new(Anim { layers: vec![l], ..Default::default() });
            layer.element_id_bitmask = bitmask;
            if min_id != u32::MAX {
                layer.min_element_id = min_id;
                layer.max_element_id = max_id;
            }
            let lp = &layer.element.props;
            (layer.blended, layer.additive) = match lp.int("BlendMode", 0) {
                0 => (true, true),
                1 => (false, false),
                2 => (true, false),
                _ => (false, false),
            };
            if let Some(w) = lp.find_prop("Weight") {
                let mut weight = w.value_real() / 100.0;
                if weight < 0.0 {
                    weight = 0.0;
                }
                if weight > 0.99999f32 as f64 {
                    weight = 1.0;
                }
                layer.weight = weight;
                layer.weight_is_animated = w.flags & prop_flags::ANIMATED != 0;
            } else {
                layer.weight = 1.0;
                layer.weight_is_animated = false;
            }
            layer.compose_rotation = lp.int("RotationAccumulationMode", 0) == 0;
            layer.compose_scale = lp.int("ScaleAccumulationMode", 0) == 0;
            layer.anim_props = props;
        }

        for v in 0..self.scene.anim_values.len() {
            let mut curves: Vec<(usize, usize, Option<Real>)> = Vec::new();
            {
                let value = &self.scene.anim_values[v];
                for conn in &value.element.connections_dst {
                    if type_of(&self.scene, conn.src) == ElementType::AnimCurve && conn.src_prop.is_empty() {
                        let name = conn.dst_prop.as_str();
                        let mut index = 0;
                        if name == "Y" || name == "d|Y" {
                            index = 1;
                        }
                        if name == "Z" || name == "d|Z" {
                            index = 2;
                        }
                        let def = value.element.props.find(name).map(|p| p.value_real());
                        curves.push((index, typed_index(&self.scene, conn.src), def));
                    }
                }
            }
            let value = &mut self.scene.anim_values[v];
            let p = &value.element.props;
            let mut d = value.default_value;
            d.x = p.real("X", d.x);
            d.x = p.real("d|X", d.x);
            d.y = p.real("Y", d.y);
            d.y = p.real("d|Y", d.y);
            d.z = p.real("Z", d.z);
            d.z = p.real("d|Z", d.z);
            for (index, curve, def) in curves {
                if let Some(def) = def {
                    match index {
                        0 => d.x = def,
                        1 => d.y = def,
                        _ => d.z = def,
                    }
                }
                value.curves[index] = Some(curve);
            }
            value.default_value = d;
        }

        let ktime_sec = self.ktime_sec;
        for curve in &mut self.scene.anim_curves {
            curve.ktime_second = ktime_sec;
            if let (Some(first), Some(last)) = (curve.keyframes.first(), curve.keyframes.last()) {
                curve.min_time = first.time;
                curve.max_time = last.time;
            }
        }
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_shaders_and_materials(&mut self) {
        for s in 0..self.scene.shaders.len() {
            let id = self.scene.shaders[s].element.element_id;
            let (mut f, _) = self.fetch();
            let bindings = f.fetch_dst_elements(id, false, false, "", ElementType::ShaderBinding);
            let bindings = typed(&self.scene, &bindings);
            let shader = &mut self.scene.shaders[s];
            shader.bindings = bindings;
            if let Some(api) = shader.element.props.find("RenderAPI") {
                match api.value_str.as_str() {
                    "ARNOLD_SHADER_ID" => shader.shader_type = ShaderType::ArnoldStandardSurface,
                    "OSL" => shader.shader_type = ShaderType::OslStandardSurface,
                    "SFX_PBS_SHADER" => shader.shader_type = ShaderType::ShaderfxGraph,
                    _ => {}
                }
            }
        }

        for m in 0..self.scene.materials.len() {
            let id = self.scene.materials[m].element.element_id;
            let shader = fetch_src_element(&self.scene, id, false, "", ElementType::Shader);
            let shader = typed_opt(&self.scene, shader);
            let shader_type = shader.map(|s| self.scene.shaders[s].shader_type);
            let textures = fetch_textures(&self.scene, id, false);
            let blender_pbr = self.opts.use_blender_pbr_material && self.exporter == Exporter::BlenderBinary && self.exporter_version >= pack_version(4, 12, 0);
            let mat = &mut self.scene.materials[m];
            mat.shader = shader;
            match mat.shading_model_name.as_str() {
                "lambert" | "Lambert" => mat.shader_type = ShaderType::FbxLambert,
                "phong" | "Phong" => mat.shader_type = ShaderType::FbxPhong,
                _ => {}
            }
            if let Some(t) = shader_type {
                mat.shader_type = t;
            } else {
                if blender_pbr {
                    mat.shader_type = ShaderType::BlenderPhong;
                }
                if mat.shader_type == ShaderType::Unknown {
                    let a = mat.element.props.find_int("3dsMax|ClassIDa", 0) as u64 as u32;
                    let b = mat.element.props.find_int("3dsMax|ClassIDb", 0) as u64 as u32;
                    let found = match (a, b) {
                        (0x3d6b1cec, 0xdeadc001) => Some((ShaderType::Max3dsPhysicalMaterial, "3dsMax|Parameters|")),
                        (0xf1551e33, 0x37fb1337) => Some((ShaderType::OpenpbrMaterial, "3dsMax|Parameters|")),
                        (0x38420192, 0x45fe4e1b) => Some((ShaderType::GltfMaterial, "3dsMax|")),
                        (0xd00f1e00, 0xbe77e500) => Some((ShaderType::Max3dsPbrMetalRough, "3dsMax|main|")),
                        (0xd00f1e00, 0x01dbad33) => Some((ShaderType::Max3dsPbrSpecGloss, "3dsMax|main|")),
                        _ => None,
                    };
                    if let Some((t, prefix)) = found {
                        mat.shader_type = t;
                        mat.shader_prop_prefix = prefix.to_string();
                    }
                }
            }
            mat.textures = textures;
        }
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_legacy_mesh_textures(&mut self) {
        #[derive(Clone)]
        struct MatTex {
            material_id: i32,
            texture_id: i32,
            prop_name: String,
        }
        for m in 0..self.scene.meshes.len() {
            let id = self.scene.meshes[m].element.element_id;
            let num_materials = self.scene.meshes[m].materials.len();
            let Some(extra) = self.get_element_extra(id) else { continue };
            if num_materials == 0 {
                continue;
            }
            let mesh_textures = extra.mesh_textures.clone();
            let (mut f, _) = self.fetch();
            let textures = f.fetch_dst_elements(id, true, false, "", ElementType::Texture);
            let textures = typed(&self.scene, &textures);
            let mesh = &self.scene.meshes[m];

            let mut mat_texs: Vec<MatTex> = Vec::new();
            for tex in &mesh_textures {
                if tex.all_same {
                    let texture_id = tex.face_texture.first().map_or(0, |&t| t as i32);
                    if texture_id >= 0 && (texture_id as usize) < textures.len() {
                        for i in 0..num_materials {
                            mat_texs.push(MatTex { material_id: i as i32, texture_id, prop_name: tex.prop_name.clone() });
                        }
                    }
                } else if !mesh.face_material.is_empty() {
                    let num_faces = tex.face_texture.len().min(mesh.num_faces);
                    let (mut prev_material, mut prev_texture) = (-1i32, -1i32);
                    for i in 0..num_faces {
                        let texture_id = tex.face_texture[i] as i32;
                        let material_id = mesh.face_material[i] as i32;
                        if texture_id < 0 || texture_id as usize >= textures.len() {
                            continue;
                        }
                        if material_id < 0 || material_id as usize >= num_materials {
                            continue;
                        }
                        if material_id == prev_material && texture_id == prev_texture {
                            continue;
                        }
                        prev_material = material_id;
                        prev_texture = texture_id;
                        mat_texs.push(MatTex { material_id, texture_id, prop_name: tex.prop_name.clone() });
                    }
                }
            }
            // `ufbxi_cmp_tmp_material_texture_less` and `ufbxi_sort_tmp_material_textures` in ufbx.c
            stable_sort(&mut mat_texs, 32, |a, b| {
                if a.material_id != b.material_id {
                    return a.material_id < b.material_id;
                }
                if a.texture_id != b.texture_id {
                    return a.texture_id < b.texture_id;
                }
                a.prop_name.as_bytes() < b.prop_name.as_bytes()
            });
            mat_texs.push(MatTex { material_id: -1, texture_id: -1, prop_name: String::new() });

            let materials = mesh.materials.clone();
            let mut prev_material = -2i32;
            let mut prev_texture = -2i32;
            let mut prev_prop: Option<String> = None;
            let mut pending: Vec<MaterialTexture> = Vec::new();
            for mt in &mat_texs {
                if mt.material_id != prev_material {
                    if prev_material >= 0 && !pending.is_empty() {
                        if let Some(Some(mat)) = materials.get(prev_material as usize)
                            && self.scene.materials[*mat].textures.is_empty()
                        {
                            self.scene.materials[*mat].textures = std::mem::take(&mut pending);
                        }
                        pending.clear();
                    }
                    if mt.material_id < 0 {
                        break;
                    }
                    prev_material = mt.material_id;
                    prev_texture = -1;
                    prev_prop = None;
                    pending.clear();
                }
                if mt.texture_id == prev_texture && prev_prop.as_deref() == Some(mt.prop_name.as_str()) {
                    continue;
                }
                prev_texture = mt.texture_id;
                prev_prop = Some(mt.prop_name.clone());
                pending.push(MaterialTexture {
                    material_prop: mt.prop_name.clone(),
                    shader_prop: mt.prop_name.clone(),
                    texture: textures[mt.texture_id as usize],
                });
            }
        }
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_material_maps(&mut self) {
        for m in 0..self.scene.materials.len() {
            // `ufbxi_material_texture_less` and `ufbxi_sort_material_textures` in ufbx.c
            stable_sort(&mut self.scene.materials[m].textures, 32, |a, b| a.material_prop.as_bytes() < b.material_prop.as_bytes());
            crate::materials::fetch_maps(&mut self.scene, m);

            let Some(shader) = self.scene.materials[m].shader else { continue };
            let bindings = self.scene.shaders[shader].bindings.clone();
            for b in bindings {
                let prop_bindings = self.scene.shader_bindings[b].prop_bindings.clone();
                let textures = &mut self.scene.materials[m].textures;
                for pb in &prop_bindings {
                    let name = &pb.material_prop;
                    let n = textures.len();
                    let Some(mut index) =
                        lower_bound_eq(0, n, 4, |i| textures[i].material_prop.as_bytes() < name.as_bytes(), |i| textures[i].material_prop == *name)
                    else {
                        continue;
                    };
                    while index < n && textures[index].shader_prop == *name {
                        textures[index].shader_prop = pb.shader_prop.clone();
                        index += 1;
                    }
                }
            }
        }
    }

    fn validate_indices(&mut self, indices: &mut Vec<u32>, max_index: usize) -> Res<()> {
        if max_index == 0 && self.opts.index_error_handling == IndexErrorHandling::Clamp {
            indices.clear();
            return Ok(());
        }
        for ix in indices.iter_mut() {
            if *ix as usize >= max_index {
                *ix = self.fix_index(*ix, max_index)?;
            }
        }
        Ok(())
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c
    fn finalize_collections(&mut self) -> Res<()> {
        for l in 0..self.scene.display_layers.len() {
            let id = self.scene.display_layers[l].element.element_id;
            let (mut f, _) = self.fetch();
            let nodes = f.fetch_dst_elements(id, false, true, "", ElementType::Node);
            self.scene.display_layers[l].nodes = typed(&self.scene, &nodes);
        }
        for s in 0..self.scene.selection_sets.len() {
            let id = self.scene.selection_sets[s].element.element_id;
            let (mut f, _) = self.fetch();
            let nodes = f.fetch_dst_elements(id, false, true, "", ElementType::SelectionNode);
            self.scene.selection_sets[s].nodes = typed(&self.scene, &nodes);
        }
        for s in 0..self.scene.selection_nodes.len() {
            let id = self.scene.selection_nodes[s].element.element_id;
            let mut target_node = typed_opt(&self.scene, fetch_dst_element(&self.scene, id, false, "", ElementType::Node));
            let mut target_mesh = typed_opt(&self.scene, fetch_dst_element(&self.scene, id, false, "", ElementType::Mesh));
            if target_mesh.is_none()
                && let Some(n) = target_node
            {
                target_mesh = self.scene.nodes[n].mesh;
            } else if target_node.is_none()
                && let Some(m) = target_mesh
                && let Some(&n) = self.scene.meshes[m].element.instances.first()
            {
                target_node = Some(n);
            }
            self.scene.selection_nodes[s].target_node = target_node;
            self.scene.selection_nodes[s].target_mesh = target_mesh;
            if let Some(m) = target_mesh {
                let (nv, ne, nf) = (self.scene.meshes[m].num_vertices, self.scene.meshes[m].num_edges, self.scene.meshes[m].num_faces);
                let mut v = std::mem::take(&mut self.scene.selection_nodes[s].vertices);
                let mut e = std::mem::take(&mut self.scene.selection_nodes[s].edges);
                let mut f = std::mem::take(&mut self.scene.selection_nodes[s].faces);
                self.validate_indices(&mut v, nv)?;
                self.validate_indices(&mut e, ne)?;
                self.validate_indices(&mut f, nf)?;
                let node = &mut self.scene.selection_nodes[s];
                (node.vertices, node.edges, node.faces) = (v, e, f);
            }
        }

        for c in 0..self.scene.constraints.len() {
            let mut found: Vec<(usize, String)> = Vec::new();
            let e = &self.scene.constraints[c].element;
            for conn in &e.connections_src {
                if conn.src_prop.is_empty() || type_of(&self.scene, conn.dst) != ElementType::Node {
                    continue;
                }
                found.push((typed_index(&self.scene, conn.dst), conn.src_prop.clone()));
            }
            for conn in &e.connections_dst {
                if conn.dst_prop.is_empty() || type_of(&self.scene, conn.src) != ElementType::Node {
                    continue;
                }
                found.push((typed_index(&self.scene, conn.src), conn.dst_prop.clone()));
            }
            let constraint = &mut self.scene.constraints[c];
            let mut targets = Vec::new();
            for (node, prop) in found {
                add_constraint_prop(constraint, &mut targets, node, &prop);
            }
            constraint.targets = targets;
        }

        for l in 0..self.scene.audio_layers.len() {
            let id = self.scene.audio_layers[l].element.element_id;
            let (mut f, _) = self.fetch();
            let clips = f.fetch_dst_elements(id, false, true, "", ElementType::AudioClip);
            self.scene.audio_layers[l].clips = typed(&self.scene, &clips);
        }

        for l in 0..self.scene.lod_groups.len() {
            finalize_lod_group(&self.scene.nodes, &mut self.scene.lod_groups[l]);
        }
        Ok(())
    }
}

const CONSTRAINT_PROPS: [(u8, &str); 10] = [
    (0, "Constrained Object"),
    (0, "Constrained object (Child)"),
    (0, "First Joint"),
    (4, "Source"),
    (4, "Source (Parent)"),
    (4, "Aim At Object"),
    (4, "Pole Vector Object"),
    (1, "Effector"),
    (2, "End Joint"),
    (3, "World Up Object"),
];

fn add_constraint_prop(constraint: &mut Constraint, targets: &mut Vec<ConstraintTarget>, node: usize, prop: &str) {
    for (kind, name) in CONSTRAINT_PROPS {
        if name != prop {
            continue;
        }
        match kind {
            0 => constraint.node = Some(node),
            1 => constraint.ik_effector = Some(node),
            2 => constraint.ik_end_node = Some(node),
            3 => constraint.aim_up_node = Some(node),
            _ => targets.push(ConstraintTarget { node, weight: 1.0, transform: Transform::IDENTITY }),
        }
    }
}

fn finalize_nurbs_basis(basis: &mut NurbsBasis) -> Res<()> {
    check!(basis.order < u32::MAX / 4);
    basis.num_wrap_control_points = match basis.topology {
        NurbsTopology::Closed => 1,
        NurbsTopology::Periodic => basis.order as usize - 1,
        _ => 0,
    };
    if basis.order > 1 {
        let degree = basis.order as usize - 1;
        let knots = &basis.knot_vector;
        if knots.len() >= 2 * degree + 1 {
            basis.t_min = knots[degree];
            basis.t_max = knots[knots.len() - degree - 1];
            let max_spans = knots.len() - 2 * degree;
            let mut spans = Vec::with_capacity(max_spans);
            let mut prev = Real::NEG_INFINITY;
            for i in 0..max_spans {
                let t = knots[degree + i];
                if t != prev {
                    spans.push(t);
                    prev = t;
                }
            }
            basis.spans = spans;
            basis.valid = (1..knots.len()).all(|i| !(knots[i - 1] > knots[i]));
        }
    }
    Ok(())
}

/// As in ufbx, every instance counts the children of the first one
fn finalize_lod_group(nodes: &[Node], lod: &mut LodGroup) {
    let mut num_levels = 0usize;
    for _ in 0..lod.element.instances.len() {
        num_levels = num_levels.max(nodes[lod.element.instances[0]].children.len());
    }
    let props = &lod.element.props;
    let mut i = 0usize;
    while props.find(&format!("Thresholds|Level{i}")).is_some() {
        num_levels = num_levels.max(i + 1);
        i += 1;
    }

    lod.relative_distances = props.find_bool("ThresholdsUsedAsPercentage", false);
    lod.ignore_parent_transform = !props.find_bool("WorldSpace", true);
    lod.use_distance_limit = props.find_bool("MinMaxDistance", false);
    lod.distance_limit_min = props.find_real("MinDistance", -100.0);
    lod.distance_limit_max = props.find_real("MaxDistance", 100.0);

    let mut levels = vec![LodLevel::default(); num_levels];
    for (i, level) in levels.iter_mut().enumerate() {
        if i > 0 {
            level.distance = props.find_real(&format!("Thresholds|Level{}", i - 1), 0.0);
        } else if lod.relative_distances {
            level.distance = 100.0;
        }
        match props.find_int(&format!("DisplayLevels|Level{i}"), 0) {
            0 => level.display = LodDisplay::UseLod,
            1 => level.display = LodDisplay::Show,
            2 => level.display = LodDisplay::Hide,
            _ => {}
        }
    }
    lod.lod_levels = levels;
}

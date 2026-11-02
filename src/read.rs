use std::sync::Arc;

use crate::api::pack_version;
use crate::context::{Context, DomAlias, ElementInfo, POINTER_ID_START, Template, TmpBonePose, TmpConnection, TmpMeshTexture};
use crate::error::{Res, WarningType, check, fail, fail_msg};
use crate::finalize::is_node_property_name;
use crate::math::{max_real, min_real};
use crate::mesh::{mesh_part_add_face, patch_mesh_reals};
use crate::parse::{PArr, PNode, PVal};
use crate::props::{deduplicate_properties, name_key, sort_properties};
use crate::types::*;

pub(crate) struct TopChildren<'a> {
    pub ix: u32,
    pub lazy: bool,
    pub parsed: std::vec::IntoIter<PNode<'a>>,
    pub read: u32,
}

impl<'a> TopChildren<'a> {
    pub fn parsed(ix: u32, children: Vec<PNode<'a>>) -> TopChildren<'a> {
        TopChildren { ix, lazy: false, parsed: children.into_iter(), read: 0 }
    }
}

impl<'a> Context<'a> {
    #[allow(clippy::wrong_self_convention)] // ufbx's flag `from_ascii`
    pub fn from_ascii(&self) -> bool {
        self.p.from_ascii
    }
    pub fn synthetic_id_from_ptr_id(&mut self, ptr: u64, id: u64) -> u64 {
        if let Some(&v) = self.ptr_fbx_id_map.get(&(ptr, id)) {
            return v;
        }
        let v = self.push_synthetic_id();
        self.ptr_fbx_id_map.insert((ptr, id), v);
        v
    }
    pub fn validate_fbx_id(&mut self, id: u64) -> u64 {
        if id >= POINTER_ID_START { self.synthetic_id_from_ptr_id(0, id) } else { id }
    }
    pub fn insert_fbx_attr(&mut self, fbx_id: u64, attrib_fbx_id: u64) {
        self.fbx_attr_map.entry(fbx_id).or_insert(attrib_fbx_id);
    }
    pub fn connect_pp(&mut self, src: u64, dst: u64, src_prop: &str, dst_prop: &str) {
        self.tmp_connections.push(TmpConnection { src, dst, src_prop: src_prop.to_string(), dst_prop: dst_prop.to_string() });
    }
    pub fn find_template(&self, name: &str, sub_type: &str) -> Option<Arc<Props>> {
        for t in &self.templates {
            if t.type_ == name {
                if !matches!(t.type_.as_str(), "Material" | "Model" | "AnimationStack" | "AnimationLayer") && t.sub_type != sub_type {
                    return None;
                }
                return if t.props.props.is_empty() { None } else { Some(t.props.clone()) };
            }
        }
        None
    }
    /// Extended with `NO_INDEX` to `num_indexers`, every index past `num_elems` fixed, `num_indexers` of them kept
    pub fn check_indices(&mut self, mut indices: Vec<u32>, num_indexers: usize, num_elems: usize) -> Res<Vec<u32>> {
        if indices.len() < num_indexers {
            indices.resize(num_indexers, NO_INDEX);
        }
        for i in 0..indices.len() {
            let ix = indices[i];
            if ix as usize >= num_elems {
                indices[i] = self.fix_index(ix, num_elems)?;
            }
        }
        // a longer list is fixed whole and then counted as long as the mesh's
        indices.truncate(num_indexers);
        Ok(indices)
    }
    pub fn dom_alias(&mut self, base: &Option<DomRef>, parent: &PNode, child: &str, alias: DomAlias) {
        if let Some(base) = base
            && let Some(ci) = parent.children.iter().position(|c| c.name == child)
        {
            let mut path = base.clone();
            path.push(ci as u32);
            self.dom_aliases.push((path, alias));
        }
    }
    pub fn fail_strict(&self, msg: &str) -> Res<()> {
        if self.opts.strict { fail_msg(msg) } else { Ok(()) }
    }
    /// Met for the first time, its children are left to the parser's top level, as ufbx leaves them
    fn take_toplevel(&mut self, name: &str) -> Res<(PNode<'a>, Option<usize>)> {
        let Some(i) = self.p.parse_toplevel(name)? else { return Ok((PNode::default(), None)) };
        Ok((std::mem::take(&mut self.p.top_nodes[i]), Some(i)))
    }
    fn put_toplevel(&mut self, node: PNode<'a>, index: Option<usize>) {
        if let Some(i) = index {
            self.p.top_nodes[i] = node;
        }
    }
    /// Part of `ufbxi_parse_toplevel` in ufbx.c
    pub(crate) fn toplevel_children(&mut self, name: &str) -> Res<TopChildren<'a>> {
        let Some(i) = self.p.parse_toplevel(name)? else { return Ok(TopChildren::parsed(0, Vec::new())) };
        if self.p.is_lazy(i) {
            return Ok(TopChildren { ix: i as u32, lazy: true, parsed: Vec::new().into_iter(), read: 0 });
        }
        Ok(TopChildren::parsed(i as u32, self.p.top_nodes[i].children.clone()))
    }
    /// A node met for the first time has its next child parsed now, kept for the DOM and not for later lookups, as ufbx keeps it
    pub(crate) fn parse_toplevel_child(&mut self, c: &mut TopChildren<'a>) -> Res<Option<PNode<'a>>> {
        let child = if c.lazy {
            let child = self.p.next_lazy_child()?;
            if let Some(ch) = &child
                && self.p.opts.retain_dom
            {
                self.p.keep_child(c.ix as usize, ch.clone());
            }
            child
        } else {
            c.parsed.next()
        };
        if child.is_some() {
            c.read += 1;
        }
        Ok(child)
    }
    pub(crate) fn read_root(&mut self) -> Res<()> {
        let mut children = self.toplevel_children("FBXHeaderExtension")?;
        self.read_header_extension(&mut children)?;
        // ufbx's tokenizer sets the exporter itself on a second header comment "Created by Blender"
        if self.p.ascii_blender {
            self.exporter = Exporter::BlenderAscii;
        }

        if self.exporter == Exporter::BlenderAscii {
            let (top, ix) = self.take_toplevel("Creator")?;
            if ix.is_some()
                && let Some(s) = top.str_at(0)
            {
                self.scene.metadata.creator = s.to_string();
            }
            self.put_toplevel(top, ix);
        }

        self.match_exporter();
        self.p.set_found_version();

        if self.version() >= 7000 {
            let mut children = self.toplevel_children("Documents")?;
            self.read_document(&mut children)?;
        } else {
            let root_name: &[u8] = if self.from_ascii() { b"Model::Scene" } else { b"Scene\x00\x01Model" };
            self.root_id = self.synthetic_id_from_string(root_name);
        }

        let root_info = ElementInfo { fbx_id: self.root_id, ..Default::default() };
        let root = self.push_element(&root_info, ElementType::Node);
        crate::load::setup_root_node(self, root);
        let root_id = self.scene.nodes[root].element.element_id;
        self.tmp_node_ids.push(root_id);

        let mut children = self.toplevel_children("Definitions")?;
        self.read_definitions(&mut children)?;

        let ix = self.p.parse_toplevel("Objects")?;
        if !self.p.sure_fbx {
            check!(ix.is_some(), "Not an FBX file");
        }
        if let Some(ix) = ix {
            self.read_objects(ix)?;
        }

        let mut children = self.toplevel_children("Connections")?;
        self.read_connections(&mut children)?;

        let mut children = self.toplevel_children("Takes")?;
        crate::legacy::read_takes(self, &mut children)?;

        let (top, ix) = self.take_toplevel("GlobalSettings")?;
        if ix.is_some() {
            // `ufbxi_read_global_settings` in ufbx.c
            self.scene.settings.props = self.read_properties(&top)?;
        }
        self.put_toplevel(top, ix);

        let (top, ix) = self.take_toplevel("Version5")?;
        if ix.is_some()
            && let Some(settings) = top.children.iter().find(|c| c.name == "Settings")
        {
            crate::legacy::read_legacy_settings(self, settings)?;
        }
        self.put_toplevel(top, ix);

        if self.opts.retain_dom {
            self.p.parse_to_end()?;
        }
        Ok(())
    }
}

pub(crate) fn get_prop_type(name: &str) -> PropType {
    match name {
        "Boolean" | "bool" | "Bool" => PropType::Boolean,
        "Integer" | "int" | "enum" | "Enum" | "Visibility" | "Visibility Inheritance" | "KTime" => PropType::Integer,
        "Number" | "double" | "Real" | "Float" | "Intensity" => PropType::Number,
        "Vector" | "Vector3D" => PropType::Vector,
        "Color" | "ColorRGB" => PropType::Color,
        "ColorAndAlpha" => PropType::ColorWithAlpha,
        "String" | "KString" | "object" => PropType::String,
        "DateTime" => PropType::DateTime,
        "Lcl Translation" => PropType::Translation,
        "Lcl Rotation" => PropType::Rotation,
        "Lcl Scaling" => PropType::Scaling,
        "Distance" => PropType::Distance,
        "Compound" => PropType::Compound,
        "Blob" => PropType::Blob,
        "Reference" => PropType::Reference,
        _ => PropType::Unknown,
    }
}

// An array of another type counts as absent (`ufbxi_get_array` in ufbx.c)

/// Part of `ufbxi_get_array` in ufbx.c
pub(crate) fn arr_f64<'n>(n: &'n PNode) -> Option<&'n Vec<f64>> {
    match n.get_array()? {
        PArr::F64(v) => Some(v),
        _ => None,
    }
}
/// Part of `ufbxi_get_array` in ufbx.c
pub(crate) fn arr_i32<'n>(n: &'n PNode) -> Option<&'n Vec<i32>> {
    match n.get_array()? {
        PArr::I32(v) => Some(v),
        _ => None,
    }
}
/// Part of `ufbxi_get_array` in ufbx.c
pub(crate) fn arr_i64<'n>(n: &'n PNode) -> Option<&'n Vec<i64>> {
    match n.get_array()? {
        PArr::I64(v) => Some(v),
        _ => None,
    }
}
/// Part of `ufbxi_get_array` in ufbx.c
pub(crate) fn arr_bool<'n>(n: &'n PNode) -> Option<&'n Vec<bool>> {
    match n.get_array()? {
        PArr::Bool(v) | PArr::BoolInts(v, _) => Some(v),
        _ => None,
    }
}
/// Part of `ufbxi_get_array` in ufbx.c
pub(crate) fn arr_u8<'n>(n: &'n PNode) -> Option<&'n Vec<u8>> {
    match n.get_array()? {
        PArr::U8(v) => Some(v),
        _ => None,
    }
}
/// Part of `ufbxi_find_array` in ufbx.c
pub(crate) fn find_f64<'n>(n: &'n PNode, name: &str) -> Option<&'n Vec<f64>> {
    n.find_child(name).and_then(arr_f64)
}
/// Part of `ufbxi_find_array` in ufbx.c
pub(crate) fn find_i32<'n>(n: &'n PNode, name: &str) -> Option<&'n Vec<i32>> {
    n.find_child(name).and_then(arr_i32)
}
/// Part of `ufbxi_find_array` in ufbx.c
pub(crate) fn find_i64<'n>(n: &'n PNode, name: &str) -> Option<&'n Vec<i64>> {
    n.find_child(name).and_then(arr_i64)
}
/// Part of `ufbxi_find_array` in ufbx.c
pub(crate) fn find_bool<'n>(n: &'n PNode, name: &str) -> Option<&'n Vec<bool>> {
    n.find_child(name).and_then(arr_bool)
}
pub(crate) fn as_u32(v: &[i32]) -> Vec<u32> {
    v.iter().map(|&x| x as u32).collect()
}
pub(crate) fn vec3s(v: &[f64]) -> Vec<Vec3> {
    v.as_chunks::<3>().0.iter().map(|c| Vec3::new(c[0], c[1], c[2])).collect()
}
pub(crate) fn vec4s(v: &[f64]) -> Vec<Vec4> {
    v.as_chunks::<4>().0.iter().map(|c| Vec4::new(c[0], c[1], c[2], c[3])).collect()
}

/// 16 values of a column-major 4x4, the last row dropped
pub(crate) fn read_transform_matrix(d: &[f64]) -> Matrix {
    Matrix { m00: d[0], m10: d[1], m20: d[2], m01: d[4], m11: d[5], m21: d[6], m02: d[8], m12: d[9], m22: d[10], m03: d[12], m13: d[13], m23: d[14] }
}

fn match_version_string(fmt: &str, s: &str, out: &mut [u32; 3]) -> bool {
    let s = s.as_bytes();
    let mut pos = 0;
    let mut num_ix = 0;
    for c in fmt.bytes() {
        if c.is_ascii_lowercase() {
            if pos >= s.len() {
                return false;
            }
            let ch = s[pos];
            if ch != c && (ch as i32) + (b'a' - b'A') as i32 != c as i32 {
                return false;
            }
            pos += 1;
        } else if c == b' ' {
            while pos < s.len() && (s[pos] == b' ' || s[pos] == b'\t') {
                pos += 1;
            }
        } else if c == b'-' {
            while pos < s.len() && s[pos] != b'-' {
                pos += 1;
            }
            if pos >= s.len() {
                return false;
            }
            pos += 1;
        } else if matches!(c, b'/' | b'.' | b'(' | b')' | b'_') {
            if pos >= s.len() || s[pos] != c {
                return false;
            }
            pos += 1;
        } else if c == b'?' {
            let mut num = 0u32;
            let mut len = 0;
            while pos < s.len() && s[pos].is_ascii_digit() {
                num = num.wrapping_mul(10).wrapping_add((s[pos] - b'0') as u32);
                pos += 1;
                len += 1;
            }
            if len == 0 {
                return false;
            }
            out[num_ix] = num;
            num_ix += 1;
        }
    }
    true
}

const KEY_INTERPOLATION_CONSTANT: u32 = 0x2;
const KEY_INTERPOLATION_CUBIC: u32 = 0x8;
const KEY_TANGENT_TCB: u32 = 0x200;
const KEY_TANGENT_USER: u32 = 0x400;
const KEY_TANGENT_BROKEN: u32 = 0x800;
const KEY_CONSTANT_NEXT: u32 = 0x100;
const KEY_CLAMP: u32 = 0x1000;
pub(crate) const KEY_TIME_INDEPENDENT: u32 = 0x2000;
pub(crate) const KEY_CLAMP_PROGRESSIVE: u32 = 0x4000;
const KEY_WEIGHTED_RIGHT: u32 = 0x1000000;
const KEY_WEIGHTED_NEXT_LEFT: u32 = 0x2000000;

impl<'a> Context<'a> {
    pub(crate) fn get_dom_node(&self, path: &[u32]) -> Option<DomRef> {
        if self.opts.retain_dom { Some(path.to_vec()) } else { None }
    }

    pub(crate) fn read_embedded_blob(&self, node: Option<&PNode>) -> Option<Vec<u8>> {
        let node = node?;
        match node.get_array()? {
            PArr::Content(parts) if !parts.is_empty() => Some(parts.concat()),
            _ => None,
        }
    }

    fn read_property(&mut self, node: &PNode, version: u32) -> Res<Prop> {
        let mut prop = Prop::default();
        let (Some(name), Some(type_str)) = (node.str_at(0), node.str_at(1)) else { return fail() };
        prop.name = name.to_string();
        let type_str = type_str.to_string();
        let mut val_ix = 2;
        let mut subtype_str: Option<String> = None;
        if version == 70 {
            let Some(s) = node.str_at(val_ix) else { return fail() };
            subtype_str = Some(s.to_string());
            val_ix += 1;
        }
        let mut flags = 0u32;
        prop.internal_key = name_key(prop.name.as_bytes());
        if let Some(fs) = node.str_at(val_ix) {
            let fs = fs.as_bytes();
            for i in 0..fs.len() {
                let next = if i + 1 < fs.len() { fs[i + 1] } else { b'0' };
                match fs[i] {
                    b'A' => flags |= prop_flags::ANIMATABLE,
                    b'U' => flags |= prop_flags::USER_DEFINED,
                    b'H' => flags |= prop_flags::HIDDEN,
                    b'L' => flags |= ((next.wrapping_sub(b'0')) as u32 & 0xf) << 4,
                    b'M' => flags |= ((next.wrapping_sub(b'0')) as u32 & 0xf) << 8,
                    _ => {}
                }
            }
        }
        val_ix += 1;

        prop.prop_type = get_prop_type(&type_str);
        if prop.prop_type == PropType::Unknown
            && let Some(sub) = &subtype_str
        {
            prop.prop_type = get_prop_type(sub);
        }

        if let Some(v) = node.i64_at(val_ix) {
            prop.value_int = v;
            flags |= prop_flags::VALUE_INT;
        }
        let mut real_ix = 0;
        while real_ix < 4 {
            let Some(v) = node.f64_at(val_ix + real_ix) else { break };
            prop.value_real_arr[real_ix] = v;
            real_ix += 1;
        }
        if real_ix > 0 {
            flags |= prop_flags::VALUE_REAL << (real_ix - 1);
        }

        // A number in the string's place is passed over: "Thresholds|Level0", "Distance", "", "", 64, "cm"
        if !matches!(node.vals().get(val_ix), Some(PVal::Str(_))) {
            val_ix += 1;
        }
        if let Some(s) = node.str_at(val_ix) {
            prop.value_str = s.to_string();
            prop.value_blob = node.raw_at(val_ix).unwrap_or(&[]).to_vec();
            flags |= prop_flags::VALUE_STR;
        }

        if !node.children.is_empty() {
            if let Some(blob) = self.read_embedded_blob(node.find_child("BinaryData")) {
                prop.value_blob = blob;
            }
            flags |= prop_flags::VALUE_BLOB;
        }

        prop.flags = flags;
        Ok(prop)
    }

    pub(crate) fn read_properties(&mut self, parent: &PNode) -> Res<Props> {
        let mut version = 70;
        let node = match parent.find_child("Properties70") {
            Some(n) => n,
            None => match parent.find_child("Properties60") {
                Some(n) => {
                    version = 60;
                    n
                }
                None => return Ok(Props::default()),
            },
        };
        let mut props = Vec::with_capacity(node.children.len());
        for c in &node.children {
            props.push(self.read_property(c, version)?);
        }
        sort_properties(&mut props);
        deduplicate_properties(&mut props);
        Ok(Props { props, num_animated: 0, defaults: None })
    }

    fn read_thumbnail(&mut self, node: &PNode) -> Res<Thumbnail> {
        let props = self.read_properties(node)?;
        let custom_width = props.find_int("CustomWidth", 0);
        let custom_height = props.find_int("CustomHeight", 0);
        let mut t = Thumbnail { props, ..Default::default() };
        if let Some(format) = node.find_child("Format").and_then(|n| n.i32_at(0))
            && format >= 0
            && format + 1 < 3
        {
            t.format = if format == 0 { ThumbnailFormat::Rgb24 } else { ThumbnailFormat::Rgba32 };
        }
        if let Some(size) = node.find_i32("Size") {
            if size > 0 {
                t.width = size as u32;
                t.height = size as u32;
            } else if size < 0 && custom_width > 0 && custom_height > 0 {
                t.width = custom_width as u32;
                t.height = custom_height as u32;
            }
        }
        if let Some(data) = node.find_child("ImageData").and_then(arr_u8) {
            t.data = data.clone();
        }
        Ok(t)
    }

    pub(crate) fn read_scene_info(&mut self, node: &PNode) -> Res<()> {
        self.scene.metadata.scene_props = self.read_properties(node)?;
        if let Some(t) = node.find_child("Thumbnail") {
            self.scene.metadata.thumbnail = self.read_thumbnail(t)?;
        }
        Ok(())
    }

    pub(crate) fn read_header_extension(&mut self, children: &mut TopChildren<'a>) -> Res<()> {
        let mut has_tc_definition = false;
        let mut tc_definition = 0;
        let mut header_version = 0;
        while let Some(child) = self.parse_toplevel_child(children)? {
            let child = &child;
            if child.name == "Creator"
                && let Some(s) = child.str_at(0)
            {
                self.scene.metadata.creator = s.to_string();
            }
            if self.version() < 6000
                && child.name == "FBXVersion"
                && let Some(v) = child.i32_at(0)
                && v > 0
                && v < 6000
                && (v as u32) > self.version()
            {
                self.p.version = v as u32;
            }
            if child.name == "FBXHeaderVersion"
                && let Some(v) = child.i32_at(0)
            {
                header_version = v;
            }
            if child.name == "OtherFlags"
                && let Some(v) = child.find_i32("TCDefinition")
            {
                tc_definition = v;
                has_tc_definition = true;
            }
            if child.name == "SceneInfo" {
                self.read_scene_info(child)?;
            }
        }
        // FBX 8000 changes the KTime unit, opted into by `TCDefinition`, which counts from `FBXHeaderVersion` 1004 on;
        // 127 keeps the old unit
        let mut use_v7_ktime = self.version() < 8000;
        if header_version >= 1004 && has_tc_definition {
            use_v7_ktime = tc_definition == 127;
        }
        self.ktime_sec = if use_v7_ktime { 46186158000 } else { 141120000 };
        self.ktime_sec_double = self.ktime_sec as f64;
        Ok(())
    }

    pub(crate) fn match_exporter(&mut self) {
        let creator = self.scene.metadata.creator.clone();
        let mut v = [0u32; 3];
        let (exporter, version) = if match_version_string("blender-- ?.?.?", &creator, &mut v) {
            (Exporter::BlenderBinary, pack_version(v[0], v[1], v[2]))
        } else if match_version_string("blender- ?.?", &creator, &mut v) {
            (Exporter::BlenderBinary, pack_version(v[0], v[1], 0))
        } else if match_version_string("blender version ?.?", &creator, &mut v) {
            (Exporter::BlenderAscii, pack_version(v[0], v[1], 0))
        } else if match_version_string("fbx sdk/fbx plugins version ?.?", &creator, &mut v) {
            (Exporter::FbxSdk, pack_version(v[0], v[1], 0))
        } else if match_version_string("fbx sdk/fbx plugins build ?", &creator, &mut v) {
            (Exporter::FbxSdk, pack_version(v[0] / 10000, v[0] / 100 % 100, v[0] % 100))
        } else if match_version_string("motionbuilder version ?.?", &creator, &mut v)
            || match_version_string("motionbuilder/mocap/online version ?.?", &creator, &mut v)
        {
            (Exporter::MotionBuilder, pack_version(v[0], v[1], 0))
        } else if match_version_string("ufbx_write", &creator, &mut v) {
            (Exporter::UfbxWrite, pack_version(0, 0, 1))
        } else {
            (self.exporter, self.exporter_version)
        };
        self.exporter = exporter;
        self.exporter_version = version;
        self.scene.metadata.exporter = self.exporter;
        self.scene.metadata.exporter_version = self.exporter_version;
        if self.opts.disable_quirks {
            self.exporter = Exporter::Unknown;
            self.exporter_version = 0;
        }
        if self.exporter == Exporter::BlenderBinary {
            self.p.blender_full_weights = true;
        }
    }

    pub(crate) fn read_document(&mut self, children: &mut TopChildren<'a>) -> Res<()> {
        let mut found_root_id = false;
        while let Some(child) = self.parse_toplevel_child(children)? {
            if child.name == "Document"
                && !found_root_id
                && let Some(id) = child.find_i64("RootNode")
            {
                self.root_id = id as u64;
                found_root_id = true;
            }
        }
        Ok(())
    }

    pub(crate) fn read_definitions(&mut self, children: &mut TopChildren<'a>) -> Res<()> {
        while let Some(object) = self.parse_toplevel_child(children)? {
            let object = &object;
            if object.name != "ObjectType" {
                continue;
            }
            let Some(type_) = object.str_at(0) else { return fail() };
            let mut tmpl = Template { type_: type_.to_string(), ..Default::default() };
            if let Some(props) = object.find_child("PropertyTemplate") {
                let Some(sub) = props.str_at(0) else { return fail() };
                let mut sub = sub.to_string();
                if sub.len() > 3 && sub.starts_with("Fbx") {
                    sub = sub[3..].to_string();
                    // LOD groups are "LODGroup" in the template and "LodGroup" in the object
                    if sub == "LODGroup" {
                        sub = "LodGroup".to_string();
                    }
                }
                tmpl.sub_type = sub;
                tmpl.props = Arc::new(self.read_properties(props)?);
            }
            self.templates.push(tmpl);
        }
        Ok(())
    }

    /// "Type::Name" in ASCII, "Name\x00\x01Type" in binary
    pub(crate) fn split_type_and_name(&mut self, s: &[u8]) -> Res<(String, String)> {
        let sep: &[u8] = if self.from_ascii() { b"::" } else { b"\x00\x01" };
        let mut type_end = 2;
        while type_end <= s.len() {
            if s[type_end - 2] == sep[0] && s[type_end - 1] == sep[1] {
                break;
            }
            type_end += 1;
        }
        let (type_, name): (&[u8], &[u8]) = if type_end <= s.len() {
            if self.from_ascii() { (&s[..type_end - 2], &s[type_end..]) } else { (&s[type_end..], &s[..type_end - 2]) }
        } else {
            (b"", s)
        };
        Ok((self.p.utf8(type_)?, self.p.utf8(name)?))
    }

    pub(crate) fn init_synthetic_int_prop(name: &str, value: i64, t: PropType) -> Prop {
        Prop {
            name: name.to_string(),
            internal_key: name_key(&name.as_bytes()[..4]),
            prop_type: t,
            flags: prop_flags::SYNTHETIC | prop_flags::VALUE_REAL | prop_flags::VALUE_INT,
            value_int: value,
            value_real_arr: [value as Real, 0.0, 0.0, 0.0],
            ..Default::default()
        }
    }

    pub(crate) fn init_synthetic_real_prop(name: &str, value: Real, t: PropType) -> Prop {
        Prop {
            name: name.to_string(),
            internal_key: name_key(&name.as_bytes()[..4]),
            prop_type: t,
            flags: prop_flags::SYNTHETIC | prop_flags::VALUE_REAL,
            value_int: value as i64,
            value_real_arr: [value, 0.0, 0.0, 0.0],
            ..Default::default()
        }
    }

    pub(crate) fn read_model(&mut self, info: &ElementInfo) -> Res<usize> {
        let ix = self.push_element(info, ElementType::Node);
        let id = self.scene.nodes[ix].element.element_id;
        self.tmp_node_ids.push(id);
        let inherit_type = self.scene.nodes[ix].element.props.int("InheritType", -1);
        let node = &mut self.scene.nodes[ix];
        match inherit_type {
            0 => node.original_inherit_mode = InheritMode::ComponentwiseScale,
            2 => node.original_inherit_mode = InheritMode::IgnoreParentScale,
            _ => {}
        }
        match self.opts.inherit_mode_handling {
            InheritModeHandling::Preserve => node.inherit_mode = node.original_inherit_mode,
            InheritModeHandling::Ignore => {
                node.original_inherit_mode = InheritMode::Normal;
                node.inherit_mode = InheritMode::Normal;
            }
            _ => {}
        }
        Ok(ix)
    }

    pub(crate) fn read_unknown(&mut self, info: &ElementInfo, type_: &str, sub_type: &[u8], node_name: &[u8]) -> Res<()> {
        let ix = self.push_element(info, ElementType::Unknown);
        let sub_type = self.p.utf8(sub_type)?;
        let super_type = self.p.utf8(node_name)?;
        let u = &mut self.scene.unknowns[ix];
        u.type_ = type_.to_string();
        u.sub_type = sub_type;
        u.super_type = super_type;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn read_vertex_element<T: Copy + Default>(
        &mut self,
        mesh: &Mesh,
        node: &PNode,
        data_name: &str,
        index_name: &str,
        w_name: Option<&str>,
        num_components: usize,
        make: fn(&[f64]) -> T,
    ) -> Res<(VertexAttrib<T>, bool, bool)> {
        let mut attrib = VertexAttrib::<T>::default();
        let mut indices_shared = false;
        let data = find_f64(node, data_name);
        let indices = find_i32(node, index_name);
        if !self.opts.strict && data.is_none() {
            return Ok((attrib, false, false));
        }
        let Some(data) = data else { return fail() };
        check!(data.len().is_multiple_of(num_components));
        let num_elems = data.len() / num_components;
        if num_elems == 0 {
            return Ok((attrib, false, false));
        }
        check!(num_elems < i32::MAX as usize);
        attrib.exists = true;
        let mut mapping = node.find_str("MappingInformationType").unwrap_or("").to_string();
        attrib.values = data.chunks_exact(num_components).map(make).collect();

        // Some old exporters write ByPolygon meaning ByPolygonVertex
        if mapping == "ByPolygon" {
            let num_indices = indices.map_or(num_elems, |i| i.len());
            if num_indices == mesh.num_indices {
                mapping = "ByPolygonVertex".to_string();
            }
        }

        let n = mesh.num_indices;
        if let Some(index_data) = indices {
            let index_data = as_u32(index_data);
            match mapping.as_str() {
                "ByPolygonVertex" => {
                    // ufbx keeps the file's own index array, fixed in place, where it is long enough
                    indices_shared = index_data.len() >= n;
                    attrib.indices = self.check_indices(index_data, n, num_elems)?;
                }
                "ByVertex" | "ByVertice" => {
                    let mut new = vec![0u32; n];
                    for i in 0..n {
                        let ix = mesh.vertex_indices[i];
                        if (ix as usize) < index_data.len() {
                            new[i] = index_data[ix as usize];
                        } else {
                            new[i] = self.fix_index(ix, num_elems)?;
                        }
                    }
                    attrib.indices = self.check_indices(new, n, num_elems)?;
                    attrib.unique_per_vertex = true;
                }
                "ByPolygon" => {
                    let mut new = vec![0u32; n];
                    for (face_ix, face) in mesh.faces.iter().enumerate() {
                        let mut index = NO_INDEX;
                        if face_ix < index_data.len() {
                            index = index_data[face_ix];
                        }
                        if index as usize >= num_elems {
                            index = self.fix_index(index, num_elems)?;
                        }
                        for i in 0..face.num_indices as usize {
                            new[face.index_begin as usize + i] = index;
                        }
                    }
                    attrib.indices = new;
                }
                "AllSame" => {
                    attrib.indices = vec![0; n];
                    attrib.unique_per_vertex = true;
                }
                _ => {
                    self.warn_polygon_mapping(data_name, mapping.as_bytes());
                    return Ok((VertexAttrib::default(), false, false));
                }
            }
        } else {
            match mapping.as_str() {
                "ByPolygonVertex" => {
                    let index_data: Vec<u32> = (0..n as u32).collect();
                    attrib.indices = if num_elems >= n { index_data } else { self.check_indices(index_data, n, num_elems)? };
                }
                "ByVertex" | "ByVertice" => {
                    attrib.indices = self.check_indices(mesh.vertex_position.indices.clone(), n, num_elems)?;
                    attrib.unique_per_vertex = true;
                }
                "ByPolygon" => {
                    let mut new = vec![0u32; n];
                    for (face_ix, face) in mesh.faces.iter().enumerate() {
                        for i in 0..face.num_indices as usize {
                            new[face.index_begin as usize + i] = face_ix as u32;
                        }
                    }
                    attrib.indices = self.check_indices(new, n, num_elems)?;
                }
                "AllSame" => {
                    attrib.indices = vec![0; n];
                    attrib.unique_per_vertex = true;
                }
                _ => {
                    self.warn_polygon_mapping(data_name, mapping.as_bytes());
                    return Ok((VertexAttrib::default(), false, false));
                }
            }
        }

        if self.opts.retain_vertex_attrib_w
            && let Some(w_name) = w_name
            && let Some(w) = find_f64(node, w_name)
        {
            if w.len() == num_elems {
                attrib.values_w = w.clone();
            } else {
                self.warnf(WarningType::BadVertexWAttribute, format!("Bad W array size {w_name}={}, {data_name}={num_elems}", w.len()));
            }
        }
        Ok((attrib, true, indices_shared))
    }

    pub(crate) fn read_truncated_array<T: Copy + Default>(&mut self, arr: Option<&Vec<T>>, name: &str, size: usize) -> Option<Vec<T>> {
        let Some(arr) = arr else {
            self.warnf(WarningType::MissingGeometryData, format!("Missing geometry data: {name}"));
            return None;
        };
        if arr.len() < size {
            self.warnf(WarningType::TruncatedArray, format!("Truncated array: {name}"));
            let mut v = arr.clone();
            let fill = arr.last().copied().unwrap_or_default();
            v.resize(size, fill);
            return Some(v);
        }
        Some(arr[..size].to_vec())
    }

    pub(crate) fn read_shape(&mut self, node: &PNode, info: &ElementInfo) -> Res<()> {
        let (Some(nv), Some(ni)) = (node.find_child("Vertices"), node.find_child("Indexes")) else { return Ok(()) };
        let nn = node.find_child("Normals");
        let ix = self.push_element(info, ElementType::BlendShape);
        if self.opts.ignore_geometry {
            return Ok(());
        }
        let (Some(vertices), Some(indices)) = (arr_f64(nv), arr_i32(ni)) else { return fail() };
        check!(vertices.len().is_multiple_of(3));
        check!(indices.len() == vertices.len() / 3);
        let num_offsets = indices.len();
        let mut offset_vertices = as_u32(indices);
        let mut position_offsets = vec3s(vertices);
        let mut normal_offsets = Vec::new();
        if let Some(nn) = nn {
            let normals = arr_f64(nn);
            check!(normals.is_some_and(|n| n.len() == vertices.len()));
            normal_offsets = vec3s(normals.unwrap());
        }
        let sorted = offset_vertices.windows(2).all(|w| w[0] <= w[1]);
        if !sorted {
            let mut order: Vec<usize> = (0..num_offsets).collect();
            // `ufbxi_blend_offset_less` and `ufbxi_sort_blend_offsets` in ufbx.c
            order.sort_by_key(|&i| offset_vertices[i]);
            offset_vertices = order.iter().map(|&i| offset_vertices[i]).collect();
            position_offsets = order.iter().map(|&i| position_offsets[i]).collect();
            if !normal_offsets.is_empty() {
                normal_offsets = order.iter().map(|&i| normal_offsets[i]).collect();
            }
        }
        let base = info.dom_node.clone();
        self.dom_alias(&base, node, "Vertices", DomAlias::ShapeVertices(ix));
        self.dom_alias(&base, node, "Indexes", DomAlias::ShapeIndices(ix));
        if !normal_offsets.is_empty() {
            self.dom_alias(&base, node, "Normals", DomAlias::ShapeNormals(ix));
        }
        let shape = &mut self.scene.blend_shapes[ix];
        shape.num_offsets = num_offsets;
        shape.offset_vertices = offset_vertices;
        shape.position_offsets = position_offsets;
        shape.normal_offsets = normal_offsets;
        Ok(())
    }

    /// Blend shapes inside the geometry, as files up to 7100 keep them
    pub(crate) fn read_synthetic_blend_shapes(&mut self, node: &PNode, path: &[u32], info: &ElementInfo) -> Res<()> {
        let mut deformer_fbx_id: Option<u64> = None;
        for (ci, n) in node.children.iter().enumerate() {
            if n.name != "Shape" {
                continue;
            }
            let Some(name) = n.str_at(0) else { return fail() };
            let name = name.to_string();
            let mut child_path = path.to_vec();
            child_path.push(ci as u32);
            let dom = self.get_dom_node(&child_path);
            let deformer_id = match deformer_fbx_id {
                Some(id) => id,
                None => {
                    let (_, id) = self.push_synthetic_element(dom.clone(), Some(&name), ElementType::BlendDeformer);
                    self.connect_oo(id, info.fbx_id);
                    deformer_fbx_id = Some(id);
                    id
                }
            };
            let (channel_ix, channel_fbx_id) = self.push_synthetic_element(dom.clone(), Some(&name), ElementType::BlendChannel);
            self.tmp_full_weights.push(Vec::new());
            let mut shape_prop =
                Prop { name: "DeformPercent".to_string(), internal_key: name_key(b"DeformPercent"), prop_type: PropType::Number, ..Default::default() };
            let self_prop = info.props.find(&name);
            if let Some(sp) = self_prop
                && matches!(sp.prop_type, PropType::Number | PropType::Integer)
            {
                shape_prop.value_real_arr[0] = sp.value_real();
                self.connect_pp(info.fbx_id, channel_fbx_id, &name, "DeformPercent");
            } else if self.version() < 6000 {
                self.connect_pp(info.fbx_id, channel_fbx_id, &name, "DeformPercent");
            }
            let channel = &mut self.scene.blend_channels[channel_ix];
            channel.element.name = name.clone();
            channel.element.props.props = vec![shape_prop];

            let shape_info = ElementInfo { fbx_id: self.push_synthetic_id(), name: name.clone(), props: Props::default(), dom_node: dom };
            self.read_shape(n, &shape_info)?;
            self.connect_oo(channel_fbx_id, deformer_id);
            self.connect_oo(shape_info.fbx_id, channel_fbx_id);
        }
        Ok(())
    }

    pub(crate) fn process_indices(&mut self, mesh: &mut Mesh, index_data: &mut [u32]) -> Res<()> {
        let mut faces = Vec::with_capacity(index_data.iter().filter(|&&i| (i as i32) < 0).count());
        let mut num_triangles = 0;
        let mut max_face_triangles = 0;
        let mut num_bad = [0usize; 3];
        let mut face_begin = 0;
        for i in 0..index_data.len() {
            let mut ix = index_data[i];
            if (ix as i32) < 0 {
                ix = !ix;
                index_data[i] = ix;
                let num_indices = (i - face_begin + 1) as u32;
                faces.push(Face { index_begin: face_begin as u32, num_indices });
                if num_indices >= 3 {
                    num_triangles += num_indices as usize - 2;
                    max_face_triangles = max_face_triangles.max(num_indices as usize - 2);
                } else {
                    num_bad[num_indices as usize] += 1;
                }
                face_begin = i + 1;
            }
            check!((ix as usize) < mesh.num_vertices);
        }
        mesh.num_faces = faces.len();
        mesh.faces = faces;
        mesh.num_triangles = num_triangles;
        mesh.max_face_triangles = max_face_triangles;
        mesh.num_empty_faces = num_bad[0];
        mesh.num_point_faces = num_bad[1];
        mesh.num_line_faces = num_bad[2];

        let mut first = vec![NO_INDEX; mesh.num_vertices];
        for ix in 0..index_data.len() {
            let vx = index_data[ix];
            if (vx as usize) < mesh.num_vertices {
                if first[vx as usize] == NO_INDEX {
                    first[vx as usize] = ix as u32;
                }
            } else {
                index_data[ix] = self.fix_index(vx, mesh.num_vertices)?;
            }
        }
        mesh.vertex_first_index = first;
        Ok(())
    }

    pub(crate) fn read_mesh(&mut self, node: &PNode, path: &[u32], info: &ElementInfo) -> Res<usize> {
        let ix = self.push_element(info, ElementType::Mesh);
        if self.version() <= 7100 {
            self.read_synthetic_blend_shapes(node, path, info)?;
        }
        let mut mesh = std::mem::take(&mut self.scene.meshes[ix]);
        let r = self.read_mesh_data(node, &mut mesh);
        patch_mesh_reals(&mut mesh);
        self.scene.meshes[ix] = mesh;
        r?;
        Ok(ix)
    }

    /// Part of `ufbxi_read_mesh` in ufbx.c
    fn read_mesh_data(&mut self, node: &PNode, mesh: &mut Mesh) -> Res<()> {
        let Some(node_vertices) = node.find_child("Vertices") else { return Ok(()) };
        let node_indices = node.find_child("PolygonVertexIndex");
        if self.opts.ignore_geometry {
            return Ok(());
        }
        let vertices = arr_f64(node_vertices);
        let indices = node_indices.map(arr_i32);
        let edge_indices = find_i32(node, "Edges");
        let Some(vertices) = vertices else { return fail() };
        check!(node_indices.is_none() || indices.flatten().is_some());
        let base = mesh.element.dom_node.clone();
        self.dom_alias(&base, node, "Vertices", DomAlias::MeshVertices(mesh.element.typed_id as usize));
        check!(vertices.len().is_multiple_of(3));
        let indices = indices.flatten();

        mesh.num_vertices = vertices.len() / 3;
        mesh.num_indices = indices.map_or(0, |i| i.len());
        let mut index_data: Vec<u32> = indices.map(|i| as_u32(i)).unwrap_or_default();
        mesh.vertices = vec3s(vertices);
        mesh.vertex_position.exists = true;
        mesh.vertex_position.values = mesh.vertices.clone();
        mesh.vertex_position.unique_per_vertex = true;

        if mesh.num_indices > 0 {
            let last = mesh.num_indices - 1;
            if (index_data[last] as i32) >= 0 {
                self.fail_strict("Non-negated last index")?;
                index_data[last] = !index_data[last];
            }
        }

        // Edges are an index into PolygonVertexIndex each, to the next corner of that polygon; read before the
        // indices are turned positive
        if let Some(edge_data) = edge_indices {
            let mut edges = Vec::with_capacity(edge_data.len());
            for &e in edge_data {
                let mut index_ix = e as u32;
                if index_ix as usize >= mesh.num_indices {
                    self.fail_strict("Edge index out of bounds")?;
                    continue;
                }
                let a = index_ix;
                if (index_data[index_ix as usize] as i32) < 0 {
                    while index_ix > 0 && (index_data[index_ix as usize - 1] as i32) >= 0 {
                        index_ix -= 1;
                    }
                } else {
                    index_ix += 1;
                }
                check!((index_ix as usize) < mesh.num_indices);
                edges.push(Edge { a, b: index_ix });
            }
            mesh.num_edges = edges.len();
            mesh.edges = edges;
            mesh.has_edges = true;
        }

        self.process_indices(mesh, &mut index_data)?;
        mesh.vertex_indices = index_data.clone();
        mesh.vertex_position.indices = index_data;

        let mut bitangents: Vec<(VertexVec3, u32, [Option<DomRef>; 2])> = Vec::new();
        let mut tangents: Vec<(VertexVec3, u32, [Option<DomRef>; 2])> = Vec::new();
        let mut uv_sets: Vec<UvSet> = Vec::new();
        let mut color_sets: Vec<ColorSet> = Vec::new();
        let (mut num_uv, mut num_color, mut num_bitangents, mut num_tangents) = (0, 0, 0, 0);
        let mut textures: Vec<TmpMeshTexture> = Vec::new();
        let mut face_group_read = false;

        let m = mesh.element.typed_id as usize;
        let base = mesh.element.dom_node.clone();
        for (li, n) in node.children.iter().enumerate() {
            if !n.name.starts_with('L') {
                continue;
            }
            let layer = base.as_ref().map(|b| [b.as_slice(), &[li as u32]].concat());
            match &*n.name {
                "LayerElementNormal" => {
                    if mesh.vertex_normal.exists {
                        continue;
                    }
                    let (e, vs, is) = self.read_vertex_element(mesh, n, "Normals", "NormalsIndex", Some("NormalsW"), 3, |c| Vec3::new(c[0], c[1], c[2]))?;
                    mesh.vertex_normal = e;
                    if vs {
                        self.dom_alias(&layer, n, "Normals", DomAlias::NormalValues(m));
                    }
                    if is {
                        self.dom_alias(&layer, n, "NormalsIndex", DomAlias::NormalIndices(m));
                    }
                }
                "LayerElementBinormal" => {
                    num_bitangents += 1;
                    let index = n.i32_at(0).map_or(0, |v| v as u32);
                    let (e, vs, is) =
                        self.read_vertex_element(mesh, n, "Binormals", "BinormalsIndex", Some("BinormalsW"), 3, |c| Vec3::new(c[0], c[1], c[2]))?;
                    if e.exists {
                        let paths = [vs.then(|| child_path(&layer, n, "Binormals")).flatten(), is.then(|| child_path(&layer, n, "BinormalsIndex")).flatten()];
                        bitangents.push((e, index, paths));
                    }
                }
                "LayerElementTangent" => {
                    num_tangents += 1;
                    let index = n.i32_at(0).map_or(0, |v| v as u32);
                    let (e, vs, is) = self.read_vertex_element(mesh, n, "Tangents", "TangentsIndex", Some("TangentsW"), 3, |c| Vec3::new(c[0], c[1], c[2]))?;
                    if e.exists {
                        let paths = [vs.then(|| child_path(&layer, n, "Tangents")).flatten(), is.then(|| child_path(&layer, n, "TangentsIndex")).flatten()];
                        tangents.push((e, index, paths));
                    }
                }
                "LayerElementUV" => {
                    num_uv += 1;
                    let index = n.i32_at(0).map_or(0, |v| v as u32);
                    let name = n.find_str("Name").unwrap_or("").to_string();
                    let (e, vs, is) = self.read_vertex_element(mesh, n, "UV", "UVIndex", None, 2, |c| Vec2::new(c[0], c[1]))?;
                    if vs {
                        self.dom_alias(&layer, n, "UV", DomAlias::UvValues(m, index));
                    }
                    if is {
                        self.dom_alias(&layer, n, "UVIndex", DomAlias::UvIndices(m, index));
                    }
                    if e.exists {
                        uv_sets.push(UvSet { name, index, vertex_uv: e, ..Default::default() });
                    }
                }
                "LayerElementColor" => {
                    num_color += 1;
                    let index = n.i32_at(0).map_or(0, |v| v as u32);
                    let name = n.find_str("Name").unwrap_or("").to_string();
                    let (e, vs, is) = self.read_vertex_element(mesh, n, "Colors", "ColorIndex", None, 4, |c| Vec4::new(c[0], c[1], c[2], c[3]))?;
                    if vs {
                        self.dom_alias(&layer, n, "Colors", DomAlias::ColorValues(m, index));
                    }
                    if is {
                        self.dom_alias(&layer, n, "ColorIndex", DomAlias::ColorIndices(m, index));
                    }
                    if e.exists {
                        color_sets.push(ColorSet { name, index, vertex_color: e });
                    }
                }
                "LayerElementVertexCrease" => {
                    mesh.vertex_crease = self.read_vertex_element(mesh, n, "VertexCrease", "VertexCreaseIndex", None, 1, |c| c[0])?.0;
                }
                "LayerElementEdgeCrease" => {
                    let mapping = raw_str(n, "MappingInformationType");
                    if mapping == "ByEdge" {
                        if !mesh.edge_crease.is_empty() {
                            continue;
                        }
                        if let Some(v) = self.read_truncated_array(find_f64(n, "EdgeCrease"), "EdgeCrease", mesh.num_edges) {
                            mesh.edge_crease = v;
                        }
                    } else {
                        self.warn_polygon_mapping("EdgeCrease", &c_bytes(n, "MappingInformationType"));
                    }
                }
                "LayerElementSmoothing" => {
                    let mapping = raw_str(n, "MappingInformationType");
                    if mapping == "ByEdge" {
                        if !mesh.edge_smoothing.is_empty() {
                            continue;
                        }
                        if let Some(v) = self.read_truncated_array(find_bool(n, "Smoothing"), "Smoothing", mesh.num_edges) {
                            mesh.edge_smoothing = v;
                            mesh.has_edge_smoothing = true;
                        }
                    } else if mapping == "ByPolygon" {
                        if !mesh.face_smoothing.is_empty() {
                            continue;
                        }
                        if let Some(v) = self.read_truncated_array(find_bool(n, "Smoothing"), "Smoothing", mesh.num_faces) {
                            mesh.face_smoothing = v;
                            if let Some(PArr::BoolInts(_, ints)) = n.find_child("Smoothing").and_then(|c| c.get_array()) {
                                let mut groups: Vec<u32> = ints.iter().map(|&i| i as u32).collect();
                                let fill = groups.last().copied().unwrap_or(0);
                                groups.resize(mesh.num_faces, fill);
                                mesh.face_smoothing_groups = groups;
                            }
                        }
                    } else {
                        self.warn_polygon_mapping("Smoothing", &c_bytes(n, "MappingInformationType"));
                    }
                }
                "LayerElementVisibility" => {
                    let mapping = raw_str(n, "MappingInformationType");
                    if mapping == "ByEdge" {
                        if !mesh.edge_visibility.is_empty() {
                            continue;
                        }
                        if let Some(v) = self.read_truncated_array(find_bool(n, "Visibility"), "Visibility", mesh.num_edges) {
                            mesh.edge_visibility = v;
                        }
                    } else {
                        self.warn_polygon_mapping("Visibility", &c_bytes(n, "MappingInformationType"));
                    }
                }
                "LayerElementMaterial" => {
                    if !mesh.face_material.is_empty() {
                        continue;
                    }
                    let mapping = raw_str(n, "MappingInformationType");
                    if mapping == "ByPolygon" {
                        if let Some(v) = self.read_truncated_array(find_i32(n, "Materials"), "Materials", mesh.num_faces) {
                            mesh.face_material = as_u32(&v);
                            if find_i32(n, "Materials").is_some_and(|a| a.len() >= mesh.num_faces) {
                                self.dom_alias(&layer, n, "Materials", DomAlias::FaceMaterial(m));
                            }
                        }
                    } else if mapping == "AllSame" {
                        let arr = find_i32(n, "Materials");
                        check!(arr.is_some_and(|a| !a.is_empty()));
                        let material = arr.unwrap()[0] as u32;
                        mesh.face_material = vec![material; mesh.num_faces];
                    } else {
                        self.warn_polygon_mapping("Materials", &c_bytes(n, "MappingInformationType"));
                    }
                }
                "LayerElementPolygonGroup" => {
                    if face_group_read {
                        continue;
                    }
                    let Some(mapping) = n.find_child("MappingInformationType").and_then(|c| c.raw_at(0)) else { return fail() };
                    if mapping == b"ByPolygon"
                        && let Some(v) = self.read_truncated_array(find_i32(n, "PolygonGroup"), "PolygonGroup", mesh.num_faces)
                    {
                        mesh.face_group = as_u32(&v);
                        face_group_read = true;
                        if find_i32(n, "PolygonGroup").is_some_and(|a| a.len() >= mesh.num_faces) {
                            self.dom_alias(&layer, n, "PolygonGroup", DomAlias::FaceGroup(m));
                        }
                    }
                }
                "LayerElementHole" => {
                    if face_group_read {
                        continue;
                    }
                    let Some(mapping) = n.find_child("MappingInformationType").and_then(|c| c.raw_at(0)) else { return fail() };
                    if mapping == b"ByPolygon"
                        && let Some(v) = self.read_truncated_array(find_bool(n, "Hole"), "Hole", mesh.num_faces)
                    {
                        mesh.face_hole = v;
                    }
                }
                other if other.starts_with("LayerElement") => {
                    check!(!other.contains('\0'));
                    // 6x00 keeps textures in the geometry: "LayerElementTexture", "LayerElementDiffuseFactorTextures"...
                    let mut prop_name = String::new();
                    if other.len() > 20 && other.ends_with("Textures") {
                        prop_name = other[12..other.len() - 8].to_string();
                        if prop_name.ends_with('_') {
                            prop_name.pop();
                        }
                    } else if other == "LayerElementTexture" {
                        prop_name = "Diffuse".to_string();
                    }
                    if !prop_name.is_empty()
                        && let Some(mapping) = n.find_child("MappingInformationType").and_then(|c| c.raw_at(0))
                    {
                        let face_texture = find_i32(n, "TextureId").map(|a| as_u32(a)).unwrap_or_default();
                        textures.push(TmpMeshTexture { prop_name, face_texture, all_same: mapping == b"AllSame" });
                    }
                }
                _ => {}
            }
        }

        if mesh.face_material.is_empty() {
            mesh.face_material = vec![0; mesh.num_faces];
        }

        if self.opts.strict {
            check!(uv_sets.len() == num_uv);
            check!(color_sets.len() == num_color);
            check!(bitangents.len() == num_bitangents);
            check!(tangents.len() == num_tangents);
        }

        // Bitangents and tangents go to the UV set the file's Layer lists them with
        for n in &node.children {
            if n.name != "Layer" {
                continue;
            }
            let mut uv_set: Option<usize> = None;
            let mut bitangent: Option<usize> = None;
            let mut tangent: Option<usize> = None;
            for c in &n.children {
                if c.name != "LayerElement" {
                    continue;
                }
                let Some(index) = c.find_i32("TypedIndex").map(|v| v as u32) else { continue };
                let Some(t) = c.find_str("Type") else { continue };
                match t {
                    "LayerElementUV" => {
                        if let Some(i) = uv_sets.iter().position(|s| s.index == index) {
                            uv_set = Some(i);
                        }
                    }
                    "LayerElementBinormal" => {
                        if let Some(i) = bitangents.iter().position(|l| l.1 == index) {
                            bitangent = Some(i);
                        }
                    }
                    "LayerElementTangent" => {
                        if let Some(i) = tangents.iter().position(|l| l.1 == index) {
                            tangent = Some(i);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(u) = uv_set {
                let set_index = uv_sets[u].index;
                if let Some(b) = bitangent {
                    uv_sets[u].vertex_bitangent = bitangents[b].0.clone();
                    let [values, indices] = std::mem::take(&mut bitangents[b].2);
                    if let Some(p) = values {
                        self.dom_aliases.push((p, DomAlias::BitangentValues(m, set_index)));
                    }
                    if let Some(p) = indices {
                        self.dom_aliases.push((p, DomAlias::BitangentIndices(m, set_index)));
                    }
                }
                if let Some(t) = tangent {
                    uv_sets[u].vertex_tangent = tangents[t].0.clone();
                    let [values, indices] = std::mem::take(&mut tangents[t].2);
                    if let Some(p) = values {
                        self.dom_aliases.push((p, DomAlias::TangentValues(m, set_index)));
                    }
                    if let Some(p) = indices {
                        self.dom_aliases.push((p, DomAlias::TangentIndices(m, set_index)));
                    }
                }
            }
        }

        mesh.skinned_is_local = true;
        mesh.skinned_position = mesh.vertex_position.clone();
        mesh.skinned_normal = mesh.vertex_normal.clone();

        // `ufbxi_uv_set_less` and `ufbxi_sort_uv_sets` in ufbx.c
        uv_sets.sort_by_key(|s| s.index);
        // `ufbxi_color_set_less` and `ufbxi_sort_color_sets` in ufbx.c
        color_sets.sort_by_key(|s| s.index);
        mesh.uv_sets = uv_sets;
        mesh.color_sets = color_sets;

        if !mesh.face_group.is_empty() && mesh.face_groups.is_empty() {
            assign_face_groups(mesh, self.retain_mesh_parts)?;
        }

        if !textures.is_empty() {
            let id = mesh.element.element_id;
            self.push_element_extra(id).mesh_textures = textures;
        }

        if let Some(v) = node.find_i32("PreviewDivisionLevels") {
            mesh.subdivision_preview_levels = v as u32;
        }
        if let Some(v) = node.find_i32("RenderDivisionLevels") {
            mesh.subdivision_render_levels = v as u32;
        }
        if let Some(s) = node.find_i32("Smoothness")
            && (0..=3).contains(&s)
        {
            mesh.subdivision_display_mode = match s {
                0 => SubdivisionDisplayMode::Disabled,
                1 => SubdivisionDisplayMode::Hull,
                2 => SubdivisionDisplayMode::HullAndSmooth,
                _ => SubdivisionDisplayMode::Smooth,
            };
        }
        if let Some(b) = node.find_i32("BoundaryRule")
            && (0..=1).contains(&b)
        {
            mesh.subdivision_boundary = if b == 0 { SubdivisionBoundary::Legacy } else { SubdivisionBoundary::SharpCorners };
        }
        Ok(())
    }

    pub(crate) fn read_nurbs_curve(&mut self, node: &PNode, info: &ElementInfo) -> Res<()> {
        let ix = self.push_element(info, ElementType::NurbsCurve);
        let Some(order) = node.find_i32("Order") else { return fail() };
        let dimension = node.find_i32("Dimension").unwrap_or(3);
        let Some(form) = node.find_str("Form") else { return fail() };
        let topology = read_nurbs_topology(form);
        let mut control_points = Vec::new();
        let mut knots = Vec::new();
        if !self.opts.ignore_geometry {
            let (Some(points), Some(knot)) = (find_f64(node, "Points"), find_f64(node, "KnotVector")) else { return fail() };
            check!(points.len().is_multiple_of(4));
            control_points = vec4s(points);
            knots = knot.clone();
            let base = info.dom_node.clone();
            self.dom_alias(&base, node, "Points", DomAlias::NurbsCurvePoints(ix));
        }
        let c = &mut self.scene.nurbs_curves[ix];
        c.basis.order = order as u32;
        c.basis.topology = topology;
        c.basis.is_2d = dimension == 2;
        c.control_points = control_points;
        c.basis.knot_vector = knots;
        Ok(())
    }

    pub(crate) fn read_nurbs_surface(&mut self, node: &PNode, info: &ElementInfo) -> Res<()> {
        let ix = self.push_element(info, ElementType::NurbsSurface);
        let o = node.find_child("NurbsSurfaceOrder");
        let (Some(order_u), Some(order_v)) = (o.and_then(|n| n.i32_at(0)), o.and_then(|n| n.i32_at(1))) else { return fail() };
        let d = node.find_child("Dimensions");
        let (Some(dim_u), Some(dim_v)) = (d.and_then(|n| n.size_at(0)), d.and_then(|n| n.size_at(1))) else { return fail() };
        let s = node.find_child("Step");
        let (Some(step_u), Some(step_v)) = (s.and_then(|n| n.i32_at(0)), s.and_then(|n| n.i32_at(1))) else { return fail() };
        let f = node.find_child("Form");
        let (Some(form_u), Some(form_v)) = (f.and_then(|n| n.str_at(0)), f.and_then(|n| n.str_at(1))) else { return fail() };
        let flip = node.find_child("FlipNormals").and_then(|n| n.bool_at(0)).unwrap_or(false);
        if let Some(most) = (u32::MAX as usize).checked_div(dim_u) {
            check!(dim_v <= most);
        }
        let (tu, tv) = (read_nurbs_topology(form_u), read_nurbs_topology(form_v));
        let mut control_points = Vec::new();
        let (mut ku, mut kv) = (Vec::new(), Vec::new());
        if !self.opts.ignore_geometry {
            let (Some(points), Some(knot_u), Some(knot_v)) = (find_f64(node, "Points"), find_f64(node, "KnotVectorU"), find_f64(node, "KnotVectorV")) else {
                return fail();
            };
            check!(points.len().is_multiple_of(4));
            check!(points.len() / 4 == dim_u * dim_v);
            control_points = vec4s(points);
            ku = knot_u.clone();
            kv = knot_v.clone();
            let base = info.dom_node.clone();
            self.dom_alias(&base, node, "Points", DomAlias::NurbsSurfacePoints(ix));
        }
        let n = &mut self.scene.nurbs_surfaces[ix];
        n.basis_u.order = order_u as u32;
        n.basis_v.order = order_v as u32;
        n.flip_normals = flip;
        n.basis_u.topology = tu;
        n.basis_v.topology = tv;
        n.num_control_points_u = dim_u;
        n.num_control_points_v = dim_v;
        n.span_subdivision_u = if step_u > 0 { step_u as u32 } else { 4 };
        n.span_subdivision_v = if step_v > 0 { step_v as u32 } else { 4 };
        n.control_points = control_points;
        n.basis_u.knot_vector = ku;
        n.basis_v.knot_vector = kv;
        Ok(())
    }

    pub(crate) fn read_line(&mut self, node: &PNode, info: &ElementInfo) -> Res<()> {
        let ix = self.push_element(info, ElementType::LineCurve);
        if self.opts.ignore_geometry {
            return Ok(());
        }
        let (Some(points), Some(points_index)) = (find_f64(node, "Points"), find_i32(node, "PointsIndex")) else { return fail() };
        check!(points.len().is_multiple_of(3));
        if points.is_empty() {
            return Ok(());
        }
        let control_points = vec3s(points);
        let mut point_indices = as_u32(points_index);
        check!(control_points.len() < i32::MAX as usize);
        let mut segments = Vec::new();
        let mut prev_end = 0usize;
        let count = point_indices.len();
        for i in 0..count {
            let mut ix = point_indices[i];
            if (ix as i32) < 0 {
                ix = !ix;
                if i + 1 < count {
                    segments.push(LineSegment { index_begin: prev_end as u32, num_indices: (i - prev_end) as u32 });
                    prev_end = i;
                }
            }
            if (ix as usize) < control_points.len() {
                point_indices[i] = ix;
            } else {
                point_indices[i] = self.fix_index(ix, control_points.len())?;
            }
        }
        segments.push(LineSegment { index_begin: prev_end as u32, num_indices: (count - prev_end) as u32 });
        let base = info.dom_node.clone();
        self.dom_alias(&base, node, "Points", DomAlias::LinePoints(ix));
        self.dom_alias(&base, node, "PointsIndex", DomAlias::LineIndices(ix));
        let l = &mut self.scene.line_curves[ix];
        l.control_points = control_points;
        l.point_indices = point_indices;
        l.segments = segments;
        Ok(())
    }

    pub(crate) fn read_bone(&mut self, info: &ElementInfo, sub_type: &str) {
        let ix = self.push_element(info, ElementType::Bone);
        if sub_type == "Root" {
            self.scene.bones[ix].is_root = true;
        }
    }

    pub(crate) fn read_marker(&mut self, info: &ElementInfo, marker_type: MarkerType) {
        let ix = self.push_element(info, ElementType::Marker);
        self.scene.markers[ix].marker_type = marker_type;
    }

    pub(crate) fn read_skin(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::SkinDeformer);
        let skin = &mut self.scene.skin_deformers[ix];
        if let Some(t) = node.find_str("SkinningType") {
            match t {
                "Rigid" => skin.skinning_method = SkinningMethod::Rigid,
                "Linear" => skin.skinning_method = SkinningMethod::Linear,
                "DualQuaternion" => skin.skinning_method = SkinningMethod::DualQuaternion,
                "Blend" => skin.skinning_method = SkinningMethod::BlendedDqLinear,
                _ => {}
            }
        }
        if let (Some(indices), Some(weights)) = (find_i32(node, "Indexes"), find_f64(node, "BlendWeights")) {
            let n = indices.len().min(weights.len());
            skin.num_dq_weights = n;
            skin.dq_vertices = as_u32(&indices[..n]);
            skin.dq_weights = weights[..n].to_vec();
        }
    }

    pub(crate) fn read_skin_cluster(&mut self, node: &PNode, info: &ElementInfo) -> Res<()> {
        let ix = self.push_element(info, ElementType::SkinCluster);
        let indices = find_i32(node, "Indexes");
        let weights = find_f64(node, "Weights");
        let transform = find_f64(node, "Transform");
        let transform_link = find_f64(node, "TransformLink");
        let cluster = &mut self.scene.skin_clusters[ix];
        if let (Some(indices), Some(weights)) = (indices, weights) {
            check!(indices.len() == weights.len());
            cluster.num_weights = indices.len();
            cluster.vertices = as_u32(indices);
            cluster.weights = weights.clone();
        }
        if let (Some(t), Some(tl)) = (transform, transform_link) {
            check!(t.len() >= 16);
            check!(tl.len() >= 16);
            cluster.mesh_node_to_bone = read_transform_matrix(t);
            cluster.bind_to_world = read_transform_matrix(tl);
        }
        Ok(())
    }

    pub(crate) fn read_blend_channel(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::BlendChannel);
        let full = find_f64(node, "FullWeights").cloned().unwrap_or_default();
        self.tmp_full_weights.push(full);
        // Blender writes DeformPercent as a field, the animation still goes to the property
        let deform = node.find_child("DeformPercent");
        let channel = &mut self.scene.blend_channels[ix];
        if channel.element.props.props.is_empty()
            && let Some(d) = deform
        {
            let value = d.f64_at(0).unwrap_or(100.0);
            channel.element.props.props = vec![Prop {
                name: "DeformPercent".to_string(),
                internal_key: name_key(b"DeformPercent"),
                prop_type: PropType::Number,
                value_real_arr: [value, 0.0, 0.0, 0.0],
                ..Default::default()
            }];
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn solve_auto_tangent(
        &self,
        prev_time: f64,
        time: f64,
        next_time: f64,
        prev_value: Real,
        value: Real,
        next_value: Real,
        weight_left: f32,
        weight_right: f32,
        auto_bias: f32,
        flags: u32,
    ) -> f32 {
        if flags & KEY_CLAMP != 0 && (prev_value - value).abs().min((next_value - value).abs()) <= self.opts.key_clamp_threshold {
            return 0.0;
        }
        let mut slope = (next_value - prev_value) / (next_time - prev_time);
        if flags & KEY_TIME_INDEPENDENT == 0 {
            let slope_left = (value - prev_value) / (time - prev_time);
            let slope_right = (next_value - value) / (next_time - time);
            let delta = (time - prev_time) / (next_time - prev_time);
            slope = slope * 0.5 + (slope_left * (1.0 - delta) + slope_right * delta) * 0.5;
            let bias_weight = (auto_bias as f64).abs() / 100.0;
            if bias_weight > 0.0001 {
                let bias_target = if auto_bias as f64 > 0.0 { slope_right } else { slope_left };
                let bias_delta = bias_target - slope;
                slope = slope * (1.0 - bias_weight) + bias_target * bias_weight;
                // A bias past 500 either way adds ((bias - 500) / 100)^2 * 40 to the slope
                let abs_bias_weight = bias_weight - 5.0;
                if abs_bias_weight > 0.0 {
                    let mut sign = if bias_delta.abs() > 0.00001 { bias_delta } else { auto_bias as f64 };
                    sign = if sign > 0.0 { 1.0 } else { -1.0 };
                    slope += abs_bias_weight * abs_bias_weight * sign * 40.0;
                }
            }
        }
        if flags & KEY_CLAMP_PROGRESSIVE != 0 {
            let sign = if slope >= 0.0 { 1.0 } else { -1.0 };
            let mut abs_slope = sign * slope;
            let range_left = weight_left as f64 * (time - prev_time);
            let range_right = weight_right as f64 * (next_time - time);
            let mut max_left = if range_left > 0.0 { sign * (value - prev_value) / range_left } else { 0.0 };
            let mut max_right = if range_right > 0.0 { sign * (next_value - value) / range_right } else { 0.0 };
            #[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN fails both, as in C
            if !(max_left > 0.0) {
                max_left = 0.0;
            }
            #[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN fails both, as in C
            if !(max_right > 0.0) {
                max_right = 0.0;
            }
            if abs_slope > max_left {
                abs_slope = max_left;
            }
            if abs_slope > max_right {
                abs_slope = max_right;
            }
            slope = sign * abs_slope;
        }
        slope as f32
    }

    /// `ufbxi_solve_auto_tangent_left` and `ufbxi_solve_auto_tangent_right` in ufbx.c
    fn solve_auto_tangent_side(&self, delta_value: Real, delta_time: f64, auto_bias: f32, flags: u32) -> f32 {
        if flags & KEY_CLAMP_PROGRESSIVE != 0 {
            return 0.0;
        }
        if flags & KEY_CLAMP != 0 && delta_value.abs() <= self.opts.key_clamp_threshold {
            return 0.0;
        }
        let mut slope = delta_value / delta_time;
        if flags & KEY_TIME_INDEPENDENT == 0 {
            let abs_bias_weight = (auto_bias as f64).abs() / 100.0 - 5.0;
            if abs_bias_weight > 0.0 {
                let sign = if auto_bias as f64 > 0.0 { 1.0 } else { -1.0 };
                slope += abs_bias_weight * abs_bias_weight * sign * 40.0;
            }
        }
        slope as f32
    }

    pub(crate) fn read_animation_curve(&mut self, node: &PNode, info: &ElementInfo) -> Res<()> {
        let ix = self.push_element(info, ElementType::AnimCurve);
        let pre = read_extrapolation(node, "Pre-Extrapolation");
        let post = read_extrapolation(node, "Post-Extrapolation");
        self.scene.anim_curves[ix].pre_extrapolation = pre;
        self.scene.anim_curves[ix].post_extrapolation = post;
        if self.opts.ignore_animation {
            return Ok(());
        }
        let (Some(times), Some(values), Some(attr_flags), Some(attrs), Some(refs)) = (
            find_i64(node, "KeyTime"),
            find_f64(node, "KeyValueFloat"),
            find_i32(node, "KeyAttrFlags"),
            node.find_array("KeyAttrDataFloat"),
            find_i32(node, "KeyAttrRefCount"),
        ) else {
            return fail();
        };
        check!(times.len() == values.len());
        check!(attr_flags.len() == refs.len());
        check!(attrs.len() == refs.len() * 4);
        // The attributes are four floats per key, as floats or as their bits in an integer array
        let attr_f32: Vec<f32> = match attrs {
            PArr::F32(v) => v.clone(),
            PArr::I32(v) => v.iter().map(|&i| f32::from_bits(i as u32)).collect(),
            PArr::F64(v) => v.iter().map(|&f| f as f32).collect(),
            PArr::I64(v) => v.iter().map(|&i| f32::from_bits(i as u32)).collect(),
            _ => return fail(),
        };
        let attr_bits = |k: usize| -> u32 {
            match attrs {
                PArr::I32(v) => v[k] as u32,
                PArr::I64(v) => v[k] as u32,
                _ => attr_f32[k].to_bits(),
            }
        };

        let num_keys = times.len();
        let mut keys = Vec::with_capacity(num_keys);
        let mut slope_left: f32 = 0.0;
        let mut weight_left: f32 = 0.333333;
        let mut prev_time = 0.0f64;
        let mut next_time = 0.0f64;
        let (mut p_flag, mut p_attr, mut p_ref) = (0usize, 0usize, 0usize);
        let mut refs_left = 0i32;
        if num_keys > 0 {
            next_time = times[0] as f64 / self.ktime_sec_double;
            if p_ref < refs.len() {
                refs_left = refs[p_ref];
            }
        }
        let (mut min_value, mut max_value) = (0.0, 0.0);
        for i in 0..num_keys {
            check!(refs_left > 0);
            let value = values[i];
            if i == 0 {
                min_value = value;
                max_value = value;
            } else {
                min_value = min_real(min_value, value);
                max_value = max_real(max_value, value);
            }
            let mut key = Keyframe { time: next_time, value, ..Default::default() };
            if i + 1 < num_keys {
                next_time = times[i + 1] as f64 / self.ktime_sec_double;
            }
            let flags = attr_flags[p_flag] as u32;
            let mut slope_right = attr_f32[p_attr];
            let mut weight_right: f32 = 0.333333;
            let mut next_slope_left = attr_f32[p_attr + 1];
            let mut next_weight_left: f32 = 0.333333;

            if flags & (KEY_WEIGHTED_RIGHT | KEY_WEIGHTED_NEXT_LEFT) != 0 {
                // Two 0.4 decimal fixed-point weights packed in the bits of the third float
                let packed = attr_bits(p_attr + 2);
                if flags & KEY_WEIGHTED_RIGHT != 0 {
                    weight_right = (packed & 0xffff) as f32 * 0.0001f32;
                }
                if flags & KEY_WEIGHTED_NEXT_LEFT != 0 {
                    next_weight_left = (packed >> 16) as f32 * 0.0001f32;
                }
            }

            let prev_value = if i > 0 { values[i - 1] } else { 0.0 };
            let next_value = if i + 1 < num_keys { values[i + 1] } else { 0.0 };

            if flags & KEY_INTERPOLATION_CONSTANT != 0 {
                key.interpolation = if flags & KEY_CONSTANT_NEXT != 0 { Interpolation::ConstantNext } else { Interpolation::ConstantPrev };
                weight_right = 0.333333;
                next_weight_left = 0.333333;
                slope_right = 0.0;
                next_slope_left = 0.0;
            } else if flags & KEY_INTERPOLATION_CUBIC != 0 {
                key.interpolation = Interpolation::Cubic;
                if flags & KEY_TANGENT_TCB != 0 {
                    let mut tcb_left = 0.0;
                    let mut tcb_right = 0.0;
                    let mut edge = false;
                    if i > 0 && key.time > prev_time {
                        tcb_left = (key.value - prev_value) / (key.time - prev_time);
                    } else {
                        edge = true;
                    }
                    if i + 1 < num_keys && next_time > key.time {
                        tcb_right = (next_value - key.value) / (next_time - key.time);
                    } else {
                        edge = true;
                    }
                    let (l, r) = solve_tcb(attr_f32[p_attr] as f64, attr_f32[p_attr + 1] as f64, attr_f32[p_attr + 2] as f64, tcb_left, tcb_right, edge);
                    slope_left = l;
                    slope_right = r;
                    next_slope_left = 0.0;
                    next_weight_left = 0.333333;
                } else if flags & KEY_TANGENT_USER != 0 {
                    let _ = flags & KEY_TANGENT_BROKEN;
                } else if i > 0 && i + 1 < num_keys && key.time > prev_time && next_time > key.time {
                    if ((slope_left + slope_right) as f64).abs() <= 0.0001f32 as f64 {
                        let s = self.solve_auto_tangent(
                            prev_time,
                            key.time,
                            next_time,
                            prev_value,
                            key.value,
                            next_value,
                            weight_left,
                            weight_right,
                            slope_right,
                            flags,
                        );
                        slope_left = s;
                        slope_right = s;
                    } else {
                        let l = self.solve_auto_tangent(
                            prev_time,
                            key.time,
                            next_time,
                            prev_value,
                            key.value,
                            next_value,
                            weight_left,
                            weight_right,
                            -slope_left,
                            flags,
                        );
                        let r = self.solve_auto_tangent(
                            prev_time,
                            key.time,
                            next_time,
                            prev_value,
                            key.value,
                            next_value,
                            weight_left,
                            weight_right,
                            slope_right,
                            flags,
                        );
                        slope_left = l;
                        slope_right = r;
                    }
                } else if i > 0 && key.time > prev_time {
                    let s = self.solve_auto_tangent_side(key.value - prev_value, key.time - prev_time, -slope_left, flags);
                    slope_left = s;
                    slope_right = s;
                } else if i + 1 < num_keys && next_time > key.time {
                    let s = self.solve_auto_tangent_side(next_value - key.value, next_time - key.time, slope_right, flags);
                    slope_left = s;
                    slope_right = s;
                } else {
                    slope_left = 0.0;
                    slope_right = 0.0;
                }
            } else {
                key.interpolation = Interpolation::Linear;
                weight_right = 0.333333;
                next_weight_left = 0.333333;
                if next_time > key.time {
                    let delta_time = next_time - key.time;
                    if delta_time > 0.0 {
                        let slope = (next_value - key.value) / delta_time;
                        slope_right = slope as f32;
                        next_slope_left = slope as f32;
                    } else {
                        slope_right = 0.0;
                        next_slope_left = 0.0;
                    }
                } else {
                    slope_right = 0.0;
                    next_slope_left = 0.0;
                }
            }

            if key.time > prev_time {
                let delta = key.time - prev_time;
                key.left.dx = (weight_left as f64 * delta) as f32;
                key.left.dy = key.left.dx * slope_left;
            }
            if next_time > key.time {
                let delta = next_time - key.time;
                key.right.dx = (weight_right as f64 * delta) as f32;
                key.right.dy = key.right.dx * slope_right;
            }
            slope_left = next_slope_left;
            weight_left = next_weight_left;
            prev_time = key.time;
            keys.push(key);

            refs_left -= 1;
            if refs_left == 0 {
                p_flag += 1;
                p_attr += 4;
                p_ref += 1;
                if p_ref < refs.len() {
                    refs_left = refs[p_ref];
                }
            }
        }
        let c = &mut self.scene.anim_curves[ix];
        c.keyframes = keys;
        c.min_value = min_value;
        c.max_value = max_value;
        Ok(())
    }

    pub(crate) fn read_material(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::Material);
        self.scene.materials[ix].shading_model_name = node.find_str("ShadingModel").unwrap_or("").to_string();
    }

    fn read_filenames(node: &PNode) -> (String, String, Vec<u8>, Vec<u8>) {
        let mut abs = String::new();
        let mut rel = String::new();
        let mut raw_abs = Vec::new();
        let mut raw_rel = Vec::new();
        for (name, is_abs) in [("FileName", true), ("Filename", true), ("RelativeFileName", false), ("RelativeFilename", false)] {
            let Some(c) = node.find_child(name) else { continue };
            if let Some(s) = c.str_at(0) {
                if is_abs { abs = s.to_string() } else { rel = s.to_string() }
            }
            if let Some(b) = c.raw_at(0) {
                if is_abs { raw_abs = b.to_vec() } else { raw_rel = b.to_vec() }
            }
        }
        (abs, rel, raw_abs, raw_rel)
    }

    pub(crate) fn read_texture(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::Texture);
        let (abs, rel, raw_abs, raw_rel) = Self::read_filenames(node);
        let t = &mut self.scene.textures[ix];
        t.texture_type = TextureType::File;
        t.absolute_filename = abs;
        t.relative_filename = rel;
        t.raw_absolute_filename = raw_abs;
        t.raw_relative_filename = raw_rel;
    }

    pub(crate) fn read_layered_texture(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::Texture);
        self.scene.textures[ix].texture_type = TextureType::Layered;
        let id = self.scene.textures[ix].element.element_id;
        let alphas = find_f64(node, "Alphas").cloned();
        let blend_modes = find_i32(node, "BlendModes").cloned();
        let extra = self.push_element_extra(id);
        extra.alphas = Some(alphas.unwrap_or_default());
        extra.blend_modes = Some(blend_modes.unwrap_or_default());
    }

    pub(crate) fn read_video(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::Video);
        let (abs, rel, raw_abs, raw_rel) = Self::read_filenames(node);
        let content = self.read_embedded_blob(node.find_child("Content"));
        let v = &mut self.scene.videos[ix];
        v.absolute_filename = abs;
        v.relative_filename = rel;
        v.raw_absolute_filename = raw_abs;
        v.raw_relative_filename = raw_rel;
        if let Some(c) = content {
            v.content = c;
        }
    }

    pub(crate) fn read_anim_stack(&mut self, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::AnimStack);
        self.anim_stack_map.entry(info.name.clone()).or_insert(ix);
    }

    pub(crate) fn read_pose(&mut self, node: &PNode, info: &ElementInfo, sub_type: &str) -> Res<()> {
        let ix = self.push_element(info, ElementType::Pose);
        self.scene.poses[ix].is_bind_pose = sub_type == "BindPose";
        let mut bones = Vec::new();
        for n in &node.children {
            if n.name != "PoseNode" {
                continue;
            }
            let fbx_id = if self.version() < 7000 {
                let Some(name) = n.find_child("Node").and_then(|c| c.raw_at(0)) else { continue };
                let name = name.to_vec();
                self.synthetic_id_from_string(&name)
            } else {
                let Some(id) = n.find_i64("Node") else { continue };
                self.validate_fbx_id(id as u64)
            };
            let Some(matrix) = find_f64(n, "Matrix") else { continue };
            check!(matrix.len() >= 16);
            bones.push(TmpBonePose { bone_fbx_id: fbx_id, bone_to_world: read_transform_matrix(matrix) });
        }
        let id = self.scene.poses[ix].element.element_id;
        self.push_element_extra(id).bone_poses = bones;
        Ok(())
    }

    pub(crate) fn read_binding_table(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::ShaderBinding);
        let mut bindings = Vec::new();
        for n in &node.children {
            if n.name != "Entry" {
                continue;
            }
            let (Some(src), Some(src_type), Some(dst), Some(dst_type)) = (n.str_at(0), n.str_at(1), n.str_at(2), n.str_at(3)) else { continue };
            if src_type == "FbxPropertyEntry" && dst_type == "FbxSemanticEntry" {
                bindings.push(ShaderPropBinding { material_prop: src.to_string(), shader_prop: dst.to_string() });
            } else if src_type == "FbxSemanticEntry" && dst_type == "FbxPropertyEntry" {
                bindings.push(ShaderPropBinding { material_prop: dst.to_string(), shader_prop: src.to_string() });
            }
        }
        // `ufbxi_sort_shader_prop_bindings` in ufbx.c
        bindings.sort_by(|a, b| a.shader_prop.as_bytes().cmp(b.shader_prop.as_bytes()));
        self.scene.shader_bindings[ix].prop_bindings = bindings;
    }

    pub(crate) fn read_selection_node(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::SelectionNode);
        let s = &mut self.scene.selection_nodes[ix];
        if node.find_i32("IsTheNodeInSet").is_some_and(|v| v != 0) {
            s.include_node = true;
        }
        // `ufbxi_find_uint32_list` in ufbx.c
        s.vertices = find_i32(node, "VertexIndexArray").map(|a| as_u32(a)).unwrap_or_default();
        s.edges = find_i32(node, "EdgeIndexArray").map(|a| as_u32(a)).unwrap_or_default();
        s.faces = find_i32(node, "PolygonIndexArray").map(|a| as_u32(a)).unwrap_or_default();
        // the lists are the file's arrays in ufbx, whose index fixes its DOM shows
        let dom = info.dom_node.clone();
        for (name, alias) in [
            ("VertexIndexArray", DomAlias::SelectionVertices(ix)),
            ("EdgeIndexArray", DomAlias::SelectionEdges(ix)),
            ("PolygonIndexArray", DomAlias::SelectionFaces(ix)),
        ] {
            if find_i32(node, name).is_some() {
                self.dom_alias(&dom, node, name, alias);
            }
        }
    }

    pub(crate) fn read_audio_clip(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::AudioClip);
        if let Some(c) = self.read_embedded_blob(node.find_child("Content")) {
            self.scene.audio_clips[ix].content = c;
        }
    }

    pub(crate) fn read_constraint(&mut self, node: &PNode, info: &ElementInfo) {
        let ix = self.push_element(info, ElementType::Constraint);
        let type_name = node.find_str("Type").unwrap_or("").to_string();
        let c = &mut self.scene.constraints[ix];
        c.constraint_type = match type_name.as_str() {
            "Aim" => ConstraintType::Aim,
            "Parent-Child" => ConstraintType::Parent,
            "Position From Positions" => ConstraintType::Position,
            "Rotation From Rotations" => ConstraintType::Rotation,
            "Scale From Scales" => ConstraintType::Scale,
            "Single Chain IK" => ConstraintType::SingleChainIk,
            _ => ConstraintType::Unknown,
        };
        c.type_name = type_name;
    }

    #[allow(clippy::too_many_arguments)]
    fn read_synthetic_attribute(&mut self, node: &PNode, path: &[u32], info: &mut ElementInfo, type_str: &str, sub_raw: &[u8], super_type: &str) -> Res<()> {
        let mut sub_raw = c_str(sub_raw);
        // Some 6000 files have meshes without a sub-type: told apart by their Vertices and PolygonVertexIndex
        if sub_raw.is_empty() && node.find_child("Vertices").is_some() && node.find_child("PolygonVertexIndex").is_some() {
            sub_raw = b"Mesh";
        }
        let sub_type = String::from_utf8_lossy(sub_raw).into_owned();
        if (sub_type.is_empty() || sub_type == "Model") && type_str == "Model" {
            return Ok(());
        }
        let mut attrib = info.clone();
        attrib.fbx_id = self.push_synthetic_id();
        if let Some(type_and_name) = node.find_child("NodeAttributeName").and_then(|c| c.raw_at(0)) {
            let type_and_name = type_and_name.to_vec();
            let (_, attrib_name) = self.split_type_and_name(&type_and_name)?;
            if !attrib_name.is_empty() {
                attrib.name = attrib_name;
                let attrib_id = self.synthetic_id_from_string(&type_and_name);
                // `ufbxi_fbx_id_exists` in ufbx.c
                if info.fbx_id != attrib_id && self.find_fbx_id(attrib_id).is_none() {
                    attrib.fbx_id = attrib_id;
                }
            }
        }
        self.insert_fbx_attr(info.fbx_id, attrib.fbx_id);

        // The node keeps its own properties and the user's, the attribute the rest
        let mut node_props = Vec::new();
        let mut attrib_props = Vec::new();
        for p in std::mem::take(&mut info.props.props) {
            if !is_node_property_name(&p.name) && p.flags & prop_flags::USER_DEFINED == 0 {
                attrib_props.push(p);
            } else {
                node_props.push(p);
            }
        }
        info.props.props = node_props;
        attrib.props.props = attrib_props;

        match sub_type.as_str() {
            "Mesh" => {
                self.read_mesh(node, path, &attrib)?;
            }
            "Light" => {
                self.push_element(&attrib, ElementType::Light);
            }
            "Camera" => {
                self.push_element(&attrib, ElementType::Camera);
            }
            "LimbNode" | "Limb" | "Root" => self.read_bone(&attrib, &sub_type),
            "Null" | "Marker" => {
                self.push_element(&attrib, ElementType::Empty);
            }
            "NurbsCurve" => {
                if node.find_child("KnotVector").is_none() {
                    return Ok(());
                }
                self.read_nurbs_curve(node, &attrib)?;
            }
            "NurbsSurface" => {
                if node.find_child("KnotVectorU").is_none() || node.find_child("KnotVectorV").is_none() {
                    return Ok(());
                }
                self.read_nurbs_surface(node, &attrib)?;
            }
            "Line" => {
                if node.find_child("Points").is_none() || node.find_child("PointsIndex").is_none() {
                    return Ok(());
                }
                self.read_line(node, &attrib)?;
            }
            "TrimNurbsSurface" => {
                if node.find_child("Layer").is_none() {
                    return Ok(());
                }
                self.push_element(&attrib, ElementType::NurbsTrimSurface);
            }
            "Boundary" => {
                self.push_element(&attrib, ElementType::NurbsTrimBoundary);
            }
            "CameraStereo" => {
                self.push_element(&attrib, ElementType::StereoCamera);
            }
            "CameraSwitcher" => {
                self.push_element(&attrib, ElementType::CameraSwitcher);
            }
            "FKEffector" => self.read_marker(&attrib, MarkerType::FkEffector),
            "IKEffector" => self.read_marker(&attrib, MarkerType::IkEffector),
            "LodGroup" => {
                self.push_element(&attrib, ElementType::LodGroup);
            }
            _ => self.read_unknown(&attrib, type_str, sub_raw, super_type.as_bytes())?,
        }
        self.connect_oo(attrib.fbx_id, info.fbx_id);
        Ok(())
    }

    pub(crate) fn read_object(&mut self, node: &PNode, path: &[u32]) -> Res<()> {
        let mut info = ElementInfo { dom_node: self.get_dom_node(path), ..Default::default() };
        if node.name == "GlobalSettings" {
            // `ufbxi_read_global_settings` in ufbx.c
            self.scene.settings.props = self.read_properties(node)?;
            return Ok(());
        }
        let type_and_name: Vec<u8>;
        let mut sub_type: Vec<u8>;
        if self.version() >= 7000 {
            let (Some(id), Some(tn), Some(st)) = (node.i64_at(0), node.raw_at(1), node.raw_at(2)) else { return Ok(()) };
            info.fbx_id = self.validate_fbx_id(id as u64);
            type_and_name = tn.to_vec();
            sub_type = st.to_vec();
        } else {
            let (Some(tn), Some(st)) = (node.raw_at(0), node.raw_at(1)) else { return Ok(()) };
            type_and_name = tn.to_vec();
            sub_type = st.to_vec();
            info.fbx_id = self.synthetic_id_from_string(&type_and_name);
        }
        // ufbx interns, and repairs, a sub-type again only where it drops its "Fbx": any other is raw
        if sub_type.len() > 3 && sub_type.starts_with(b"Fbx") {
            sub_type = self.p.utf8(&sub_type[3..])?.into_bytes();
        }
        let sub_raw = sub_type;
        let sub_type = String::from_utf8_lossy(&sub_raw).into_owned();
        let node_name = node.raw_name.as_deref().unwrap_or(node.name.as_bytes()).to_vec();
        let node_name = c_str(&node_name);
        let (type_str, name) = self.split_type_and_name(&type_and_name)?;
        info.name = name;
        let name = &*node.name;
        info.props = self.read_properties(node)?;
        info.props.defaults = self.find_template(name, &sub_type);

        match name {
            "Model" => {
                if self.version() < 7000 {
                    self.read_synthetic_attribute(node, path, &mut info, &type_str, &sub_raw, name)?;
                }
                self.read_model(&info)?;
            }
            "NodeAttribute" => match sub_type.as_str() {
                "Light" => {
                    self.push_element(&info, ElementType::Light);
                }
                "Camera" => {
                    self.push_element(&info, ElementType::Camera);
                }
                "LimbNode" | "Limb" | "Root" => self.read_bone(&info, &sub_type),
                "Null" | "Marker" => {
                    self.push_element(&info, ElementType::Empty);
                }
                "CameraStereo" => {
                    self.push_element(&info, ElementType::StereoCamera);
                }
                "CameraSwitcher" => {
                    self.push_element(&info, ElementType::CameraSwitcher);
                }
                "FKEffector" => self.read_marker(&info, MarkerType::FkEffector),
                "IKEffector" => self.read_marker(&info, MarkerType::IkEffector),
                "LodGroup" => {
                    self.push_element(&info, ElementType::LodGroup);
                }
                _ => self.read_unknown(&info, &type_str, &sub_raw, node_name)?,
            },
            "Geometry" => match sub_type.as_str() {
                "Mesh" => {
                    self.read_mesh(node, path, &info)?;
                }
                "Shape" => self.read_shape(node, &info)?,
                "NurbsCurve" => self.read_nurbs_curve(node, &info)?,
                "NurbsSurface" => self.read_nurbs_surface(node, &info)?,
                "Line" => self.read_line(node, &info)?,
                "TrimNurbsSurface" => {
                    self.push_element(&info, ElementType::NurbsTrimSurface);
                }
                "Boundary" => {
                    self.push_element(&info, ElementType::NurbsTrimBoundary);
                }
                _ => self.read_unknown(&info, &type_str, &sub_raw, node_name)?,
            },
            "Deformer" => match sub_type.as_str() {
                "Skin" => self.read_skin(node, &info),
                "Cluster" => self.read_skin_cluster(node, &info)?,
                "BlendShape" => {
                    self.push_element(&info, ElementType::BlendDeformer);
                }
                "BlendShapeChannel" => self.read_blend_channel(node, &info),
                "VertexCacheDeformer" => {
                    self.push_element(&info, ElementType::CacheDeformer);
                }
                _ => self.read_unknown(&info, &type_str, &sub_raw, node_name)?,
            },
            "Material" => self.read_material(node, &info),
            "Texture" => self.read_texture(node, &info),
            "LayeredTexture" => self.read_layered_texture(node, &info),
            "Video" => self.read_video(node, &info),
            "AnimationStack" => self.read_anim_stack(&info),
            "AnimationLayer" => {
                self.push_element(&info, ElementType::AnimLayer);
            }
            "AnimationCurveNode" => {
                self.push_element(&info, ElementType::AnimValue);
            }
            "AnimationCurve" => self.read_animation_curve(node, &info)?,
            "Pose" => self.read_pose(node, &info, &sub_type)?,
            "Implementation" => {
                self.push_element(&info, ElementType::Shader);
            }
            "BindingTable" => self.read_binding_table(node, &info),
            "Collection" => {
                if sub_type == "SelectionSet" {
                    // `ufbxi_read_selection_set` in ufbx.c
                    self.push_element(&info, ElementType::SelectionSet);
                }
            }
            "CollectionExclusive" => {
                if sub_type == "DisplayLayer" {
                    self.push_element(&info, ElementType::DisplayLayer);
                }
            }
            "SelectionNode" => self.read_selection_node(node, &info),
            "Constraint" => {
                if sub_type == "Character" {
                    // `ufbxi_read_character` in ufbx.c
                    self.push_element(&info, ElementType::Character);
                } else {
                    self.read_constraint(node, &info);
                }
            }
            "SceneInfo" => self.read_scene_info(node)?,
            "Cache" => {
                self.push_element(&info, ElementType::CacheFile);
            }
            "ObjectMetaData" => {
                self.push_element(&info, ElementType::MetadataObject);
            }
            "AudioLayer" => {
                self.push_element(&info, ElementType::AudioLayer);
            }
            "Audio" => self.read_audio_clip(node, &info),
            _ => self.read_unknown(&info, &type_str, &sub_raw, node_name)?,
        }
        Ok(())
    }

    /// As in ufbx, the slot past the last child stays the one warnings and new elements are tagged with
    pub(crate) fn read_objects(&mut self, top_index: usize) -> Res<()> {
        let lazy = self.p.is_lazy(top_index);
        let mut ci = 0usize;
        loop {
            self.tmp_element_id.push(NO_INDEX);
            let slot = self.tmp_element_id.len() - 1;
            self.reading_element = Some(slot);
            self.p.warnings.deferred_element_id_plus_one = self.tmp_element_id.len() as u32;
            let node = if lazy { self.p.next_lazy_child()? } else { self.p.top_nodes[top_index].children.get_mut(ci).map(std::mem::take) };
            let Some(node) = node else { break };
            let result = self.read_object(&node, &[top_index as u32, ci as u32]);
            if lazy {
                self.p.keep_child(top_index, node);
            } else {
                self.p.top_nodes[top_index].children[ci] = node;
            }
            result?;
            ci += 1;
            self.p.warnings.deferred_element_id_plus_one = 0;
            self.reading_element = None;
        }
        Ok(())
    }

    pub(crate) fn read_connections(&mut self, children: &mut TopChildren<'a>) -> Res<()> {
        while let Some(node) = self.parse_toplevel_child(children)? {
            let node = &node;
            let (src, dst, src_prop, dst_prop);
            if self.version() < 7000 {
                let Some(t) = node.raw_at(0) else { continue };
                let t = t.to_vec();
                let s = |i: usize| node.raw_at(i).map(|b| b.to_vec());
                let (src_name, dst_name, sp, dp): (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) = match t.as_slice() {
                    b"OO" => {
                        let (Some(a), Some(b)) = (s(1), s(2)) else { continue };
                        (a, b, Vec::new(), Vec::new())
                    }
                    b"OP" => {
                        let (Some(a), Some(b), Some(d)) = (s(1), s(2), s(3)) else { continue };
                        (a, b, Vec::new(), d)
                    }
                    b"PO" => {
                        let (Some(a), Some(sp), Some(b)) = (s(1), s(2), s(3)) else { continue };
                        (a, b, sp, Vec::new())
                    }
                    b"PP" => {
                        let (Some(a), Some(sp), Some(b), Some(dp)) = (s(1), s(2), s(3), s(4)) else { continue };
                        (a, b, sp, dp)
                    }
                    _ => continue,
                };
                src_prop = if sp.is_empty() { String::new() } else { self.p.utf8(&sp)? };
                dst_prop = if dp.is_empty() { String::new() } else { self.p.utf8(&dp)? };
                src = self.synthetic_id_from_string(&src_name);
                dst = self.synthetic_id_from_string(&dst_name);
            } else {
                let Some(t) = node.str_at(0) else { continue };
                let l = |i: usize| node.i64_at(i).map(|v| v as u64);
                let st = |i: usize| node.str_at(i).map(|s| s.to_string());
                let (a, b, sp, dp) = match t {
                    "OO" => {
                        let (Some(a), Some(b)) = (l(1), l(2)) else { continue };
                        (a, b, String::new(), String::new())
                    }
                    "OP" => {
                        let (Some(a), Some(b), Some(d)) = (l(1), l(2), st(3)) else { continue };
                        (a, b, String::new(), d)
                    }
                    "PO" => {
                        let (Some(a), Some(sp), Some(b)) = (l(1), st(2), l(3)) else { continue };
                        (a, b, sp, String::new())
                    }
                    "PP" => {
                        let (Some(a), Some(sp), Some(b), Some(dp)) = (l(1), st(2), l(3), st(4)) else { continue };
                        (a, b, sp, dp)
                    }
                    _ => continue,
                };
                src = self.validate_fbx_id(a);
                dst = self.validate_fbx_id(b);
                src_prop = sp;
                dst_prop = dp;
            }
            self.tmp_connections.push(crate::context::TmpConnection { src, dst, src_prop, dst_prop });
        }
        Ok(())
    }
}

pub(crate) fn child_path(base: &Option<DomRef>, parent: &PNode, child: &str) -> Option<DomRef> {
    let ci = parent.children.iter().position(|c| c.name == child)?;
    base.as_ref().map(|b| [b.as_slice(), &[ci as u32]].concat())
}

impl Context<'_> {
    fn warn_polygon_mapping(&mut self, data_name: &str, mapping: &[u8]) {
        let text = [format!("Ignoring geometry '{data_name}' with bad mapping mode '").as_bytes(), mapping, b"'"].concat();
        self.warnf(WarningType::MissingPolygonMapping, text);
    }
}

/// A raw string as ufbx's `%s` prints it: up to its first NUL
fn c_bytes(n: &PNode, child: &str) -> Vec<u8> {
    c_str(n.find_child(child).and_then(|c| c.raw_at(0)).unwrap_or(b"")).to_vec()
}

fn c_str(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())]
}

fn raw_str(n: &PNode, child: &str) -> String {
    n.find_child(child).and_then(|c| c.raw_at(0)).map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default()
}

fn read_nurbs_topology(form: &str) -> NurbsTopology {
    match form {
        "Closed" => NurbsTopology::Closed,
        "Periodic" => NurbsTopology::Periodic,
        _ => NurbsTopology::Open,
    }
}

pub(crate) fn read_extrapolation(node: &PNode, name: &str) -> Extrapolation {
    let mut mode = ExtrapolationMode::Constant;
    let mut repeat_count = -1;
    if let Some(child) = node.find_child(name)
        && let Some(m) = child.find_i32("Type")
    {
        const A: i32 = b'A' as i32;
        const C: i32 = b'C' as i32;
        const K: i32 = b'K' as i32;
        const M: i32 = b'M' as i32;
        const R: i32 = b'R' as i32;
        match m {
            A => mode = ExtrapolationMode::RepeatRelative,
            C => mode = ExtrapolationMode::Constant,
            K => mode = ExtrapolationMode::Slope,
            M => mode = ExtrapolationMode::Mirror,
            R => mode = ExtrapolationMode::Repeat,
            _ => {}
        }
        if let Some(r) = child.find_i32("Repetition") {
            repeat_count = if r < 0 { -1 } else { r };
        }
    }
    Extrapolation { mode, repeat_count }
}

fn solve_tcb(tension: f64, continuity: f64, bias: f64, slope_left: f64, slope_right: f64, edge: bool) -> (f32, f32) {
    let factor = if edge { 1.0 } else { 0.5 };
    let d00 = factor * (1.0 - tension) * (1.0 + bias) * (1.0 - continuity);
    let d01 = factor * (1.0 - tension) * (1.0 - bias) * (1.0 + continuity);
    let d10 = factor * (1.0 - tension) * (1.0 + bias) * (1.0 + continuity);
    let d11 = factor * (1.0 - tension) * (1.0 - bias) * (1.0 - continuity);
    ((d00 * slope_left + d01 * slope_right) as f32, (d10 * slope_left + d11 * slope_right) as f32)
}

pub(crate) fn assign_face_groups(mesh: &mut Mesh, retain_parts: bool) -> Res<()> {
    let num_faces = mesh.num_faces;
    check!(num_faces > 0);
    check!(num_faces < u32::MAX as usize);
    check!(mesh.face_group.len() == num_faces);
    let mut ids: Vec<i32> = mesh.face_group.iter().map(|&i| i as i32).collect();
    // `ufbxi_less_int32` in ufbx.c
    ids.sort_unstable();
    ids.dedup();
    mesh.face_groups = ids.iter().map(|&id| FaceGroup { id, name: String::new() }).collect();
    let num_groups = ids.len();
    for g in mesh.face_group.iter_mut() {
        *g = ids.binary_search(&(*g as i32)).unwrap_or(0) as u32;
    }
    if retain_parts {
        let mut parts: Vec<MeshPart> = (0..num_groups).map(|i| MeshPart { index: i as u32, ..Default::default() }).collect();
        for (f, &g) in mesh.face_group.iter().enumerate() {
            let part = &mut parts[g as usize];
            mesh_part_add_face(part, mesh.faces[f].num_indices);
            part.face_indices.push(f as u32);
        }
        mesh.face_group_parts = parts;
    }
    Ok(())
}

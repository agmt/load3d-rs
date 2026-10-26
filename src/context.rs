use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{Error, Res, WarningType, check, fail_msg};
use crate::parse::{PNode, Parser};
use crate::types::*;

pub(crate) const POINTER_ID_START: u64 = 0x8000_0000_0000_0000;
pub(crate) const MAXIMUM_FAST_POINTER_ID: u64 = 0x4000_0000_0000_0000;
pub(crate) const SYNTHETIC_ID_START: u64 = POINTER_ID_START + MAXIMUM_FAST_POINTER_ID;

/// Calls `$m!(Type, field)` for every element type and its list in `Scene`
#[macro_export]
#[doc(hidden)]
macro_rules! for_each_element_list {
    ($m:ident) => {
        $m! {
            Unknown => Unknown, unknowns;
            Node => Node, nodes;
            Mesh => Mesh, meshes;
            Light => Light, lights;
            Camera => Camera, cameras;
            Bone => Bone, bones;
            Empty => Empty, empties;
            LineCurve => LineCurve, line_curves;
            NurbsCurve => NurbsCurve, nurbs_curves;
            NurbsSurface => NurbsSurface, nurbs_surfaces;
            NurbsTrimSurface => NurbsTrimSurface, nurbs_trim_surfaces;
            NurbsTrimBoundary => NurbsTrimBoundary, nurbs_trim_boundaries;
            ProceduralGeometry => ProceduralGeometry, procedural_geometries;
            StereoCamera => StereoCamera, stereo_cameras;
            CameraSwitcher => CameraSwitcher, camera_switchers;
            Marker => Marker, markers;
            LodGroup => LodGroup, lod_groups;
            SkinDeformer => SkinDeformer, skin_deformers;
            SkinCluster => SkinCluster, skin_clusters;
            BlendDeformer => BlendDeformer, blend_deformers;
            BlendChannel => BlendChannel, blend_channels;
            BlendShape => BlendShape, blend_shapes;
            CacheDeformer => CacheDeformer, cache_deformers;
            CacheFile => CacheFile, cache_files;
            Material => Material, materials;
            Texture => Texture, textures;
            Video => Video, videos;
            Shader => Shader, shaders;
            ShaderBinding => ShaderBinding, shader_bindings;
            AnimStack => AnimStack, anim_stacks;
            AnimLayer => AnimLayer, anim_layers;
            AnimValue => AnimValue, anim_values;
            AnimCurve => AnimCurve, anim_curves;
            DisplayLayer => DisplayLayer, display_layers;
            SelectionSet => SelectionSet, selection_sets;
            SelectionNode => SelectionNode, selection_nodes;
            Character => Character, characters;
            Constraint => Constraint, constraints;
            AudioLayer => AudioLayer, audio_layers;
            AudioClip => AudioClip, audio_clips;
            Pose => Pose, poses;
            MetadataObject => MetadataObject, metadata_objects;
        }
    };
}

macro_rules! impl_element_access {
    ($($variant:ident => $ty:ident, $field:ident;)*) => {
        impl Scene {
            pub fn element(&self, id: ElementId) -> &Element {
                let r = self.elements[id as usize];
                self.element_at(r)
            }

            pub fn element_at(&self, r: ElementRef) -> &Element {
                match r.element_type {
                    $(ElementType::$variant => &self.$field[r.index].element,)*
                }
            }

            pub fn element_mut(&mut self, id: ElementId) -> &mut Element {
                let r = self.elements[id as usize];
                self.element_at_mut(r)
            }

            pub fn element_at_mut(&mut self, r: ElementRef) -> &mut Element {
                match r.element_type {
                    $(ElementType::$variant => &mut self.$field[r.index].element,)*
                }
            }

            pub fn count_of(&self, t: ElementType) -> usize {
                match t {
                    $(ElementType::$variant => self.$field.len(),)*
                }
            }

            pub fn elements_by_type(&self, t: ElementType) -> Vec<ElementId> {
                match t {
                    $(ElementType::$variant => self.$field.iter().map(|e| e.element.element_id).collect(),)*
                }
            }
        }

        pub(crate) fn push_typed(scene: &mut Scene, t: ElementType, element: Element) -> usize {
            match t {
                $(ElementType::$variant => {
                    scene.$field.push($ty { element, ..Default::default() });
                    scene.$field.len() - 1
                })*
            }
        }
    };
}

for_each_element_list!(impl_element_access);

#[derive(Debug, Clone, Default)]
pub(crate) struct ElementInfo {
    pub fbx_id: u64,
    pub name: String,
    pub props: Props,
    pub dom_node: Option<DomRef>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TmpConnection {
    pub src: u64,
    pub dst: u64,
    pub src_prop: String,
    pub dst_prop: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Template {
    pub type_: String,
    pub sub_type: String,
    pub props: Arc<Props>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FbxIdEntry {
    pub element_id: u32,
    #[cfg_attr(not(feature = "format-obj"), allow(dead_code))]
    pub user_id: u32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TmpMeshTexture {
    pub prop_name: String,
    pub face_texture: Vec<u32>,
    pub all_same: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TmpBonePose {
    pub bone_fbx_id: u64,
    pub bone_to_world: Matrix,
}

/// `ufbxi_node_extra`, `ufbxi_mesh_extra` and `ufbxi_texture_extra` in ufbx.c, and the pose's bones before they are resolved
#[derive(Debug, Clone, Default)]
pub(crate) struct Extra {
    pub geometry_helper_id: Option<u32>,
    pub scale_helper_id: Option<u32>,
    pub mesh_textures: Vec<TmpMeshTexture>,
    pub blend_modes: Option<Vec<i32>>,
    pub alphas: Option<Vec<Real>>,
    pub bone_poses: Vec<TmpBonePose>,
}

/// A scene list ufbx keeps in the parsed array itself, so that its DOM shows the list as the load leaves it
#[derive(Debug, Clone, Copy)]
pub(crate) enum DomAlias {
    MeshVertices(usize),
    NormalValues(usize),
    NormalIndices(usize),
    /// A mesh and the UV set's layer number
    UvValues(usize, u32),
    UvIndices(usize, u32),
    ColorValues(usize, u32),
    ColorIndices(usize, u32),
    /// A UV set's tangents and bitangents, by the set's layer number
    TangentValues(usize, u32),
    TangentIndices(usize, u32),
    BitangentValues(usize, u32),
    BitangentIndices(usize, u32),
    FaceMaterial(usize),
    FaceGroup(usize),
    LinePoints(usize),
    LineIndices(usize),
    ShapeVertices(usize),
    ShapeIndices(usize),
    ShapeNormals(usize),
    NurbsCurvePoints(usize),
    NurbsSurfacePoints(usize),
    SelectionVertices(usize),
    SelectionEdges(usize),
    SelectionFaces(usize),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FileContent {
    pub absolute_filename: String,
    pub content: Vec<u8>,
}

pub(crate) struct Context<'a> {
    pub opts: LoadOpts,
    pub p: Parser<'a>,
    pub scene: Scene,
    pub exporter: Exporter,
    pub exporter_version: u32,
    pub retain_mesh_parts: bool,
    pub read_legacy_settings: bool,
    pub templates: Vec<Template>,
    pub fbx_id_map: HashMap<u64, FbxIdEntry>,
    pub ptr_fbx_id_map: HashMap<(u64, u64), u64>,
    pub string_ids: HashMap<Vec<u8>, u64>,
    pub fbx_attr_map: HashMap<u64, u64>,
    pub anim_stack_map: HashMap<String, usize>,
    pub tmp_connections: Vec<TmpConnection>,
    pub tmp_node_ids: Vec<u32>,
    pub element_fbx_ids: Vec<u64>,
    pub extra: HashMap<u32, Extra>,
    pub tmp_full_weights: Vec<Vec<Real>>,
    pub synthetic_id_counter: u64,
    pub has_geometry_transform_nodes: bool,
    pub has_scale_helper_nodes: bool,
    pub mirror_axis: MirrorAxis,
    pub root_id: u64,
    pub legacy_implicit_anim_layer_id: u64,
    pub file_content: Vec<FileContent>,
    pub ktime_sec: i64,
    pub ktime_sec_double: f64,
    pub axis_matrix: Matrix,
    pub unit_scale: Real,
    pub tmp_element_id: Vec<u32>,
    pub reading_element: Option<usize>,
    pub tmp_element_flag: Vec<bool>,
    pub dom_aliases: Vec<(DomRef, DomAlias)>,
    /// An .obj's meshes that share one list of positions or normals in ufbx, whose in-place edits change it once per mesh
    pub shared_positions: Vec<u32>,
    pub shared_normals: Vec<u32>,
}

pub(crate) struct TopChildren {
    pub ix: u32,
    pub lazy: bool,
    pub parsed: std::vec::IntoIter<PNode>,
    pub read: u32,
}

impl TopChildren {
    pub fn parsed(ix: u32, children: Vec<PNode>) -> TopChildren {
        TopChildren { ix, lazy: false, parsed: children.into_iter(), read: 0 }
    }
}

impl<'a> Context<'a> {
    pub fn warnf(&mut self, kind: WarningType, description: impl Into<Vec<u8>>) {
        self.p.warnings.warn(kind, NO_INDEX, description);
    }

    pub fn warnf_tag(&mut self, kind: WarningType, element_id: u32, description: impl Into<Vec<u8>>) {
        self.p.warnings.warn(kind, element_id, description);
    }

    pub fn from_ascii(&self) -> bool {
        self.p.from_ascii
    }

    pub fn version(&self) -> u32 {
        self.p.version
    }

    pub fn push_synthetic_id(&mut self) -> u64 {
        self.synthetic_id_counter += 1;
        self.synthetic_id_counter
    }

    pub fn synthetic_id_from_ptr_id(&mut self, ptr: u64, id: u64) -> u64 {
        if let Some(&v) = self.ptr_fbx_id_map.get(&(ptr, id)) {
            return v;
        }
        let v = self.push_synthetic_id();
        self.ptr_fbx_id_map.insert((ptr, id), v);
        v
    }

    /// ufbx uses the pooled string's address, so equal strings give equal ids; here each distinct string gets the next id
    /// below the range of pointer ids
    pub fn synthetic_id_from_string(&mut self, s: &[u8]) -> u64 {
        if let Some(&v) = self.string_ids.get(s) {
            return v;
        }
        let v = 0x1000_0000_0000_0000 + self.string_ids.len() as u64 + 1;
        self.string_ids.insert(s.to_vec(), v);
        v
    }

    pub fn validate_fbx_id(&mut self, id: u64) -> u64 {
        if id >= POINTER_ID_START { self.synthetic_id_from_ptr_id(0, id) } else { id }
    }

    fn insert_fbx_id(&mut self, fbx_id: u64, element_id: u32) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.fbx_id_map.entry(fbx_id) {
            e.insert(FbxIdEntry { element_id, user_id: 0 });
        } else {
            self.warnf(WarningType::DuplicateObjectId, "Duplicate object ID");
        }
    }

    pub fn find_fbx_id(&self, fbx_id: u64) -> Option<FbxIdEntry> {
        self.fbx_id_map.get(&fbx_id).copied()
    }

    pub fn insert_fbx_attr(&mut self, fbx_id: u64, attrib_fbx_id: u64) {
        self.fbx_attr_map.entry(fbx_id).or_insert(attrib_fbx_id);
    }

    /// With `ufbxi_push_element_size` in ufbx.c
    pub fn push_element(&mut self, info: &ElementInfo, t: ElementType) -> usize {
        let element_id = self.scene.elements.len() as u32;
        let typed_id = self.scene.count_of(t) as u32;
        let element = Element {
            name: info.name.clone(),
            props: info.props.clone(),
            element_id,
            typed_id,
            element_type: t,
            dom_node: info.dom_node.clone(),
            ..Default::default()
        };
        let index = push_typed(&mut self.scene, t, element);
        self.scene.elements.push(ElementRef { element_type: t, index });
        self.element_fbx_ids.push(info.fbx_id);
        if let Some(slot) = self.reading_element {
            self.tmp_element_id[slot] = element_id;
        }
        self.insert_fbx_id(info.fbx_id, element_id);
        index
    }

    /// With `ufbxi_push_synthetic_element_size` in ufbx.c
    pub fn push_synthetic_element(&mut self, dom_node: Option<DomRef>, name: Option<&str>, t: ElementType) -> (usize, u64) {
        let element_id = self.scene.elements.len() as u32;
        let typed_id = self.scene.count_of(t) as u32;
        let element = Element { name: name.unwrap_or("").to_string(), element_id, typed_id, element_type: t, dom_node, ..Default::default() };
        let index = push_typed(&mut self.scene, t, element);
        self.scene.elements.push(ElementRef { element_type: t, index });
        let fbx_id = self.push_synthetic_id();
        self.element_fbx_ids.push(fbx_id);
        self.insert_fbx_id(fbx_id, element_id);
        (index, fbx_id)
    }

    pub fn connect_oo(&mut self, src: u64, dst: u64) {
        self.tmp_connections.push(TmpConnection { src, dst, ..Default::default() });
    }

    pub fn connect_op(&mut self, src: u64, dst: u64, prop: &str) {
        self.tmp_connections.push(TmpConnection { src, dst, src_prop: String::new(), dst_prop: prop.to_string() });
    }

    pub fn connect_pp(&mut self, src: u64, dst: u64, src_prop: &str, dst_prop: &str) {
        self.tmp_connections.push(TmpConnection { src, dst, src_prop: src_prop.to_string(), dst_prop: dst_prop.to_string() });
    }

    /// With `ufbxi_push_element_extra_size` in ufbx.c
    pub fn push_element_extra(&mut self, element_id: u32) -> &mut Extra {
        self.extra.entry(element_id).or_default()
    }

    pub fn get_element_extra(&self, element_id: u32) -> Option<&Extra> {
        self.extra.get(&element_id)
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

    pub fn fix_index(&mut self, index: u32, one_past_max: usize) -> Res<u32> {
        match self.opts.index_error_handling {
            IndexErrorHandling::Clamp => {
                check!(one_past_max > 0);
                check!(one_past_max <= u32::MAX as usize);
                self.warnf(WarningType::IndexClamped, "Clamped index");
                Ok(one_past_max as u32 - 1)
            }
            IndexErrorHandling::NoIndex => Ok(NO_INDEX),
            IndexErrorHandling::AbortLoading => {
                let max = if one_past_max > 0 { one_past_max - 1 } else { 0 };
                Err(Error::new("Bad index").set_err_info(format!("{index} (max {max})")))
            }
            IndexErrorHandling::UnsafeIgnore => Ok(index),
        }
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
}

//! ufbx's pointers between elements are indices here: a field typed `usize` (or `Option<usize>`, `Vec<usize>`) indexes the
//! scene's list of that element type, `ElementId` indexes `Scene::elements`.

use std::sync::Arc;

use crate::error::{Warning, WarningType};
pub use crate::math::{CoordinateAxes, CoordinateAxis, Matrix, Quat, Real, RotationOrder, Transform, Vec2, Vec3, Vec4};

pub type ElementId = u32;
pub const NO_INDEX: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DomValueType {
    #[default]
    Number,
    String,
    Blob,
    ArrayI32,
    ArrayI64,
    ArrayF32,
    ArrayF64,
    ArrayBlob,
    ArrayIgnored,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomArray {
    U8(Vec<u8>),
    Bool(Vec<bool>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Blob(Vec<Vec<u8>>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DomValue {
    pub value_type: DomValueType,
    pub value_str: String,
    pub value_blob: Vec<u8>,
    pub value_int: i64,
    pub value_float: f64,
    pub array: Option<DomArray>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DomNode {
    pub name: String,
    pub children: Vec<DomNode>,
    pub values: Vec<DomValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PropType {
    #[default]
    Unknown,
    Boolean,
    Integer,
    Number,
    Vector,
    Color,
    ColorWithAlpha,
    String,
    DateTime,
    Translation,
    Rotation,
    Scaling,
    Distance,
    Compound,
    Blob,
    Reference,
}

pub mod prop_flags {
    pub const ANIMATABLE: u32 = 0x1;
    pub const USER_DEFINED: u32 = 0x2;
    pub const HIDDEN: u32 = 0x4;
    pub const LOCK_X: u32 = 0x10;
    pub const LOCK_Y: u32 = 0x20;
    pub const LOCK_Z: u32 = 0x40;
    pub const LOCK_W: u32 = 0x80;
    pub const MUTE_X: u32 = 0x100;
    pub const MUTE_Y: u32 = 0x200;
    pub const MUTE_Z: u32 = 0x400;
    pub const MUTE_W: u32 = 0x800;
    pub const SYNTHETIC: u32 = 0x1000;
    pub const ANIMATED: u32 = 0x2000;
    pub const NOT_FOUND: u32 = 0x4000;
    pub const CONNECTED: u32 = 0x8000;
    pub const NO_VALUE: u32 = 0x10000;
    pub const OVERRIDDEN: u32 = 0x20000;
    pub const VALUE_REAL: u32 = 0x100000;
    pub const VALUE_VEC2: u32 = 0x200000;
    pub const VALUE_VEC3: u32 = 0x400000;
    pub const VALUE_VEC4: u32 = 0x800000;
    pub const VALUE_INT: u32 = 0x1000000;
    pub const VALUE_STR: u32 = 0x2000000;
    pub const VALUE_BLOB: u32 = 0x4000000;
}

/// Every value field is filled whatever the type, `value_real` and `value_int` of an integer alike
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Prop {
    pub name: String,
    pub internal_key: u32,
    pub prop_type: PropType,
    pub flags: u32,
    pub value_str: String,
    pub value_blob: Vec<u8>,
    pub value_int: i64,
    pub value_real_arr: [Real; 4],
}

impl Prop {
    pub fn value_real(&self) -> Real {
        self.value_real_arr[0]
    }
    pub fn value_vec2(&self) -> Vec2 {
        Vec2::new(self.value_real_arr[0], self.value_real_arr[1])
    }
    pub fn value_vec3(&self) -> Vec3 {
        Vec3::new(self.value_real_arr[0], self.value_real_arr[1], self.value_real_arr[2])
    }
    pub fn value_vec4(&self) -> Vec4 {
        let a = self.value_real_arr;
        Vec4::new(a[0], a[1], a[2], a[3])
    }
}

/// Sorted by `internal_key` and name, with the template's defaults behind
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Props {
    pub props: Vec<Prop>,
    pub num_animated: usize,
    pub defaults: Option<Arc<Props>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum ElementType {
    #[default]
    Unknown,
    Node,
    Mesh,
    Light,
    Camera,
    Bone,
    Empty,
    LineCurve,
    NurbsCurve,
    NurbsSurface,
    NurbsTrimSurface,
    NurbsTrimBoundary,
    ProceduralGeometry,
    StereoCamera,
    CameraSwitcher,
    Marker,
    LodGroup,
    SkinDeformer,
    SkinCluster,
    BlendDeformer,
    BlendChannel,
    BlendShape,
    CacheDeformer,
    CacheFile,
    Material,
    Texture,
    Video,
    Shader,
    ShaderBinding,
    AnimStack,
    AnimLayer,
    AnimValue,
    AnimCurve,
    DisplayLayer,
    SelectionSet,
    SelectionNode,
    Character,
    Constraint,
    AudioLayer,
    AudioClip,
    Pose,
    MetadataObject,
}

impl ElementType {
    pub const COUNT: usize = 42;
    pub const FIRST_ATTRIB: ElementType = ElementType::Mesh;
    pub const LAST_ATTRIB: ElementType = ElementType::LodGroup;

    pub fn from_index(i: usize) -> ElementType {
        ALL_ELEMENT_TYPES.get(i).copied().unwrap_or(ElementType::Unknown)
    }

    pub fn is_attrib(self) -> bool {
        self >= ElementType::FIRST_ATTRIB && self <= ElementType::LAST_ATTRIB
    }
}

pub const ALL_ELEMENT_TYPES: [ElementType; ElementType::COUNT] = [
    ElementType::Unknown,
    ElementType::Node,
    ElementType::Mesh,
    ElementType::Light,
    ElementType::Camera,
    ElementType::Bone,
    ElementType::Empty,
    ElementType::LineCurve,
    ElementType::NurbsCurve,
    ElementType::NurbsSurface,
    ElementType::NurbsTrimSurface,
    ElementType::NurbsTrimBoundary,
    ElementType::ProceduralGeometry,
    ElementType::StereoCamera,
    ElementType::CameraSwitcher,
    ElementType::Marker,
    ElementType::LodGroup,
    ElementType::SkinDeformer,
    ElementType::SkinCluster,
    ElementType::BlendDeformer,
    ElementType::BlendChannel,
    ElementType::BlendShape,
    ElementType::CacheDeformer,
    ElementType::CacheFile,
    ElementType::Material,
    ElementType::Texture,
    ElementType::Video,
    ElementType::Shader,
    ElementType::ShaderBinding,
    ElementType::AnimStack,
    ElementType::AnimLayer,
    ElementType::AnimValue,
    ElementType::AnimCurve,
    ElementType::DisplayLayer,
    ElementType::SelectionSet,
    ElementType::SelectionNode,
    ElementType::Character,
    ElementType::Constraint,
    ElementType::AudioLayer,
    ElementType::AudioClip,
    ElementType::Pose,
    ElementType::MetadataObject,
];

/// The destination is usually the container, a parent node or the mesh of a deformer
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Connection {
    pub src: ElementId,
    pub dst: ElementId,
    pub src_prop: String,
    pub dst_prop: String,
}

/// A reference to a DOM node: the path of child indices from the scene's DOM root
pub type DomRef = Vec<u32>;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Element {
    pub name: String,
    pub props: Props,
    pub element_id: ElementId,
    pub typed_id: u32,
    pub instances: Vec<usize>,
    pub element_type: ElementType,
    pub connections_src: Vec<Connection>,
    pub connections_dst: Vec<Connection>,
    pub dom_node: Option<DomRef>,
}

/// `type` is "super_type: ID, \"type::name\", \"sub_type\"" in the ASCII format
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Unknown {
    pub element: Element,
    pub type_: String,
    pub super_type: String,
    pub sub_type: String,
}

/// How a node's scale combines with its parent's
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InheritMode {
    #[default]
    Normal,
    IgnoreParentScale,
    ComponentwiseScale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MirrorAxis {
    #[default]
    None,
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
    pub element: Element,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
    pub light: Option<usize>,
    pub camera: Option<usize>,
    pub bone: Option<usize>,
    pub attrib: Option<ElementId>,
    pub geometry_transform_helper: Option<usize>,
    pub scale_helper: Option<usize>,
    pub attrib_type: ElementType,
    pub all_attribs: Vec<ElementId>,
    pub inherit_mode: InheritMode,
    pub original_inherit_mode: InheritMode,
    pub local_transform: Transform,
    pub geometry_transform: Transform,
    pub inherit_scale: Vec3,
    pub inherit_scale_node: Option<usize>,
    pub rotation_order: RotationOrder,
    pub euler_rotation: Vec3,
    pub node_to_parent: Matrix,
    pub node_to_world: Matrix,
    pub geometry_to_node: Matrix,
    pub geometry_to_world: Matrix,
    pub unscaled_node_to_world: Matrix,
    pub adjust_pre_translation: Vec3,
    pub adjust_pre_rotation: Quat,
    pub adjust_pre_scale: Real,
    pub adjust_post_rotation: Quat,
    pub adjust_post_scale: Real,
    pub adjust_translation_scale: Real,
    pub adjust_mirror_axis: MirrorAxis,
    /// For this instance; `None` where broken
    pub materials: Vec<Option<usize>>,
    pub bind_pose: Option<usize>,
    pub visible: bool,
    pub is_root: bool,
    pub has_geometry_transform: bool,
    pub use_rotation_space: bool,
    pub has_adjust_transform: bool,
    pub has_root_adjust_transform: bool,
    pub is_geometry_transform_helper: bool,
    pub is_scale_helper: bool,
    pub is_scale_compensate_parent: bool,
    pub node_depth: u32,
}

/// The values and, per mesh index, which value it takes
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VertexAttrib<T> {
    pub exists: bool,
    pub values: Vec<T>,
    pub indices: Vec<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: Vec<Real>,
}

impl<T: Copy + Default> VertexAttrib<T> {
    /// `ufbx_get_vertex_real`, `ufbx_get_vertex_vec2`, `ufbx_get_vertex_vec3` and `ufbx_get_vertex_vec4` in ufbx.h:
    /// The default at `NO_INDEX`
    pub fn get(&self, index: usize) -> T {
        match self.indices.get(index) {
            Some(&ix) if (ix as usize) < self.values.len() => self.values[ix as usize],
            _ => T::default(),
        }
    }
}

pub type VertexReal = VertexAttrib<Real>;
pub type VertexVec2 = VertexAttrib<Vec2>;
pub type VertexVec3 = VertexAttrib<Vec3>;
pub type VertexVec4 = VertexAttrib<Vec4>;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UvSet {
    pub name: String,
    pub index: u32,
    pub vertex_uv: VertexVec2,
    pub vertex_tangent: VertexVec3,
    pub vertex_bitangent: VertexVec3,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorSet {
    pub name: String,
    pub index: u32,
    pub vertex_color: VertexVec4,
}

/// Between two mesh indices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Edge {
    pub a: u32,
    pub b: u32,
}

/// `num_indices` consecutive mesh indices from `index_begin`, an invalid face where fewer than three
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Face {
    pub index_begin: u32,
    pub num_indices: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeshPart {
    pub index: u32,
    pub num_faces: usize,
    pub num_triangles: usize,
    pub num_empty_faces: usize,
    pub num_point_faces: usize,
    pub num_line_faces: usize,
    pub face_indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FaceGroup {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SubdivisionWeightRange {
    pub weight_begin: u32,
    pub num_weights: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SubdivisionWeight {
    pub weight: Real,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SubdivisionResult {
    pub result_memory_used: usize,
    pub temp_memory_used: usize,
    pub result_allocs: usize,
    pub temp_allocs: usize,
    pub source_vertex_ranges: Vec<SubdivisionWeightRange>,
    pub source_vertex_weights: Vec<SubdivisionWeight>,
    pub skin_cluster_ranges: Vec<SubdivisionWeightRange>,
    pub skin_cluster_weights: Vec<SubdivisionWeight>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SubdivisionDisplayMode {
    #[default]
    Disabled,
    Hull,
    HullAndSmooth,
    Smooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SubdivisionBoundary {
    #[default]
    Default,
    Legacy,
    SharpCorners,
    SharpNone,
    SharpBoundary,
    SharpInterior,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    pub element: Element,
    pub num_vertices: usize,
    pub num_indices: usize,
    pub num_faces: usize,
    pub num_triangles: usize,
    pub num_edges: usize,
    pub max_face_triangles: usize,
    pub num_empty_faces: usize,
    pub num_point_faces: usize,
    pub num_line_faces: usize,
    pub faces: Vec<Face>,
    pub face_smoothing: Vec<bool>,
    /// Beyond ufbx: the smoothing groups of a polygon-mapped `LayerElementSmoothing`, the integers `face_smoothing` is
    /// made from (3ds Max writes its groups there as bits)
    pub face_smoothing_groups: Vec<u32>,
    pub face_material: Vec<u32>,
    pub face_group: Vec<u32>,
    pub face_hole: Vec<bool>,
    pub edges: Vec<Edge>,
    /// Beyond ufbx's lists: the file gave `Edges`, though it may hold none (ufbx's list then has data but no entries)
    pub has_edges: bool,
    pub edge_smoothing: Vec<bool>,
    /// The same for an edge-mapped `Smoothing`
    pub has_edge_smoothing: bool,
    pub edge_crease: Vec<Real>,
    pub edge_visibility: Vec<bool>,
    pub vertex_indices: Vec<u32>,
    pub vertices: Vec<Vec3>,
    pub vertex_first_index: Vec<u32>,
    pub vertex_position: VertexVec3,
    pub vertex_normal: VertexVec3,
    pub vertex_uv: VertexVec2,
    pub vertex_tangent: VertexVec3,
    pub vertex_bitangent: VertexVec3,
    pub vertex_color: VertexVec4,
    pub vertex_crease: VertexReal,
    pub uv_sets: Vec<UvSet>,
    pub color_sets: Vec<ColorSet>,
    /// `None` where the file's connection is broken
    pub materials: Vec<Option<usize>>,
    pub face_groups: Vec<FaceGroup>,
    pub material_parts: Vec<MeshPart>,
    pub face_group_parts: Vec<MeshPart>,
    pub material_part_usage_order: Vec<u32>,
    pub skinned_is_local: bool,
    pub skinned_position: VertexVec3,
    pub skinned_normal: VertexVec3,
    pub skin_deformers: Vec<usize>,
    pub blend_deformers: Vec<usize>,
    pub cache_deformers: Vec<usize>,
    pub all_deformers: Vec<ElementId>,
    pub subdivision_preview_levels: u32,
    pub subdivision_render_levels: u32,
    pub subdivision_display_mode: SubdivisionDisplayMode,
    pub subdivision_boundary: SubdivisionBoundary,
    pub subdivision_uv_boundary: SubdivisionBoundary,
    pub reversed_winding: bool,
    pub generated_normals: bool,
    pub subdivision_evaluated: bool,
    pub subdivision_result: Option<Box<SubdivisionResult>>,
    pub from_tessellated_nurbs: bool,
    /// Beyond ufbx: a 3DS object's mesh matrix (chunk 0x4160), its own axes and origin as it was built, in the file's
    /// axes and unit; its vertices are stored already placed
    pub mesh_matrix: Option<Matrix>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LightType {
    #[default]
    Point,
    Directional,
    Spot,
    Area,
    Volume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LightDecay {
    #[default]
    None,
    Linear,
    Quadratic,
    Cubic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LightAreaShape {
    #[default]
    Rectangle,
    Sphere,
}

/// `intensity` is a hundredth of the property "Intensity", as the programs that wrote it show it
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Light {
    pub element: Element,
    pub color: Vec3,
    pub intensity: Real,
    pub local_direction: Vec3,
    pub light_type: LightType,
    pub decay: LightDecay,
    pub area_shape: LightAreaShape,
    pub inner_angle: Real,
    pub outer_angle: Real,
    pub cast_light: bool,
    pub cast_shadows: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProjectionMode {
    #[default]
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AspectMode {
    #[default]
    WindowSize,
    FixedRatio,
    FixedResolution,
    FixedWidth,
    FixedHeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ApertureMode {
    #[default]
    HorizontalAndVertical,
    Horizontal,
    Vertical,
    FocalLength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GateFit {
    #[default]
    None,
    Vertical,
    Horizontal,
    Fill,
    Overscan,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ApertureFormat {
    #[default]
    Custom,
    Theatrical16mm,
    Super16mm,
    Academy35mm,
    TvProjection35mm,
    FullAperture35mm,
    Projection185_35mm,
    Anamorphic35mm,
    Projection70mm,
    Vistavision,
    Dynavision,
    Imax,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Camera {
    pub element: Element,
    pub projection_mode: ProjectionMode,
    pub resolution_is_pixels: bool,
    pub resolution: Vec2,
    pub field_of_view_deg: Vec2,
    pub field_of_view_tan: Vec2,
    pub orthographic_extent: Real,
    pub orthographic_size: Vec2,
    pub projection_plane: Vec2,
    pub aspect_ratio: Real,
    pub near_plane: Real,
    pub far_plane: Real,
    pub projection_axes: CoordinateAxes,
    pub aspect_mode: AspectMode,
    pub aperture_mode: ApertureMode,
    pub gate_fit: GateFit,
    pub aperture_format: ApertureFormat,
    pub focal_length_mm: Real,
    pub film_size_inch: Vec2,
    pub aperture_size_inch: Vec2,
    pub squeeze_ratio: Real,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bone {
    pub element: Element,
    pub radius: Real,
    pub relative_length: Real,
    pub is_root: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Empty {
    pub element: Element,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineSegment {
    pub index_begin: u32,
    pub num_indices: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineCurve {
    pub element: Element,
    pub color: Vec3,
    pub control_points: Vec<Vec3>,
    pub point_indices: Vec<u32>,
    pub segments: Vec<LineSegment>,
    pub from_tessellated_nurbs: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NurbsTopology {
    #[default]
    Open,
    Periodic,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NurbsBasis {
    pub order: u32,
    pub topology: NurbsTopology,
    pub knot_vector: Vec<Real>,
    pub t_min: Real,
    pub t_max: Real,
    pub spans: Vec<Real>,
    pub is_2d: bool,
    pub num_wrap_control_points: usize,
    pub valid: bool,
}

/// The control points are not homogeneous, they are multiplied by `w` to evaluate
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NurbsCurve {
    pub element: Element,
    pub basis: NurbsBasis,
    pub control_points: Vec<Vec4>,
}

/// Control points laid out `v * num_control_points_u + u`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NurbsSurface {
    pub element: Element,
    pub basis_u: NurbsBasis,
    pub basis_v: NurbsBasis,
    pub num_control_points_u: usize,
    pub num_control_points_v: usize,
    pub control_points: Vec<Vec4>,
    pub span_subdivision_u: u32,
    pub span_subdivision_v: u32,
    pub flip_normals: bool,
    pub material: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NurbsTrimSurface {
    pub element: Element,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NurbsTrimBoundary {
    pub element: Element,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProceduralGeometry {
    pub element: Element,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StereoCamera {
    pub element: Element,
    pub left: Option<usize>,
    pub right: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CameraSwitcher {
    pub element: Element,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MarkerType {
    #[default]
    Unknown,
    FkEffector,
    IkEffector,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Marker {
    pub element: Element,
    pub marker_type: MarkerType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LodDisplay {
    #[default]
    UseLod,
    Show,
    Hide,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LodLevel {
    pub distance: Real,
    pub display: LodDisplay,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LodGroup {
    pub element: Element,
    pub relative_distances: bool,
    pub lod_levels: Vec<LodLevel>,
    pub ignore_parent_transform: bool,
    pub use_distance_limit: bool,
    pub distance_limit_min: Real,
    pub distance_limit_max: Real,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SkinningMethod {
    #[default]
    Linear,
    Rigid,
    DualQuaternion,
    BlendedDqLinear,
}

/// Its weights sorted by decreasing weight, not necessarily summing to one
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SkinVertex {
    pub weight_begin: u32,
    pub num_weights: u32,
    pub dq_weight: Real,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SkinWeight {
    pub cluster_index: u32,
    pub weight: Real,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SkinDeformer {
    pub element: Element,
    pub skinning_method: SkinningMethod,
    pub clusters: Vec<usize>,
    pub vertices: Vec<SkinVertex>,
    pub weights: Vec<SkinWeight>,
    pub max_weights_per_vertex: usize,
    pub num_dq_weights: usize,
    pub dq_vertices: Vec<u32>,
    pub dq_weights: Vec<Real>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SkinCluster {
    pub element: Element,
    pub bone_node: Option<usize>,
    pub geometry_to_bone: Matrix,
    pub mesh_node_to_bone: Matrix,
    pub bind_to_world: Matrix,
    pub geometry_to_world: Matrix,
    pub geometry_to_world_transform: Transform,
    pub num_weights: usize,
    pub vertices: Vec<u32>,
    pub weights: Vec<Real>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlendDeformer {
    pub element: Element,
    pub channels: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BlendKeyframe {
    pub shape: usize,
    pub target_weight: Real,
    pub effective_weight: Real,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlendChannel {
    pub element: Element,
    pub weight: Real,
    pub keyframes: Vec<BlendKeyframe>,
    pub target_shape: Option<usize>,
}

/// `offset_vertices` may lie outside the mesh
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlendShape {
    pub element: Element,
    pub num_offsets: usize,
    pub offset_vertices: Vec<u32>,
    pub position_offsets: Vec<Vec3>,
    pub normal_offsets: Vec<Vec3>,
    pub offset_weights: Vec<Real>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CacheFileFormat {
    #[default]
    Unknown,
    Pc2,
    Mc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CacheDataFormat {
    #[default]
    Unknown,
    RealFloat,
    Vec3Float,
    RealDouble,
    Vec3Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CacheDataEncoding {
    #[default]
    Unknown,
    LittleEndian,
    BigEndian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CacheInterpretation {
    #[default]
    Unknown,
    Points,
    VertexPosition,
    VertexNormal,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CacheFrame {
    pub channel: String,
    pub time: f64,
    pub filename: String,
    pub file_format: CacheFileFormat,
    pub mirror_axis: MirrorAxis,
    pub scale_factor: Real,
    pub data_format: CacheDataFormat,
    pub data_encoding: CacheDataEncoding,
    pub data_offset: u64,
    pub data_count: u32,
    pub data_element_bytes: u32,
    pub data_total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CacheChannel {
    pub name: String,
    pub interpretation: CacheInterpretation,
    pub interpretation_name: String,
    pub frames: Vec<CacheFrame>,
    pub mirror_axis: MirrorAxis,
    pub scale_factor: Real,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GeometryCache {
    pub root_filename: String,
    pub channels: Vec<CacheChannel>,
    pub frames: Vec<CacheFrame>,
    pub extra_info: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CacheDeformer {
    pub element: Element,
    pub channel: String,
    pub file: Option<usize>,
    pub external_cache: Option<Arc<GeometryCache>>,
    /// Index into `external_cache`'s channels
    pub external_channel: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CacheFile {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Vec<u8>,
    pub raw_absolute_filename: Vec<u8>,
    pub raw_relative_filename: Vec<u8>,
    pub format: CacheFileFormat,
    pub external_cache: Option<Arc<GeometryCache>>,
}

/// A constant or factor, possibly with a texture
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MaterialMap {
    pub value_vec4: Vec4,
    pub value_int: i64,
    pub texture: Option<usize>,
    pub has_value: bool,
    pub texture_enabled: bool,
    pub feature_disabled: bool,
    pub value_components: u8,
}

impl MaterialMap {
    pub fn value_real(&self) -> Real {
        self.value_vec4.x
    }
    pub fn value_vec2(&self) -> Vec2 {
        Vec2::new(self.value_vec4.x, self.value_vec4.y)
    }
    pub fn value_vec3(&self) -> Vec3 {
        Vec3::new(self.value_vec4.x, self.value_vec4.y, self.value_vec4.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MaterialFeatureInfo {
    pub enabled: bool,
    pub is_explicit: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MaterialTexture {
    pub material_prop: String,
    pub shader_prop: String,
    pub texture: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShaderType {
    #[default]
    Unknown,
    FbxLambert,
    FbxPhong,
    OslStandardSurface,
    ArnoldStandardSurface,
    Max3dsPhysicalMaterial,
    Max3dsPbrMetalRough,
    Max3dsPbrSpecGloss,
    GltfMaterial,
    OpenpbrMaterial,
    ShaderfxGraph,
    BlenderPhong,
    WavefrontMtl,
}

macro_rules! map_enum {
    ($(#[$m:meta])* $name:ident, $maps:ident, [$($v:ident => $f:ident),* $(,)?]) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name { $($v),* }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$v),*];
            pub const COUNT: usize = [$($name::$v),*].len();
        }

        #[derive(Debug, Clone, PartialEq)]
        pub struct $maps {
            pub maps: [MaterialMap; $name::COUNT],
        }

        impl Default for $maps {
            fn default() -> $maps {
                $maps { maps: [MaterialMap::default(); $name::COUNT] }
            }
        }

        impl std::ops::Index<$name> for $maps {
            type Output = MaterialMap;
            fn index(&self, i: $name) -> &MaterialMap {
                &self.maps[i as usize]
            }
        }

        impl std::ops::IndexMut<$name> for $maps {
            fn index_mut(&mut self, i: $name) -> &mut MaterialMap {
                &mut self.maps[i as usize]
            }
        }

        impl $maps {
            $(pub fn $f(&self) -> &MaterialMap { &self.maps[$name::$v as usize] })*
        }
    };
}

map_enum!(
    MaterialFbxMap, MaterialFbxMaps, [
    DiffuseFactor => diffuse_factor,
    DiffuseColor => diffuse_color,
    SpecularFactor => specular_factor,
    SpecularColor => specular_color,
    SpecularExponent => specular_exponent,
    ReflectionFactor => reflection_factor,
    ReflectionColor => reflection_color,
    TransparencyFactor => transparency_factor,
    TransparencyColor => transparency_color,
    EmissionFactor => emission_factor,
    EmissionColor => emission_color,
    AmbientFactor => ambient_factor,
    AmbientColor => ambient_color,
    NormalMap => normal_map,
    Bump => bump,
    BumpFactor => bump_factor,
    DisplacementFactor => displacement_factor,
    Displacement => displacement,
    VectorDisplacementFactor => vector_displacement_factor,
    VectorDisplacement => vector_displacement,
]);

map_enum!(
    MaterialPbrMap, MaterialPbrMaps, [
    BaseFactor => base_factor,
    BaseColor => base_color,
    Roughness => roughness,
    Metalness => metalness,
    DiffuseRoughness => diffuse_roughness,
    SpecularFactor => specular_factor,
    SpecularColor => specular_color,
    SpecularIor => specular_ior,
    SpecularAnisotropy => specular_anisotropy,
    SpecularRotation => specular_rotation,
    TransmissionFactor => transmission_factor,
    TransmissionColor => transmission_color,
    TransmissionDepth => transmission_depth,
    TransmissionScatter => transmission_scatter,
    TransmissionScatterAnisotropy => transmission_scatter_anisotropy,
    TransmissionDispersion => transmission_dispersion,
    TransmissionRoughness => transmission_roughness,
    TransmissionExtraRoughness => transmission_extra_roughness,
    TransmissionPriority => transmission_priority,
    TransmissionEnableInAov => transmission_enable_in_aov,
    SubsurfaceFactor => subsurface_factor,
    SubsurfaceColor => subsurface_color,
    SubsurfaceRadius => subsurface_radius,
    SubsurfaceScale => subsurface_scale,
    SubsurfaceAnisotropy => subsurface_anisotropy,
    SubsurfaceTintColor => subsurface_tint_color,
    SubsurfaceType => subsurface_type,
    SheenFactor => sheen_factor,
    SheenColor => sheen_color,
    SheenRoughness => sheen_roughness,
    CoatFactor => coat_factor,
    CoatColor => coat_color,
    CoatRoughness => coat_roughness,
    CoatIor => coat_ior,
    CoatAnisotropy => coat_anisotropy,
    CoatRotation => coat_rotation,
    CoatNormal => coat_normal,
    CoatAffectBaseColor => coat_affect_base_color,
    CoatAffectBaseRoughness => coat_affect_base_roughness,
    ThinFilmFactor => thin_film_factor,
    ThinFilmThickness => thin_film_thickness,
    ThinFilmIor => thin_film_ior,
    EmissionFactor => emission_factor,
    EmissionColor => emission_color,
    Opacity => opacity,
    IndirectDiffuse => indirect_diffuse,
    IndirectSpecular => indirect_specular,
    NormalMap => normal_map,
    TangentMap => tangent_map,
    DisplacementMap => displacement_map,
    MatteFactor => matte_factor,
    MatteColor => matte_color,
    AmbientOcclusion => ambient_occlusion,
    Glossiness => glossiness,
    CoatGlossiness => coat_glossiness,
    TransmissionGlossiness => transmission_glossiness,
]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialFeature {
    Pbr,
    Metalness,
    Diffuse,
    Specular,
    Emission,
    Transmission,
    Coat,
    Sheen,
    Opacity,
    AmbientOcclusion,
    Matte,
    Unlit,
    Ior,
    DiffuseRoughness,
    TransmissionRoughness,
    ThinWalled,
    Caustics,
    ExitToBackground,
    InternalReflections,
    DoubleSided,
    RoughnessAsGlossiness,
    CoatRoughnessAsGlossiness,
    TransmissionRoughnessAsGlossiness,
}

impl MaterialFeature {
    pub const COUNT: usize = 23;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MaterialFeatures {
    pub features: [MaterialFeatureInfo; MaterialFeature::COUNT],
}

impl std::ops::Index<MaterialFeature> for MaterialFeatures {
    type Output = MaterialFeatureInfo;
    fn index(&self, i: MaterialFeature) -> &MaterialFeatureInfo {
        &self.features[i as usize]
    }
}

impl std::ops::IndexMut<MaterialFeature> for MaterialFeatures {
    fn index_mut(&mut self, i: MaterialFeature) -> &mut MaterialFeatureInfo {
        &mut self.features[i as usize]
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Material {
    pub element: Element,
    pub fbx: MaterialFbxMaps,
    pub pbr: MaterialPbrMaps,
    pub features: MaterialFeatures,
    pub shader_type: ShaderType,
    pub shader: Option<usize>,
    pub shading_model_name: String,
    pub shader_prop_prefix: String,
    pub textures: Vec<MaterialTexture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureType {
    #[default]
    File,
    Layered,
    Procedural,
    Shader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    #[default]
    Translucent,
    Additive,
    Multiply,
    Multiply2x,
    Over,
    Replace,
    Dissolve,
    Darken,
    ColorBurn,
    LinearBurn,
    DarkerColor,
    Lighten,
    Screen,
    ColorDodge,
    LinearDodge,
    LighterColor,
    SoftLight,
    HardLight,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,
    Difference,
    Exclusion,
    Subtract,
    Divide,
    Hue,
    Saturation,
    Color,
    Luminosity,
    Overlay,
}

impl BlendMode {
    pub fn from_index(i: i64) -> Option<BlendMode> {
        use BlendMode::*;
        const ALL: [BlendMode; 31] = [
            Translucent,
            Additive,
            Multiply,
            Multiply2x,
            Over,
            Replace,
            Dissolve,
            Darken,
            ColorBurn,
            LinearBurn,
            DarkerColor,
            Lighten,
            Screen,
            ColorDodge,
            LinearDodge,
            LighterColor,
            SoftLight,
            HardLight,
            VividLight,
            LinearLight,
            PinLight,
            HardMix,
            Difference,
            Exclusion,
            Subtract,
            Divide,
            Hue,
            Saturation,
            Color,
            Luminosity,
            Overlay,
        ];
        usize::try_from(i).ok().and_then(|i| ALL.get(i).copied())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WrapMode {
    #[default]
    Repeat,
    Clamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextureLayer {
    pub texture: usize,
    pub blend_mode: BlendMode,
    pub alpha: Real,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShaderTextureType {
    #[default]
    Unknown,
    SelectOutput,
    Osl,
}

/// `prop`, `texture_prop` and `texture_enabled_prop` name properties of the texture
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShaderTextureInput {
    pub name: String,
    pub value_vec4: Vec4,
    pub value_int: i64,
    pub value_str: String,
    pub value_blob: Vec<u8>,
    pub texture: Option<usize>,
    pub texture_output_index: i64,
    pub texture_enabled: bool,
    pub prop: String,
    pub texture_prop: Option<String>,
    pub texture_enabled_prop: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShaderTexture {
    pub shader_type: ShaderTextureType,
    pub shader_name: String,
    pub shader_type_id: u64,
    pub inputs: Vec<ShaderTextureInput>,
    pub shader_source: String,
    pub raw_shader_source: Vec<u8>,
    pub main_texture: Option<usize>,
    pub main_texture_output_index: i64,
    pub prop_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextureFile {
    pub index: u32,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Vec<u8>,
    pub raw_absolute_filename: Vec<u8>,
    pub raw_relative_filename: Vec<u8>,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Texture {
    pub element: Element,
    pub texture_type: TextureType,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Vec<u8>,
    pub raw_absolute_filename: Vec<u8>,
    pub raw_relative_filename: Vec<u8>,
    pub content: Vec<u8>,
    pub video: Option<usize>,
    pub file_index: u32,
    pub has_file: bool,
    pub layers: Vec<TextureLayer>,
    pub shader: Option<Box<ShaderTexture>>,
    pub file_textures: Vec<usize>,
    pub uv_set: String,
    pub wrap_u: WrapMode,
    pub wrap_v: WrapMode,
    pub has_uv_transform: bool,
    pub uv_transform: Transform,
    pub texture_to_uv: Matrix,
    pub uv_to_texture: Matrix,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Video {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Vec<u8>,
    pub raw_absolute_filename: Vec<u8>,
    pub raw_relative_filename: Vec<u8>,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shader {
    pub element: Element,
    pub shader_type: ShaderType,
    pub bindings: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShaderPropBinding {
    pub shader_prop: String,
    pub material_prop: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShaderBinding {
    pub element: Element,
    pub prop_bindings: Vec<ShaderPropBinding>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PropOverride {
    pub element_id: u32,
    pub internal_key: u32,
    pub prop_name: String,
    pub value: Vec4,
    pub value_str: String,
    pub value_int: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TransformOverride {
    pub node_id: u32,
    pub transform: Transform,
}

/// What to evaluate: the scene's, a stack's or a layer's, or one made by `create_anim`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Anim {
    pub time_begin: f64,
    pub time_end: f64,
    pub layers: Vec<usize>,
    pub override_layer_weights: Vec<Real>,
    pub prop_overrides: Vec<PropOverride>,
    pub transform_overrides: Vec<TransformOverride>,
    pub ignore_connections: bool,
    pub custom: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimStack {
    pub element: Element,
    pub time_begin: f64,
    pub time_end: f64,
    pub layers: Vec<usize>,
    pub anim: Arc<Anim>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimProp {
    pub element: ElementId,
    pub internal_key: u32,
    pub prop_name: String,
    pub anim_value: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimLayer {
    pub element: Element,
    pub weight: Real,
    pub weight_is_animated: bool,
    pub blended: bool,
    pub additive: bool,
    pub compose_rotation: bool,
    pub compose_scale: bool,
    pub anim_values: Vec<usize>,
    pub anim_props: Vec<AnimProp>,
    pub anim: Arc<Anim>,
    pub min_element_id: u32,
    pub max_element_id: u32,
    pub element_id_bitmask: [u32; 4],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimValue {
    pub element: Element,
    pub default_value: Vec3,
    pub curves: [Option<usize>; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Interpolation {
    #[default]
    ConstantPrev,
    ConstantNext,
    Linear,
    Cubic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExtrapolationMode {
    #[default]
    Constant,
    Repeat,
    Mirror,
    Slope,
    RepeatRelative,
}

/// A negative count repeats for ever
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extrapolation {
    pub mode: ExtrapolationMode,
    pub repeat_count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Tangent {
    pub dx: f32,
    pub dy: f32,
}

/// The span to the next key is interpolated as this key's `interpolation` says
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Keyframe {
    pub time: f64,
    pub value: Real,
    pub interpolation: Interpolation,
    pub left: Tangent,
    pub right: Tangent,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimCurve {
    pub element: Element,
    pub keyframes: Vec<Keyframe>,
    pub pre_extrapolation: Extrapolation,
    pub post_extrapolation: Extrapolation,
    pub min_value: Real,
    pub max_value: Real,
    pub min_time: f64,
    pub max_time: f64,
    /// The scene's `metadata.ktime_second`, which ufbx reads through `element.scene` to extrapolate in KTime ticks
    pub ktime_second: i64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayLayer {
    pub element: Element,
    pub nodes: Vec<usize>,
    pub visible: bool,
    pub frozen: bool,
    pub ui_color: Vec3,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionSet {
    pub element: Element,
    pub nodes: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionNode {
    pub element: Element,
    pub target_node: Option<usize>,
    pub target_mesh: Option<usize>,
    pub include_node: bool,
    pub vertices: Vec<u32>,
    pub edges: Vec<u32>,
    pub faces: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Character {
    pub element: Element,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConstraintType {
    #[default]
    Unknown,
    Aim,
    Parent,
    Position,
    Rotation,
    Scale,
    SingleChainIk,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ConstraintTarget {
    pub node: usize,
    pub weight: Real,
    pub transform: Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConstraintAimUpType {
    #[default]
    Scene,
    ToNode,
    AlignNode,
    Vector,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConstraintIkPoleType {
    #[default]
    Vector,
    Node,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Constraint {
    pub element: Element,
    pub constraint_type: ConstraintType,
    pub type_name: String,
    pub node: Option<usize>,
    pub targets: Vec<ConstraintTarget>,
    pub weight: Real,
    pub active: bool,
    pub constrain_translation: [bool; 3],
    pub constrain_rotation: [bool; 3],
    pub constrain_scale: [bool; 3],
    pub transform_offset: Transform,
    pub aim_vector: Vec3,
    pub aim_up_type: ConstraintAimUpType,
    pub aim_up_node: Option<usize>,
    pub aim_up_vector: Vec3,
    pub ik_effector: Option<usize>,
    pub ik_end_node: Option<usize>,
    pub ik_pole_vector: Vec3,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioLayer {
    pub element: Element,
    pub clips: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioClip {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Vec<u8>,
    pub raw_absolute_filename: Vec<u8>,
    pub raw_relative_filename: Vec<u8>,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BonePose {
    pub bone_node: usize,
    pub bone_to_world: Matrix,
    pub bone_to_parent: Matrix,
}

/// Bone poses sorted by the node's `typed_id`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pose {
    pub element: Element,
    pub is_bind_pose: bool,
    pub bone_poses: Vec<BonePose>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MetadataObject {
    pub element: Element,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NameElement {
    pub name: String,
    pub element_type: ElementType,
    pub internal_key: u32,
    pub element: ElementId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Exporter {
    #[default]
    Unknown,
    FbxSdk,
    BlenderBinary,
    BlenderAscii,
    MotionBuilder,
    UfbxWrite,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Application {
    pub vendor: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FileFormat {
    #[default]
    Unknown,
    Fbx,
    Obj,
    Mtl,
    /// Beyond ufbx: 3D Studio (.3ds), behind the feature `format-3ds`
    Tds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThumbnailFormat {
    #[default]
    Unknown,
    Rgb24,
    Rgba32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SpaceConversion {
    #[default]
    TransformRoot,
    AdjustTransforms,
    ModifyGeometry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GeometryTransformHandling {
    #[default]
    Preserve,
    HelperNodes,
    ModifyGeometry,
    ModifyGeometryNoFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InheritModeHandling {
    #[default]
    Preserve,
    HelperNodes,
    Compensate,
    CompensateNoFallback,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PivotHandling {
    #[default]
    Retain,
    AdjustToPivot,
    AdjustToRotationPivot,
}

/// Rows from bottom to top
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Thumbnail {
    pub props: Props,
    pub width: u32,
    pub height: u32,
    pub format: ThumbnailFormat,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Metadata {
    pub warnings: Vec<Warning>,
    pub ascii: bool,
    pub version: u32,
    pub file_format: FileFormat,
    pub may_contain_no_index: bool,
    pub may_contain_missing_vertex_position: bool,
    pub may_contain_broken_elements: bool,
    pub is_unsafe: bool,
    pub has_warning: [bool; WarningType::COUNT],
    pub creator: String,
    pub big_endian: bool,
    pub filename: String,
    pub relative_root: String,
    pub raw_filename: Vec<u8>,
    pub raw_relative_root: Vec<u8>,
    pub exporter: Exporter,
    pub exporter_version: u32,
    pub scene_props: Props,
    pub original_application: Application,
    pub latest_application: Application,
    pub thumbnail: Thumbnail,
    pub geometry_ignored: bool,
    pub animation_ignored: bool,
    pub embedded_ignored: bool,
    pub max_face_triangles: usize,
    pub result_memory_used: usize,
    pub temp_memory_used: usize,
    pub result_allocs: usize,
    pub temp_allocs: usize,
    pub element_buffer_size: usize,
    pub num_shader_textures: usize,
    pub bone_prop_size_unit: Real,
    pub bone_prop_limb_length_relative: bool,
    pub ortho_size_unit: Real,
    pub ktime_second: i64,
    pub original_file_path: String,
    pub raw_original_file_path: Vec<u8>,
    pub space_conversion: SpaceConversion,
    pub geometry_transform_handling: GeometryTransformHandling,
    pub inherit_mode_handling: InheritModeHandling,
    pub pivot_handling: PivotHandling,
    pub handedness_conversion_axis: MirrorAxis,
    pub root_rotation: Quat,
    pub root_scale: Real,
    pub mirror_axis: MirrorAxis,
    pub geometry_scale: Real,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TimeMode {
    #[default]
    Default,
    Fps120,
    Fps100,
    Fps60,
    Fps50,
    Fps48,
    Fps30,
    Fps30Drop,
    NtscDropFrame,
    NtscFullFrame,
    Pal,
    Fps24,
    Fps1000,
    FilmFullFrame,
    Custom,
    Fps96,
    Fps72,
    Fps59_94,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TimeProtocol {
    #[default]
    Smpte,
    FrameCount,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SnapMode {
    #[default]
    None,
    Snap,
    Play,
    SnapAndPlay,
}

/// `axes` and `unit_meters` are the file's, whatever the target options
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneSettings {
    pub props: Props,
    pub axes: CoordinateAxes,
    pub unit_meters: Real,
    pub frames_per_second: f64,
    pub ambient_color: Vec3,
    pub default_camera: String,
    pub time_mode: TimeMode,
    pub time_protocol: TimeProtocol,
    pub snap_mode: SnapMode,
    pub original_axis_up: CoordinateAxis,
    pub original_unit_meters: Real,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ElementRef {
    pub element_type: ElementType,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    pub metadata: Metadata,
    pub settings: SceneSettings,
    pub root_node: usize,
    pub anim: Arc<Anim>,
    pub unknowns: Vec<Unknown>,
    pub nodes: Vec<Node>,
    pub meshes: Vec<Mesh>,
    pub lights: Vec<Light>,
    pub cameras: Vec<Camera>,
    pub bones: Vec<Bone>,
    pub empties: Vec<Empty>,
    pub line_curves: Vec<LineCurve>,
    pub nurbs_curves: Vec<NurbsCurve>,
    pub nurbs_surfaces: Vec<NurbsSurface>,
    pub nurbs_trim_surfaces: Vec<NurbsTrimSurface>,
    pub nurbs_trim_boundaries: Vec<NurbsTrimBoundary>,
    pub procedural_geometries: Vec<ProceduralGeometry>,
    pub stereo_cameras: Vec<StereoCamera>,
    pub camera_switchers: Vec<CameraSwitcher>,
    pub markers: Vec<Marker>,
    pub lod_groups: Vec<LodGroup>,
    pub skin_deformers: Vec<SkinDeformer>,
    pub skin_clusters: Vec<SkinCluster>,
    pub blend_deformers: Vec<BlendDeformer>,
    pub blend_channels: Vec<BlendChannel>,
    pub blend_shapes: Vec<BlendShape>,
    pub cache_deformers: Vec<CacheDeformer>,
    pub cache_files: Vec<CacheFile>,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub videos: Vec<Video>,
    pub shaders: Vec<Shader>,
    pub shader_bindings: Vec<ShaderBinding>,
    pub anim_stacks: Vec<AnimStack>,
    pub anim_layers: Vec<AnimLayer>,
    pub anim_values: Vec<AnimValue>,
    pub anim_curves: Vec<AnimCurve>,
    pub display_layers: Vec<DisplayLayer>,
    pub selection_sets: Vec<SelectionSet>,
    pub selection_nodes: Vec<SelectionNode>,
    pub characters: Vec<Character>,
    pub constraints: Vec<Constraint>,
    pub audio_layers: Vec<AudioLayer>,
    pub audio_clips: Vec<AudioClip>,
    pub poses: Vec<Pose>,
    pub metadata_objects: Vec<MetadataObject>,
    pub texture_files: Vec<TextureFile>,
    pub elements: Vec<ElementRef>,
    /// Sorted by `src` then `src_prop`
    pub connections_src: Vec<Connection>,
    /// Sorted by `dst` then `dst_prop`
    pub connections_dst: Vec<Connection>,
    /// Sorted by name then type
    pub elements_by_name: Vec<NameElement>,
    pub dom_root: Option<DomNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CurvePoint {
    pub valid: bool,
    pub position: Vec3,
    pub derivative: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SurfacePoint {
    pub valid: bool,
    pub position: Vec3,
    pub derivative_u: Vec3,
    pub derivative_v: Vec3,
}

/// `ufbx_topo_flags` in ufbx.h
pub const TOPO_NON_MANIFOLD: u32 = 0x1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TopoEdge {
    pub index: u32,
    pub next: u32,
    pub prev: u32,
    pub twin: u32,
    pub face: u32,
    pub edge: u32,
    pub flags: u32,
}

/// `vertex_count` vertices of `vertex_size` bytes, compared bytewise
#[derive(Debug)]
pub struct VertexStream<'a> {
    pub data: &'a mut [u8],
    pub vertex_count: usize,
    pub vertex_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OpenFileType {
    #[default]
    MainModel,
    GeometryCache,
    ObjMtl,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OpenFileInfo {
    pub file_type: OpenFileType,
    pub original_filename: Vec<u8>,
}

/// `ufbx_open_file_cb` in ufbx.h: The file's bytes, `None` where it cannot be opened
pub type OpenFileFn = dyn Fn(&str, &OpenFileInfo) -> Option<Vec<u8>> + Send + Sync;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Progress {
    pub bytes_read: u64,
    pub bytes_total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressResult {
    #[default]
    Continue,
    Cancel,
}

pub type ProgressFn = dyn Fn(&Progress) -> ProgressResult + Send + Sync;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IndexErrorHandling {
    #[default]
    Clamp,
    NoIndex,
    AbortLoading,
    UnsafeIgnore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UnicodeErrorHandling {
    #[default]
    ReplacementCharacter,
    Underscore,
    QuestionMark,
    Remove,
    AbortLoading,
    UnsafeIgnore,
}

/// `ufbx_baked_key_flags` in ufbx.h
pub mod baked_key {
    pub const STEP_LEFT: u32 = 0x1;
    pub const STEP_RIGHT: u32 = 0x2;
    pub const STEP_KEY: u32 = 0x4;
    pub const KEYFRAME: u32 = 0x8;
    pub const REDUCED: u32 = 0x10;
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BakedVec3 {
    pub time: f64,
    pub value: Vec3,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BakedQuat {
    pub time: f64,
    pub value: Quat,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BakedNode {
    pub typed_id: u32,
    pub element_id: u32,
    pub constant_translation: bool,
    pub constant_rotation: bool,
    pub constant_scale: bool,
    pub translation_keys: Vec<BakedVec3>,
    pub rotation_keys: Vec<BakedQuat>,
    pub scale_keys: Vec<BakedVec3>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BakedProp {
    pub name: String,
    pub constant_value: bool,
    pub keys: Vec<BakedVec3>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BakedElement {
    pub element_id: u32,
    pub props: Vec<BakedProp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BakedAnimMetadata {
    pub result_memory_used: usize,
    pub temp_memory_used: usize,
    pub result_allocs: usize,
    pub temp_allocs: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BakedAnim {
    pub nodes: Vec<BakedNode>,
    pub elements: Vec<BakedElement>,
    pub playback_time_begin: f64,
    pub playback_time_end: f64,
    pub playback_duration: f64,
    pub key_time_min: f64,
    pub key_time_max: f64,
    pub metadata: BakedAnimMetadata,
}

/// `ufbx_evaluate_flags` in ufbx.h
pub const EVALUATE_FLAG_NO_EXTRAPOLATION: u32 = 0x1;

/// `ufbx_transform_flags` in ufbx.h
pub const TRANSFORM_FLAG_IGNORE_SCALE_HELPER: u32 = 0x1;
pub const TRANSFORM_FLAG_IGNORE_COMPONENTWISE_SCALE: u32 = 0x2;
pub const TRANSFORM_FLAG_EXPLICIT_INCLUDES: u32 = 0x4;
pub const TRANSFORM_FLAG_INCLUDE_TRANSLATION: u32 = 0x10;
pub const TRANSFORM_FLAG_INCLUDE_ROTATION: u32 = 0x20;
pub const TRANSFORM_FLAG_INCLUDE_SCALE: u32 = 0x40;
pub const TRANSFORM_FLAG_NO_EXTRAPOLATION: u32 = 0x80;

/// Of an allocator's limits, `memory_limit` bounds the bytes the file's arrays unpack to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllocatorOpts {
    pub memory_limit: usize,
    pub allocation_limit: usize,
    pub huge_threshold: usize,
    pub max_chunk_size: usize,
}

/// ufbx's thread pool and stream options have no counterpart here
#[derive(Clone, Default)]
pub struct LoadOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub ignore_geometry: bool,
    pub ignore_animation: bool,
    pub ignore_embedded: bool,
    pub ignore_all_content: bool,
    pub evaluate_skinning: bool,
    pub evaluate_caches: bool,
    pub load_external_files: bool,
    pub ignore_missing_external_files: bool,
    pub skip_skin_vertices: bool,
    pub skip_mesh_parts: bool,
    pub clean_skin_weights: bool,
    pub use_blender_pbr_material: bool,
    pub disable_quirks: bool,
    pub strict: bool,
    pub force_single_thread_ascii_parsing: bool,
    pub allow_unsafe: bool,
    pub index_error_handling: IndexErrorHandling,
    pub connect_broken_elements: bool,
    pub allow_nodes_out_of_root: bool,
    pub allow_missing_vertex_position: bool,
    pub allow_empty_faces: bool,
    pub generate_missing_normals: bool,
    pub open_main_file_with_default: bool,
    /// '\\' on Windows and '/' elsewhere when 0
    pub path_separator: u8,
    pub node_depth_limit: u32,
    pub file_size_estimate: u64,
    pub read_buffer_size: usize,
    pub filename: String,
    pub raw_filename: Vec<u8>,
    pub progress_cb: Option<Arc<ProgressFn>>,
    pub progress_interval_hint: u64,
    pub open_file_cb: Option<Arc<OpenFileFn>>,
    pub geometry_transform_handling: GeometryTransformHandling,
    pub inherit_mode_handling: InheritModeHandling,
    pub space_conversion: SpaceConversion,
    pub pivot_handling: PivotHandling,
    pub pivot_handling_retain_empties: bool,
    pub handedness_conversion_axis: MirrorAxis,
    pub handedness_conversion_retain_winding: bool,
    pub reverse_winding: bool,
    pub target_axes: CoordinateAxes,
    pub target_unit_meters: Real,
    pub target_camera_axes: CoordinateAxes,
    pub target_light_axes: CoordinateAxes,
    pub geometry_transform_helper_name: String,
    pub scale_helper_name: String,
    pub normalize_normals: bool,
    pub normalize_tangents: bool,
    pub use_root_transform: bool,
    pub root_transform: Transform,
    pub key_clamp_threshold: f64,
    pub unicode_error_handling: UnicodeErrorHandling,
    pub retain_vertex_attrib_w: bool,
    pub retain_dom: bool,
    pub file_format: FileFormat,
    pub file_format_lookahead: usize,
    pub no_format_from_content: bool,
    pub no_format_from_extension: bool,
    pub obj_search_mtl_by_filename: bool,
    pub obj_merge_objects: bool,
    pub obj_merge_groups: bool,
    pub obj_split_groups: bool,
    pub obj_mtl_path: String,
    pub obj_mtl_data: Vec<u8>,
    pub obj_unit_meters: Real,
    pub obj_axes: CoordinateAxes,
    /// Beyond ufbx, as `obj_unit_meters` for a 3DS, which states no unit
    pub tds_unit_meters: Real,
    /// Beyond ufbx, as `obj_axes` for a 3DS; unset, 3D Studio's own: right-handed, z up, the front -y
    pub tds_axes: CoordinateAxes,
}

impl std::fmt::Debug for LoadOpts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadOpts").field("target_axes", &self.target_axes).field("target_unit_meters", &self.target_unit_meters).finish_non_exhaustive()
    }
}

#[derive(Clone, Default)]
pub struct EvaluateOpts {
    pub evaluate_skinning: bool,
    pub evaluate_caches: bool,
    pub evaluate_flags: u32,
    pub load_external_files: bool,
    pub open_file_cb: Option<Arc<OpenFileFn>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PropOverrideDesc {
    pub element_id: u32,
    pub prop_name: String,
    pub value: Vec4,
    pub value_str: String,
    pub value_int: i64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimOpts {
    pub layer_ids: Vec<u32>,
    pub override_layer_weights: Vec<Real>,
    pub prop_overrides: Vec<PropOverrideDesc>,
    pub transform_overrides: Vec<TransformOverride>,
    pub ignore_connections: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BakeStepHandling {
    #[default]
    Default,
    CustomDuration,
    IdenticalTime,
    AdjacentDouble,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BakeOpts {
    pub trim_start_time: bool,
    pub resample_rate: f64,
    pub minimum_sample_rate: f64,
    pub maximum_sample_rate: f64,
    pub bake_transform_props: bool,
    pub skip_node_transforms: bool,
    pub no_resample_rotation: bool,
    pub ignore_layer_weight_animation: bool,
    pub max_keyframe_segments: usize,
    pub step_handling: BakeStepHandling,
    pub step_custom_duration: f64,
    pub step_custom_epsilon: f64,
    pub evaluate_flags: u32,
    pub key_reduction_enabled: bool,
    pub key_reduction_rotation: bool,
    pub key_reduction_threshold: f64,
    pub key_reduction_passes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TessellateCurveOpts {
    pub span_subdivision: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TessellateSurfaceOpts {
    pub span_subdivision_u: usize,
    pub span_subdivision_v: usize,
    pub skip_mesh_parts: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SubdivideOpts {
    pub boundary: SubdivisionBoundary,
    pub uv_boundary: SubdivisionBoundary,
    pub ignore_normals: bool,
    pub interpolate_normals: bool,
    pub interpolate_tangents: bool,
    pub evaluate_source_vertices: bool,
    pub max_source_vertices: usize,
    pub evaluate_skin_weights: bool,
    pub max_skin_weights: usize,
    pub skin_deformer_index: usize,
}

#[derive(Clone, Default)]
pub struct GeometryCacheOpts {
    pub open_file_cb: Option<Arc<OpenFileFn>>,
    pub frames_per_second: f64,
    pub mirror_axis: MirrorAxis,
    pub use_scale_factor: bool,
    pub scale_factor: Real,
}

#[derive(Clone, Default)]
pub struct GeometryCacheDataOpts {
    pub open_file_cb: Option<Arc<OpenFileFn>>,
    pub additive: bool,
    pub use_weight: bool,
    pub weight: Real,
    pub ignore_transform: bool,
}

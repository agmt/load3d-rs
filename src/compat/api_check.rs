// Written by oracle/gen_compat.py from ufbx-rust's generated.rs; do not edit.
#![allow(dead_code, unused_imports, unused_variables, non_snake_case, clippy::all)]
use super::*;
use std::ffi::c_void;

fn fields_Anim(x: &Anim) {
    let _: &f64 = &x.time_begin;
    let _: &f64 = &x.time_end;
    let _: &RefList<AnimLayer> = &x.layers;
    let _: &List<Real> = &x.override_layer_weights;
    let _: &List<PropOverride> = &x.prop_overrides;
    let _: &List<TransformOverride> = &x.transform_overrides;
    let _: &bool = &x.ignore_connections;
    let _: &bool = &x.custom;
}

fn fields_AnimCurve(x: &AnimCurve) {
    let _: &Element = &x.element;
    let _: &List<Keyframe> = &x.keyframes;
    let _: &Extrapolation = &x.pre_extrapolation;
    let _: &Extrapolation = &x.post_extrapolation;
    let _: &Real = &x.min_value;
    let _: &Real = &x.max_value;
    let _: &f64 = &x.min_time;
    let _: &f64 = &x.max_time;
}

fn fields_AnimLayer(x: &AnimLayer) {
    let _: &Element = &x.element;
    let _: &Real = &x.weight;
    let _: &bool = &x.weight_is_animated;
    let _: &bool = &x.blended;
    let _: &bool = &x.additive;
    let _: &bool = &x.compose_rotation;
    let _: &bool = &x.compose_scale;
    let _: &RefList<AnimValue> = &x.anim_values;
    let _: &List<AnimProp> = &x.anim_props;
    let _: &Ref<Anim> = &x.anim;
}

fn fields_AnimProp(x: &AnimProp) {
    let _: &Ref<Element> = &x.element;
    let _: &String = &x.prop_name;
    let _: &Ref<AnimValue> = &x.anim_value;
}

fn fields_AnimStack(x: &AnimStack) {
    let _: &Element = &x.element;
    let _: &f64 = &x.time_begin;
    let _: &f64 = &x.time_end;
    let _: &RefList<AnimLayer> = &x.layers;
    let _: &Ref<Anim> = &x.anim;
}

fn fields_AnimValue(x: &AnimValue) {
    let _: &Element = &x.element;
    let _: &Vec3 = &x.default_value;
    let _: &[Option<Ref<AnimCurve>>; 3] = &x.curves;
}

fn fields_Application(x: &Application) {
    let _: &String = &x.vendor;
    let _: &String = &x.name;
    let _: &String = &x.version;
}

fn fields_AudioClip(x: &AudioClip) {
    let _: &Element = &x.element;
    let _: &String = &x.filename;
    let _: &String = &x.absolute_filename;
    let _: &String = &x.relative_filename;
    let _: &Blob = &x.raw_filename;
    let _: &Blob = &x.raw_absolute_filename;
    let _: &Blob = &x.raw_relative_filename;
    let _: &Blob = &x.content;
}

fn fields_AudioLayer(x: &AudioLayer) {
    let _: &Element = &x.element;
    let _: &RefList<AudioClip> = &x.clips;
}

fn fields_BakedAnim(x: &BakedAnim) {
    let _: &List<BakedNode> = &x.nodes;
    let _: &List<BakedElement> = &x.elements;
    let _: &f64 = &x.playback_time_begin;
    let _: &f64 = &x.playback_time_end;
    let _: &f64 = &x.playback_duration;
    let _: &f64 = &x.key_time_min;
    let _: &f64 = &x.key_time_max;
    let _: &BakedAnimMetadata = &x.metadata;
}

fn fields_BakedAnimMetadata(x: &BakedAnimMetadata) {
    let _: &usize = &x.result_memory_used;
    let _: &usize = &x.temp_memory_used;
    let _: &usize = &x.result_allocs;
    let _: &usize = &x.temp_allocs;
}

fn fields_BakedElement(x: &BakedElement) {
    let _: &u32 = &x.element_id;
    let _: &List<BakedProp> = &x.props;
}

fn fields_BakedNode(x: &BakedNode) {
    let _: &u32 = &x.typed_id;
    let _: &u32 = &x.element_id;
    let _: &bool = &x.constant_translation;
    let _: &bool = &x.constant_rotation;
    let _: &bool = &x.constant_scale;
    let _: &List<BakedVec3> = &x.translation_keys;
    let _: &List<BakedQuat> = &x.rotation_keys;
    let _: &List<BakedVec3> = &x.scale_keys;
}

fn fields_BakedProp(x: &BakedProp) {
    let _: &String = &x.name;
    let _: &bool = &x.constant_value;
    let _: &List<BakedVec3> = &x.keys;
}

fn fields_BakedQuat(x: &BakedQuat) {
    let _: &f64 = &x.time;
    let _: &Quat = &x.value;
    let _: &BakedKeyFlags = &x.flags;
}

fn fields_BakedVec3(x: &BakedVec3) {
    let _: &f64 = &x.time;
    let _: &Vec3 = &x.value;
    let _: &BakedKeyFlags = &x.flags;
}

fn fields_BlendChannel(x: &BlendChannel) {
    let _: &Element = &x.element;
    let _: &Real = &x.weight;
    let _: &List<BlendKeyframe> = &x.keyframes;
    let _: &Option<Ref<BlendShape>> = &x.target_shape;
}

fn fields_BlendDeformer(x: &BlendDeformer) {
    let _: &Element = &x.element;
    let _: &RefList<BlendChannel> = &x.channels;
}

fn fields_BlendKeyframe(x: &BlendKeyframe) {
    let _: &Ref<BlendShape> = &x.shape;
    let _: &Real = &x.target_weight;
    let _: &Real = &x.effective_weight;
}

fn fields_BlendShape(x: &BlendShape) {
    let _: &Element = &x.element;
    let _: &usize = &x.num_offsets;
    let _: &List<u32> = &x.offset_vertices;
    let _: &List<Vec3> = &x.position_offsets;
    let _: &List<Vec3> = &x.normal_offsets;
    let _: &List<Real> = &x.offset_weights;
}

fn fields_Bone(x: &Bone) {
    let _: &Element = &x.element;
    let _: &Real = &x.radius;
    let _: &Real = &x.relative_length;
    let _: &bool = &x.is_root;
}

fn fields_BonePose(x: &BonePose) {
    let _: &Ref<Node> = &x.bone_node;
    let _: &Matrix = &x.bone_to_world;
    let _: &Matrix = &x.bone_to_parent;
}

fn fields_CacheChannel(x: &CacheChannel) {
    let _: &String = &x.name;
    let _: &CacheInterpretation = &x.interpretation;
    let _: &String = &x.interpretation_name;
    let _: &List<CacheFrame> = &x.frames;
    let _: &MirrorAxis = &x.mirror_axis;
    let _: &Real = &x.scale_factor;
}

fn fields_CacheDeformer(x: &CacheDeformer) {
    let _: &Element = &x.element;
    let _: &String = &x.channel;
    let _: &Option<Ref<CacheFile>> = &x.file;
    let _: &Option<Ref<GeometryCache>> = &x.external_cache;
    let _: &Option<Ref<CacheChannel>> = &x.external_channel;
}

fn fields_CacheFile(x: &CacheFile) {
    let _: &Element = &x.element;
    let _: &String = &x.filename;
    let _: &String = &x.absolute_filename;
    let _: &String = &x.relative_filename;
    let _: &Blob = &x.raw_filename;
    let _: &Blob = &x.raw_absolute_filename;
    let _: &Blob = &x.raw_relative_filename;
    let _: &CacheFileFormat = &x.format;
    let _: &Option<Ref<GeometryCache>> = &x.external_cache;
}

fn fields_CacheFrame(x: &CacheFrame) {
    let _: &String = &x.channel;
    let _: &f64 = &x.time;
    let _: &String = &x.filename;
    let _: &CacheFileFormat = &x.file_format;
    let _: &MirrorAxis = &x.mirror_axis;
    let _: &Real = &x.scale_factor;
    let _: &CacheDataFormat = &x.data_format;
    let _: &CacheDataEncoding = &x.data_encoding;
    let _: &u64 = &x.data_offset;
    let _: &u32 = &x.data_count;
    let _: &u32 = &x.data_element_bytes;
    let _: &u64 = &x.data_total_bytes;
}

fn fields_Camera(x: &Camera) {
    let _: &Element = &x.element;
    let _: &ProjectionMode = &x.projection_mode;
    let _: &bool = &x.resolution_is_pixels;
    let _: &Vec2 = &x.resolution;
    let _: &Vec2 = &x.field_of_view_deg;
    let _: &Vec2 = &x.field_of_view_tan;
    let _: &Real = &x.orthographic_extent;
    let _: &Vec2 = &x.orthographic_size;
    let _: &Vec2 = &x.projection_plane;
    let _: &Real = &x.aspect_ratio;
    let _: &Real = &x.near_plane;
    let _: &Real = &x.far_plane;
    let _: &CoordinateAxes = &x.projection_axes;
    let _: &AspectMode = &x.aspect_mode;
    let _: &ApertureMode = &x.aperture_mode;
    let _: &GateFit = &x.gate_fit;
    let _: &ApertureFormat = &x.aperture_format;
    let _: &Real = &x.focal_length_mm;
    let _: &Vec2 = &x.film_size_inch;
    let _: &Vec2 = &x.aperture_size_inch;
    let _: &Real = &x.squeeze_ratio;
}

fn fields_CameraSwitcher(x: &CameraSwitcher) {
    let _: &Element = &x.element;
}

fn fields_Character(x: &Character) {
    let _: &Element = &x.element;
}

fn fields_ColorSet(x: &ColorSet) {
    let _: &String = &x.name;
    let _: &u32 = &x.index;
    let _: &VertexVec4 = &x.vertex_color;
}

fn fields_Connection(x: &Connection) {
    let _: &Ref<Element> = &x.src;
    let _: &Ref<Element> = &x.dst;
    let _: &String = &x.src_prop;
    let _: &String = &x.dst_prop;
}

fn fields_Constraint(x: &Constraint) {
    let _: &Element = &x.element;
    let _: &ConstraintType = &x.type_;
    let _: &String = &x.type_name;
    let _: &Option<Ref<Node>> = &x.node;
    let _: &List<ConstraintTarget> = &x.targets;
    let _: &Real = &x.weight;
    let _: &bool = &x.active;
    let _: &[bool; 3] = &x.constrain_translation;
    let _: &[bool; 3] = &x.constrain_rotation;
    let _: &[bool; 3] = &x.constrain_scale;
    let _: &Transform = &x.transform_offset;
    let _: &Vec3 = &x.aim_vector;
    let _: &ConstraintAimUpType = &x.aim_up_type;
    let _: &Option<Ref<Node>> = &x.aim_up_node;
    let _: &Vec3 = &x.aim_up_vector;
    let _: &Option<Ref<Node>> = &x.ik_effector;
    let _: &Option<Ref<Node>> = &x.ik_end_node;
    let _: &Vec3 = &x.ik_pole_vector;
}

fn fields_ConstraintTarget(x: &ConstraintTarget) {
    let _: &Ref<Node> = &x.node;
    let _: &Real = &x.weight;
    let _: &Transform = &x.transform;
}

fn fields_CoordinateAxes(x: &CoordinateAxes) {
    let _: &CoordinateAxis = &x.right;
    let _: &CoordinateAxis = &x.up;
    let _: &CoordinateAxis = &x.front;
}

fn fields_CurvePoint(x: &CurvePoint) {
    let _: &bool = &x.valid;
    let _: &Vec3 = &x.position;
    let _: &Vec3 = &x.derivative;
}

fn fields_DisplayLayer(x: &DisplayLayer) {
    let _: &Element = &x.element;
    let _: &RefList<Node> = &x.nodes;
    let _: &bool = &x.visible;
    let _: &bool = &x.frozen;
    let _: &Vec3 = &x.ui_color;
}

fn fields_DomNode(x: &DomNode) {
    let _: &String = &x.name;
    let _: &RefList<DomNode> = &x.children;
    let _: &List<DomValue> = &x.values;
}

fn fields_DomValue(x: &DomValue) {
    let _: &DomValueType = &x.type_;
    let _: &String = &x.value_str;
    let _: &Blob = &x.value_blob;
    let _: &i64 = &x.value_int;
    let _: &f64 = &x.value_float;
}

fn fields_Edge(x: &Edge) {
    let _: &u32 = &x.a;
    let _: &u32 = &x.b;
}

fn fields_Element(x: &Element) {
    let _: &String = &x.name;
    let _: &Props = &x.props;
    let _: &u32 = &x.element_id;
    let _: &u32 = &x.typed_id;
    let _: &RefList<Node> = &x.instances;
    let _: &ElementType = &x.type_;
    let _: &List<Connection> = &x.connections_src;
    let _: &List<Connection> = &x.connections_dst;
    let _: &Option<Ref<DomNode>> = &x.dom_node;
    let _: &Ref<Scene> = &x.scene;
}

fn fields_Empty(x: &Empty) {
    let _: &Element = &x.element;
}

fn fields_Extrapolation(x: &Extrapolation) {
    let _: &ExtrapolationMode = &x.mode;
    let _: &i32 = &x.repeat_count;
}

fn fields_Face(x: &Face) {
    let _: &u32 = &x.index_begin;
    let _: &u32 = &x.num_indices;
}

fn fields_FaceGroup(x: &FaceGroup) {
    let _: &i32 = &x.id;
    let _: &String = &x.name;
}

fn fields_GeometryCache(x: &GeometryCache) {
    let _: &String = &x.root_filename;
    let _: &List<CacheChannel> = &x.channels;
    let _: &List<CacheFrame> = &x.frames;
    let _: &List<String> = &x.extra_info;
}

fn fields_Keyframe(x: &Keyframe) {
    let _: &f64 = &x.time;
    let _: &Real = &x.value;
    let _: &Interpolation = &x.interpolation;
    let _: &Tangent = &x.left;
    let _: &Tangent = &x.right;
}

fn fields_Light(x: &Light) {
    let _: &Element = &x.element;
    let _: &Vec3 = &x.color;
    let _: &Real = &x.intensity;
    let _: &Vec3 = &x.local_direction;
    let _: &LightType = &x.type_;
    let _: &LightDecay = &x.decay;
    let _: &LightAreaShape = &x.area_shape;
    let _: &Real = &x.inner_angle;
    let _: &Real = &x.outer_angle;
    let _: &bool = &x.cast_light;
    let _: &bool = &x.cast_shadows;
}

fn fields_LineCurve(x: &LineCurve) {
    let _: &Element = &x.element;
    let _: &Vec3 = &x.color;
    let _: &List<Vec3> = &x.control_points;
    let _: &List<u32> = &x.point_indices;
    let _: &List<LineSegment> = &x.segments;
    let _: &bool = &x.from_tessellated_nurbs;
}

fn fields_LineSegment(x: &LineSegment) {
    let _: &u32 = &x.index_begin;
    let _: &u32 = &x.num_indices;
}

fn fields_LodGroup(x: &LodGroup) {
    let _: &Element = &x.element;
    let _: &bool = &x.relative_distances;
    let _: &List<LodLevel> = &x.lod_levels;
    let _: &bool = &x.ignore_parent_transform;
    let _: &bool = &x.use_distance_limit;
    let _: &Real = &x.distance_limit_min;
    let _: &Real = &x.distance_limit_max;
}

fn fields_LodLevel(x: &LodLevel) {
    let _: &Real = &x.distance;
    let _: &LodDisplay = &x.display;
}

fn fields_Marker(x: &Marker) {
    let _: &Element = &x.element;
    let _: &MarkerType = &x.type_;
}

fn fields_Material(x: &Material) {
    let _: &Element = &x.element;
    let _: &MaterialFbxMaps = &x.fbx;
    let _: &MaterialPbrMaps = &x.pbr;
    let _: &MaterialFeatures = &x.features;
    let _: &ShaderType = &x.shader_type;
    let _: &Option<Ref<Shader>> = &x.shader;
    let _: &String = &x.shading_model_name;
    let _: &String = &x.shader_prop_prefix;
    let _: &List<MaterialTexture> = &x.textures;
}

fn fields_MaterialFbxMaps(x: &MaterialFbxMaps) {
    let _: &MaterialMap = &x.diffuse_factor;
    let _: &MaterialMap = &x.diffuse_color;
    let _: &MaterialMap = &x.specular_factor;
    let _: &MaterialMap = &x.specular_color;
    let _: &MaterialMap = &x.specular_exponent;
    let _: &MaterialMap = &x.reflection_factor;
    let _: &MaterialMap = &x.reflection_color;
    let _: &MaterialMap = &x.transparency_factor;
    let _: &MaterialMap = &x.transparency_color;
    let _: &MaterialMap = &x.emission_factor;
    let _: &MaterialMap = &x.emission_color;
    let _: &MaterialMap = &x.ambient_factor;
    let _: &MaterialMap = &x.ambient_color;
    let _: &MaterialMap = &x.normal_map;
    let _: &MaterialMap = &x.bump;
    let _: &MaterialMap = &x.bump_factor;
    let _: &MaterialMap = &x.displacement_factor;
    let _: &MaterialMap = &x.displacement;
    let _: &MaterialMap = &x.vector_displacement_factor;
    let _: &MaterialMap = &x.vector_displacement;
}

fn fields_MaterialFeatureInfo(x: &MaterialFeatureInfo) {
    let _: &bool = &x.enabled;
    let _: &bool = &x.is_explicit;
}

fn fields_MaterialFeatures(x: &MaterialFeatures) {
    let _: &MaterialFeatureInfo = &x.pbr;
    let _: &MaterialFeatureInfo = &x.metalness;
    let _: &MaterialFeatureInfo = &x.diffuse;
    let _: &MaterialFeatureInfo = &x.specular;
    let _: &MaterialFeatureInfo = &x.emission;
    let _: &MaterialFeatureInfo = &x.transmission;
    let _: &MaterialFeatureInfo = &x.coat;
    let _: &MaterialFeatureInfo = &x.sheen;
    let _: &MaterialFeatureInfo = &x.opacity;
    let _: &MaterialFeatureInfo = &x.ambient_occlusion;
    let _: &MaterialFeatureInfo = &x.matte;
    let _: &MaterialFeatureInfo = &x.unlit;
    let _: &MaterialFeatureInfo = &x.ior;
    let _: &MaterialFeatureInfo = &x.diffuse_roughness;
    let _: &MaterialFeatureInfo = &x.transmission_roughness;
    let _: &MaterialFeatureInfo = &x.thin_walled;
    let _: &MaterialFeatureInfo = &x.caustics;
    let _: &MaterialFeatureInfo = &x.exit_to_background;
    let _: &MaterialFeatureInfo = &x.internal_reflections;
    let _: &MaterialFeatureInfo = &x.double_sided;
    let _: &MaterialFeatureInfo = &x.roughness_as_glossiness;
    let _: &MaterialFeatureInfo = &x.coat_roughness_as_glossiness;
    let _: &MaterialFeatureInfo = &x.transmission_roughness_as_glossiness;
}

fn fields_MaterialMap(x: &MaterialMap) {
    let _: &Vec4 = &x.value_vec4;
    let _: &i64 = &x.value_int;
    let _: &Option<Ref<Texture>> = &x.texture;
    let _: &bool = &x.has_value;
    let _: &bool = &x.texture_enabled;
    let _: &bool = &x.feature_disabled;
    let _: &u8 = &x.value_components;
}

fn fields_MaterialPbrMaps(x: &MaterialPbrMaps) {
    let _: &MaterialMap = &x.base_factor;
    let _: &MaterialMap = &x.base_color;
    let _: &MaterialMap = &x.roughness;
    let _: &MaterialMap = &x.metalness;
    let _: &MaterialMap = &x.diffuse_roughness;
    let _: &MaterialMap = &x.specular_factor;
    let _: &MaterialMap = &x.specular_color;
    let _: &MaterialMap = &x.specular_ior;
    let _: &MaterialMap = &x.specular_anisotropy;
    let _: &MaterialMap = &x.specular_rotation;
    let _: &MaterialMap = &x.transmission_factor;
    let _: &MaterialMap = &x.transmission_color;
    let _: &MaterialMap = &x.transmission_depth;
    let _: &MaterialMap = &x.transmission_scatter;
    let _: &MaterialMap = &x.transmission_scatter_anisotropy;
    let _: &MaterialMap = &x.transmission_dispersion;
    let _: &MaterialMap = &x.transmission_roughness;
    let _: &MaterialMap = &x.transmission_extra_roughness;
    let _: &MaterialMap = &x.transmission_priority;
    let _: &MaterialMap = &x.transmission_enable_in_aov;
    let _: &MaterialMap = &x.subsurface_factor;
    let _: &MaterialMap = &x.subsurface_color;
    let _: &MaterialMap = &x.subsurface_radius;
    let _: &MaterialMap = &x.subsurface_scale;
    let _: &MaterialMap = &x.subsurface_anisotropy;
    let _: &MaterialMap = &x.subsurface_tint_color;
    let _: &MaterialMap = &x.subsurface_type;
    let _: &MaterialMap = &x.sheen_factor;
    let _: &MaterialMap = &x.sheen_color;
    let _: &MaterialMap = &x.sheen_roughness;
    let _: &MaterialMap = &x.coat_factor;
    let _: &MaterialMap = &x.coat_color;
    let _: &MaterialMap = &x.coat_roughness;
    let _: &MaterialMap = &x.coat_ior;
    let _: &MaterialMap = &x.coat_anisotropy;
    let _: &MaterialMap = &x.coat_rotation;
    let _: &MaterialMap = &x.coat_normal;
    let _: &MaterialMap = &x.coat_affect_base_color;
    let _: &MaterialMap = &x.coat_affect_base_roughness;
    let _: &MaterialMap = &x.thin_film_factor;
    let _: &MaterialMap = &x.thin_film_thickness;
    let _: &MaterialMap = &x.thin_film_ior;
    let _: &MaterialMap = &x.emission_factor;
    let _: &MaterialMap = &x.emission_color;
    let _: &MaterialMap = &x.opacity;
    let _: &MaterialMap = &x.indirect_diffuse;
    let _: &MaterialMap = &x.indirect_specular;
    let _: &MaterialMap = &x.normal_map;
    let _: &MaterialMap = &x.tangent_map;
    let _: &MaterialMap = &x.displacement_map;
    let _: &MaterialMap = &x.matte_factor;
    let _: &MaterialMap = &x.matte_color;
    let _: &MaterialMap = &x.ambient_occlusion;
    let _: &MaterialMap = &x.glossiness;
    let _: &MaterialMap = &x.coat_glossiness;
    let _: &MaterialMap = &x.transmission_glossiness;
}

fn fields_MaterialTexture(x: &MaterialTexture) {
    let _: &String = &x.material_prop;
    let _: &String = &x.shader_prop;
    let _: &Ref<Texture> = &x.texture;
}

fn fields_Matrix(x: &Matrix) {
    let _: &Real = &x.m00;
    let _: &Real = &x.m10;
    let _: &Real = &x.m20;
    let _: &Real = &x.m01;
    let _: &Real = &x.m11;
    let _: &Real = &x.m21;
    let _: &Real = &x.m02;
    let _: &Real = &x.m12;
    let _: &Real = &x.m22;
    let _: &Real = &x.m03;
    let _: &Real = &x.m13;
    let _: &Real = &x.m23;
}

fn fields_Mesh(x: &Mesh) {
    let _: &Element = &x.element;
    let _: &usize = &x.num_vertices;
    let _: &usize = &x.num_indices;
    let _: &usize = &x.num_faces;
    let _: &usize = &x.num_triangles;
    let _: &usize = &x.num_edges;
    let _: &usize = &x.max_face_triangles;
    let _: &usize = &x.num_empty_faces;
    let _: &usize = &x.num_point_faces;
    let _: &usize = &x.num_line_faces;
    let _: &List<Face> = &x.faces;
    let _: &List<bool> = &x.face_smoothing;
    let _: &List<u32> = &x.face_material;
    let _: &List<u32> = &x.face_group;
    let _: &List<bool> = &x.face_hole;
    let _: &List<Edge> = &x.edges;
    let _: &List<bool> = &x.edge_smoothing;
    let _: &List<Real> = &x.edge_crease;
    let _: &List<bool> = &x.edge_visibility;
    let _: &List<u32> = &x.vertex_indices;
    let _: &List<Vec3> = &x.vertices;
    let _: &List<u32> = &x.vertex_first_index;
    let _: &VertexVec3 = &x.vertex_position;
    let _: &VertexVec3 = &x.vertex_normal;
    let _: &VertexVec2 = &x.vertex_uv;
    let _: &VertexVec3 = &x.vertex_tangent;
    let _: &VertexVec3 = &x.vertex_bitangent;
    let _: &VertexVec4 = &x.vertex_color;
    let _: &VertexReal = &x.vertex_crease;
    let _: &List<UvSet> = &x.uv_sets;
    let _: &List<ColorSet> = &x.color_sets;
    let _: &RefList<Material> = &x.materials;
    let _: &List<FaceGroup> = &x.face_groups;
    let _: &List<MeshPart> = &x.material_parts;
    let _: &List<MeshPart> = &x.face_group_parts;
    let _: &List<u32> = &x.material_part_usage_order;
    let _: &bool = &x.skinned_is_local;
    let _: &VertexVec3 = &x.skinned_position;
    let _: &VertexVec3 = &x.skinned_normal;
    let _: &RefList<SkinDeformer> = &x.skin_deformers;
    let _: &RefList<BlendDeformer> = &x.blend_deformers;
    let _: &RefList<CacheDeformer> = &x.cache_deformers;
    let _: &RefList<Element> = &x.all_deformers;
    let _: &u32 = &x.subdivision_preview_levels;
    let _: &u32 = &x.subdivision_render_levels;
    let _: &SubdivisionDisplayMode = &x.subdivision_display_mode;
    let _: &SubdivisionBoundary = &x.subdivision_boundary;
    let _: &SubdivisionBoundary = &x.subdivision_uv_boundary;
    let _: &bool = &x.reversed_winding;
    let _: &bool = &x.generated_normals;
    let _: &bool = &x.subdivision_evaluated;
    let _: &Option<Ref<SubdivisionResult>> = &x.subdivision_result;
    let _: &bool = &x.from_tessellated_nurbs;
}

fn fields_MeshPart(x: &MeshPart) {
    let _: &u32 = &x.index;
    let _: &usize = &x.num_faces;
    let _: &usize = &x.num_triangles;
    let _: &usize = &x.num_empty_faces;
    let _: &usize = &x.num_point_faces;
    let _: &usize = &x.num_line_faces;
    let _: &List<u32> = &x.face_indices;
}

fn fields_Metadata(x: &Metadata) {
    let _: &List<Warning> = &x.warnings;
    let _: &bool = &x.ascii;
    let _: &u32 = &x.version;
    let _: &FileFormat = &x.file_format;
    let _: &bool = &x.may_contain_no_index;
    let _: &bool = &x.may_contain_missing_vertex_position;
    let _: &bool = &x.may_contain_broken_elements;
    let _: &bool = &x.is_unsafe;
    let _: &[bool; 15] = &x.has_warning;
    let _: &String = &x.creator;
    let _: &bool = &x.big_endian;
    let _: &String = &x.filename;
    let _: &String = &x.relative_root;
    let _: &Blob = &x.raw_filename;
    let _: &Blob = &x.raw_relative_root;
    let _: &Exporter = &x.exporter;
    let _: &u32 = &x.exporter_version;
    let _: &Props = &x.scene_props;
    let _: &Application = &x.original_application;
    let _: &Application = &x.latest_application;
    let _: &Thumbnail = &x.thumbnail;
    let _: &bool = &x.geometry_ignored;
    let _: &bool = &x.animation_ignored;
    let _: &bool = &x.embedded_ignored;
    let _: &usize = &x.max_face_triangles;
    let _: &usize = &x.result_memory_used;
    let _: &usize = &x.temp_memory_used;
    let _: &usize = &x.result_allocs;
    let _: &usize = &x.temp_allocs;
    let _: &usize = &x.element_buffer_size;
    let _: &usize = &x.num_shader_textures;
    let _: &Real = &x.bone_prop_size_unit;
    let _: &bool = &x.bone_prop_limb_length_relative;
    let _: &Real = &x.ortho_size_unit;
    let _: &i64 = &x.ktime_second;
    let _: &String = &x.original_file_path;
    let _: &Blob = &x.raw_original_file_path;
    let _: &SpaceConversion = &x.space_conversion;
    let _: &GeometryTransformHandling = &x.geometry_transform_handling;
    let _: &InheritModeHandling = &x.inherit_mode_handling;
    let _: &PivotHandling = &x.pivot_handling;
    let _: &MirrorAxis = &x.handedness_conversion_axis;
    let _: &Quat = &x.root_rotation;
    let _: &Real = &x.root_scale;
    let _: &MirrorAxis = &x.mirror_axis;
    let _: &Real = &x.geometry_scale;
}

fn fields_MetadataObject(x: &MetadataObject) {
    let _: &Element = &x.element;
}

fn fields_NameElement(x: &NameElement) {
    let _: &String = &x.name;
    let _: &ElementType = &x.type_;
    let _: &Ref<Element> = &x.element;
}

fn fields_Node(x: &Node) {
    let _: &Element = &x.element;
    let _: &Option<Ref<Node>> = &x.parent;
    let _: &RefList<Node> = &x.children;
    let _: &Option<Ref<Mesh>> = &x.mesh;
    let _: &Option<Ref<Light>> = &x.light;
    let _: &Option<Ref<Camera>> = &x.camera;
    let _: &Option<Ref<Bone>> = &x.bone;
    let _: &Option<Ref<Element>> = &x.attrib;
    let _: &Option<Ref<Node>> = &x.geometry_transform_helper;
    let _: &Option<Ref<Node>> = &x.scale_helper;
    let _: &ElementType = &x.attrib_type;
    let _: &RefList<Element> = &x.all_attribs;
    let _: &InheritMode = &x.inherit_mode;
    let _: &InheritMode = &x.original_inherit_mode;
    let _: &Transform = &x.local_transform;
    let _: &Transform = &x.geometry_transform;
    let _: &Vec3 = &x.inherit_scale;
    let _: &Option<Ref<Node>> = &x.inherit_scale_node;
    let _: &RotationOrder = &x.rotation_order;
    let _: &Vec3 = &x.euler_rotation;
    let _: &Matrix = &x.node_to_parent;
    let _: &Matrix = &x.node_to_world;
    let _: &Matrix = &x.geometry_to_node;
    let _: &Matrix = &x.geometry_to_world;
    let _: &Matrix = &x.unscaled_node_to_world;
    let _: &Vec3 = &x.adjust_pre_translation;
    let _: &Quat = &x.adjust_pre_rotation;
    let _: &Real = &x.adjust_pre_scale;
    let _: &Quat = &x.adjust_post_rotation;
    let _: &Real = &x.adjust_post_scale;
    let _: &Real = &x.adjust_translation_scale;
    let _: &MirrorAxis = &x.adjust_mirror_axis;
    let _: &RefList<Material> = &x.materials;
    let _: &Option<Ref<Pose>> = &x.bind_pose;
    let _: &bool = &x.visible;
    let _: &bool = &x.is_root;
    let _: &bool = &x.has_geometry_transform;
    let _: &bool = &x.use_rotation_space;
    let _: &bool = &x.has_adjust_transform;
    let _: &bool = &x.has_root_adjust_transform;
    let _: &bool = &x.is_geometry_transform_helper;
    let _: &bool = &x.is_scale_helper;
    let _: &bool = &x.is_scale_compensate_parent;
    let _: &u32 = &x.node_depth;
}

fn fields_NurbsBasis(x: &NurbsBasis) {
    let _: &u32 = &x.order;
    let _: &NurbsTopology = &x.topology;
    let _: &List<Real> = &x.knot_vector;
    let _: &Real = &x.t_min;
    let _: &Real = &x.t_max;
    let _: &List<Real> = &x.spans;
    let _: &bool = &x.is_2d;
    let _: &usize = &x.num_wrap_control_points;
    let _: &bool = &x.valid;
}

fn fields_NurbsCurve(x: &NurbsCurve) {
    let _: &Element = &x.element;
    let _: &NurbsBasis = &x.basis;
    let _: &List<Vec4> = &x.control_points;
}

fn fields_NurbsSurface(x: &NurbsSurface) {
    let _: &Element = &x.element;
    let _: &NurbsBasis = &x.basis_u;
    let _: &NurbsBasis = &x.basis_v;
    let _: &usize = &x.num_control_points_u;
    let _: &usize = &x.num_control_points_v;
    let _: &List<Vec4> = &x.control_points;
    let _: &u32 = &x.span_subdivision_u;
    let _: &u32 = &x.span_subdivision_v;
    let _: &bool = &x.flip_normals;
    let _: &Option<Ref<Material>> = &x.material;
}

fn fields_NurbsTrimBoundary(x: &NurbsTrimBoundary) {
    let _: &Element = &x.element;
}

fn fields_NurbsTrimSurface(x: &NurbsTrimSurface) {
    let _: &Element = &x.element;
}

fn fields_Pose(x: &Pose) {
    let _: &Element = &x.element;
    let _: &bool = &x.is_bind_pose;
    let _: &List<BonePose> = &x.bone_poses;
}

fn fields_ProceduralGeometry(x: &ProceduralGeometry) {
    let _: &Element = &x.element;
}

fn fields_Progress(x: &Progress) {
    let _: &u64 = &x.bytes_read;
    let _: &u64 = &x.bytes_total;
}

fn fields_Prop(x: &Prop) {
    let _: &String = &x.name;
    let _: &PropType = &x.type_;
    let _: &PropFlags = &x.flags;
    let _: &String = &x.value_str;
    let _: &Blob = &x.value_blob;
    let _: &i64 = &x.value_int;
    let _: &Vec4 = &x.value_vec4;
}

fn fields_PropOverride(x: &PropOverride) {
    let _: &u32 = &x.element_id;
    let _: &String = &x.prop_name;
    let _: &Vec4 = &x.value;
    let _: &String = &x.value_str;
    let _: &i64 = &x.value_int;
}

fn fields_Props(x: &Props) {
    let _: &List<Prop> = &x.props;
    let _: &usize = &x.num_animated;
    let _: &Option<Ref<Props>> = &x.defaults;
}

fn fields_Quat(x: &Quat) {
    let _: &Real = &x.x;
    let _: &Real = &x.y;
    let _: &Real = &x.z;
    let _: &Real = &x.w;
}

fn fields_Scene(x: &Scene) {
    let _: &Metadata = &x.metadata;
    let _: &SceneSettings = &x.settings;
    let _: &Ref<Node> = &x.root_node;
    let _: &Ref<Anim> = &x.anim;
    let _: &RefList<Unknown> = &x.unknowns;
    let _: &RefList<Node> = &x.nodes;
    let _: &RefList<Mesh> = &x.meshes;
    let _: &RefList<Light> = &x.lights;
    let _: &RefList<Camera> = &x.cameras;
    let _: &RefList<Bone> = &x.bones;
    let _: &RefList<Empty> = &x.empties;
    let _: &RefList<LineCurve> = &x.line_curves;
    let _: &RefList<NurbsCurve> = &x.nurbs_curves;
    let _: &RefList<NurbsSurface> = &x.nurbs_surfaces;
    let _: &RefList<NurbsTrimSurface> = &x.nurbs_trim_surfaces;
    let _: &RefList<NurbsTrimBoundary> = &x.nurbs_trim_boundaries;
    let _: &RefList<ProceduralGeometry> = &x.procedural_geometries;
    let _: &RefList<StereoCamera> = &x.stereo_cameras;
    let _: &RefList<CameraSwitcher> = &x.camera_switchers;
    let _: &RefList<Marker> = &x.markers;
    let _: &RefList<LodGroup> = &x.lod_groups;
    let _: &RefList<SkinDeformer> = &x.skin_deformers;
    let _: &RefList<SkinCluster> = &x.skin_clusters;
    let _: &RefList<BlendDeformer> = &x.blend_deformers;
    let _: &RefList<BlendChannel> = &x.blend_channels;
    let _: &RefList<BlendShape> = &x.blend_shapes;
    let _: &RefList<CacheDeformer> = &x.cache_deformers;
    let _: &RefList<CacheFile> = &x.cache_files;
    let _: &RefList<Material> = &x.materials;
    let _: &RefList<Texture> = &x.textures;
    let _: &RefList<Video> = &x.videos;
    let _: &RefList<Shader> = &x.shaders;
    let _: &RefList<ShaderBinding> = &x.shader_bindings;
    let _: &RefList<AnimStack> = &x.anim_stacks;
    let _: &RefList<AnimLayer> = &x.anim_layers;
    let _: &RefList<AnimValue> = &x.anim_values;
    let _: &RefList<AnimCurve> = &x.anim_curves;
    let _: &RefList<DisplayLayer> = &x.display_layers;
    let _: &RefList<SelectionSet> = &x.selection_sets;
    let _: &RefList<SelectionNode> = &x.selection_nodes;
    let _: &RefList<Character> = &x.characters;
    let _: &RefList<Constraint> = &x.constraints;
    let _: &RefList<AudioLayer> = &x.audio_layers;
    let _: &RefList<AudioClip> = &x.audio_clips;
    let _: &RefList<Pose> = &x.poses;
    let _: &RefList<MetadataObject> = &x.metadata_objects;
    let _: &List<TextureFile> = &x.texture_files;
    let _: &RefList<Element> = &x.elements;
    let _: &List<Connection> = &x.connections_src;
    let _: &List<Connection> = &x.connections_dst;
    let _: &List<NameElement> = &x.elements_by_name;
    let _: &Option<Ref<DomNode>> = &x.dom_root;
}

fn fields_SceneSettings(x: &SceneSettings) {
    let _: &Props = &x.props;
    let _: &CoordinateAxes = &x.axes;
    let _: &Real = &x.unit_meters;
    let _: &f64 = &x.frames_per_second;
    let _: &Vec3 = &x.ambient_color;
    let _: &String = &x.default_camera;
    let _: &TimeMode = &x.time_mode;
    let _: &TimeProtocol = &x.time_protocol;
    let _: &SnapMode = &x.snap_mode;
    let _: &CoordinateAxis = &x.original_axis_up;
    let _: &Real = &x.original_unit_meters;
}

fn fields_SelectionNode(x: &SelectionNode) {
    let _: &Element = &x.element;
    let _: &Option<Ref<Node>> = &x.target_node;
    let _: &Option<Ref<Mesh>> = &x.target_mesh;
    let _: &bool = &x.include_node;
    let _: &List<u32> = &x.vertices;
    let _: &List<u32> = &x.edges;
    let _: &List<u32> = &x.faces;
}

fn fields_SelectionSet(x: &SelectionSet) {
    let _: &Element = &x.element;
    let _: &RefList<SelectionNode> = &x.nodes;
}

fn fields_Shader(x: &Shader) {
    let _: &Element = &x.element;
    let _: &ShaderType = &x.type_;
    let _: &RefList<ShaderBinding> = &x.bindings;
}

fn fields_ShaderBinding(x: &ShaderBinding) {
    let _: &Element = &x.element;
    let _: &List<ShaderPropBinding> = &x.prop_bindings;
}

fn fields_ShaderPropBinding(x: &ShaderPropBinding) {
    let _: &String = &x.shader_prop;
    let _: &String = &x.material_prop;
}

fn fields_ShaderTexture(x: &ShaderTexture) {
    let _: &ShaderTextureType = &x.type_;
    let _: &String = &x.shader_name;
    let _: &u64 = &x.shader_type_id;
    let _: &List<ShaderTextureInput> = &x.inputs;
    let _: &String = &x.shader_source;
    let _: &Blob = &x.raw_shader_source;
    let _: &Ref<Texture> = &x.main_texture;
    let _: &i64 = &x.main_texture_output_index;
    let _: &String = &x.prop_prefix;
}

fn fields_ShaderTextureInput(x: &ShaderTextureInput) {
    let _: &String = &x.name;
    let _: &Vec4 = &x.value_vec4;
    let _: &i64 = &x.value_int;
    let _: &String = &x.value_str;
    let _: &Blob = &x.value_blob;
    let _: &Option<Ref<Texture>> = &x.texture;
    let _: &i64 = &x.texture_output_index;
    let _: &bool = &x.texture_enabled;
    let _: &Ref<Prop> = &x.prop;
    let _: &Option<Ref<Prop>> = &x.texture_prop;
    let _: &Option<Ref<Prop>> = &x.texture_enabled_prop;
}

fn fields_SkinCluster(x: &SkinCluster) {
    let _: &Element = &x.element;
    let _: &Option<Ref<Node>> = &x.bone_node;
    let _: &Matrix = &x.geometry_to_bone;
    let _: &Matrix = &x.mesh_node_to_bone;
    let _: &Matrix = &x.bind_to_world;
    let _: &Matrix = &x.geometry_to_world;
    let _: &Transform = &x.geometry_to_world_transform;
    let _: &usize = &x.num_weights;
    let _: &List<u32> = &x.vertices;
    let _: &List<Real> = &x.weights;
}

fn fields_SkinDeformer(x: &SkinDeformer) {
    let _: &Element = &x.element;
    let _: &SkinningMethod = &x.skinning_method;
    let _: &RefList<SkinCluster> = &x.clusters;
    let _: &List<SkinVertex> = &x.vertices;
    let _: &List<SkinWeight> = &x.weights;
    let _: &usize = &x.max_weights_per_vertex;
    let _: &usize = &x.num_dq_weights;
    let _: &List<u32> = &x.dq_vertices;
    let _: &List<Real> = &x.dq_weights;
}

fn fields_SkinVertex(x: &SkinVertex) {
    let _: &u32 = &x.weight_begin;
    let _: &u32 = &x.num_weights;
    let _: &Real = &x.dq_weight;
}

fn fields_SkinWeight(x: &SkinWeight) {
    let _: &u32 = &x.cluster_index;
    let _: &Real = &x.weight;
}

fn fields_StereoCamera(x: &StereoCamera) {
    let _: &Element = &x.element;
    let _: &Option<Ref<Camera>> = &x.left;
    let _: &Option<Ref<Camera>> = &x.right;
}

fn fields_SubdivisionResult(x: &SubdivisionResult) {
    let _: &usize = &x.result_memory_used;
    let _: &usize = &x.temp_memory_used;
    let _: &usize = &x.result_allocs;
    let _: &usize = &x.temp_allocs;
    let _: &List<SubdivisionWeightRange> = &x.source_vertex_ranges;
    let _: &List<SubdivisionWeight> = &x.source_vertex_weights;
    let _: &List<SubdivisionWeightRange> = &x.skin_cluster_ranges;
    let _: &List<SubdivisionWeight> = &x.skin_cluster_weights;
}

fn fields_SubdivisionWeight(x: &SubdivisionWeight) {
    let _: &Real = &x.weight;
    let _: &u32 = &x.index;
}

fn fields_SubdivisionWeightRange(x: &SubdivisionWeightRange) {
    let _: &u32 = &x.weight_begin;
    let _: &u32 = &x.num_weights;
}

fn fields_SurfacePoint(x: &SurfacePoint) {
    let _: &bool = &x.valid;
    let _: &Vec3 = &x.position;
    let _: &Vec3 = &x.derivative_u;
    let _: &Vec3 = &x.derivative_v;
}

fn fields_Tangent(x: &Tangent) {
    let _: &f32 = &x.dx;
    let _: &f32 = &x.dy;
}

fn fields_Texture(x: &Texture) {
    let _: &Element = &x.element;
    let _: &TextureType = &x.type_;
    let _: &String = &x.filename;
    let _: &String = &x.absolute_filename;
    let _: &String = &x.relative_filename;
    let _: &Blob = &x.raw_filename;
    let _: &Blob = &x.raw_absolute_filename;
    let _: &Blob = &x.raw_relative_filename;
    let _: &Blob = &x.content;
    let _: &Option<Ref<Video>> = &x.video;
    let _: &u32 = &x.file_index;
    let _: &bool = &x.has_file;
    let _: &List<TextureLayer> = &x.layers;
    let _: &Option<Ref<ShaderTexture>> = &x.shader;
    let _: &RefList<Texture> = &x.file_textures;
    let _: &String = &x.uv_set;
    let _: &WrapMode = &x.wrap_u;
    let _: &WrapMode = &x.wrap_v;
    let _: &bool = &x.has_uv_transform;
    let _: &Transform = &x.uv_transform;
    let _: &Matrix = &x.texture_to_uv;
    let _: &Matrix = &x.uv_to_texture;
}

fn fields_TextureFile(x: &TextureFile) {
    let _: &u32 = &x.index;
    let _: &String = &x.filename;
    let _: &String = &x.absolute_filename;
    let _: &String = &x.relative_filename;
    let _: &Blob = &x.raw_filename;
    let _: &Blob = &x.raw_absolute_filename;
    let _: &Blob = &x.raw_relative_filename;
    let _: &Blob = &x.content;
}

fn fields_TextureLayer(x: &TextureLayer) {
    let _: &Ref<Texture> = &x.texture;
    let _: &BlendMode = &x.blend_mode;
    let _: &Real = &x.alpha;
}

fn fields_Thumbnail(x: &Thumbnail) {
    let _: &Props = &x.props;
    let _: &u32 = &x.width;
    let _: &u32 = &x.height;
    let _: &ThumbnailFormat = &x.format;
    let _: &Blob = &x.data;
}

fn fields_TopoEdge(x: &TopoEdge) {
    let _: &u32 = &x.index;
    let _: &u32 = &x.next;
    let _: &u32 = &x.prev;
    let _: &u32 = &x.twin;
    let _: &u32 = &x.face;
    let _: &u32 = &x.edge;
    let _: &TopoFlags = &x.flags;
}

fn fields_Transform(x: &Transform) {
    let _: &Vec3 = &x.translation;
    let _: &Quat = &x.rotation;
    let _: &Vec3 = &x.scale;
}

fn fields_TransformOverride(x: &TransformOverride) {
    let _: &u32 = &x.node_id;
    let _: &Transform = &x.transform;
}

fn fields_Unknown(x: &Unknown) {
    let _: &Element = &x.element;
    let _: &String = &x.type_;
    let _: &String = &x.super_type;
    let _: &String = &x.sub_type;
}

fn fields_UvSet(x: &UvSet) {
    let _: &String = &x.name;
    let _: &u32 = &x.index;
    let _: &VertexVec2 = &x.vertex_uv;
    let _: &VertexVec3 = &x.vertex_tangent;
    let _: &VertexVec3 = &x.vertex_bitangent;
}

fn fields_Vec2(x: &Vec2) {
    let _: &Real = &x.x;
    let _: &Real = &x.y;
}

fn fields_Vec3(x: &Vec3) {
    let _: &Real = &x.x;
    let _: &Real = &x.y;
    let _: &Real = &x.z;
}

fn fields_Vec4(x: &Vec4) {
    let _: &Real = &x.x;
    let _: &Real = &x.y;
    let _: &Real = &x.z;
    let _: &Real = &x.w;
}

fn fields_VertexReal(x: &VertexReal) {
    let _: &bool = &x.exists;
    let _: &List<Real> = &x.values;
    let _: &List<u32> = &x.indices;
    let _: &usize = &x.value_reals;
    let _: &bool = &x.unique_per_vertex;
    let _: &List<Real> = &x.values_w;
}

fn fields_VertexVec2(x: &VertexVec2) {
    let _: &bool = &x.exists;
    let _: &List<Vec2> = &x.values;
    let _: &List<u32> = &x.indices;
    let _: &usize = &x.value_reals;
    let _: &bool = &x.unique_per_vertex;
    let _: &List<Real> = &x.values_w;
}

fn fields_VertexVec3(x: &VertexVec3) {
    let _: &bool = &x.exists;
    let _: &List<Vec3> = &x.values;
    let _: &List<u32> = &x.indices;
    let _: &usize = &x.value_reals;
    let _: &bool = &x.unique_per_vertex;
    let _: &List<Real> = &x.values_w;
}

fn fields_VertexVec4(x: &VertexVec4) {
    let _: &bool = &x.exists;
    let _: &List<Vec4> = &x.values;
    let _: &List<u32> = &x.indices;
    let _: &usize = &x.value_reals;
    let _: &bool = &x.unique_per_vertex;
    let _: &List<Real> = &x.values_w;
}

fn fields_Video(x: &Video) {
    let _: &Element = &x.element;
    let _: &String = &x.filename;
    let _: &String = &x.absolute_filename;
    let _: &String = &x.relative_filename;
    let _: &Blob = &x.raw_filename;
    let _: &Blob = &x.raw_absolute_filename;
    let _: &Blob = &x.raw_relative_filename;
    let _: &Blob = &x.content;
}

fn fields_Warning(x: &Warning) {
    let _: &WarningType = &x.type_;
    let _: &String = &x.description;
    let _: &u32 = &x.element_id;
    let _: &usize = &x.count;
}

const _: fn() -> bool = is_thread_safe;
const _: fn(&[u8], LoadOpts) -> Result<SceneRoot> = load_memory;
const _: fn(&str, LoadOpts) -> Result<SceneRoot> = load_file;
const _: fn(*mut c_void, LoadOpts) -> Result<SceneRoot> = load_stdio;
const _: fn(*mut c_void, &[u8], LoadOpts) -> Result<SceneRoot> = load_stdio_prefix;
const _: fn(Stream, LoadOpts) -> Result<SceneRoot> = load_stream;
const _: fn(Stream, &[u8], LoadOpts) -> Result<SceneRoot> = load_stream_prefix;
const _: fn(&mut [u8], &Error) -> usize = format_error;
const _: for<'a> fn(&'a Props, &str) -> Option<&'a Prop> = find_prop;
const _: fn(&Props, &str, Blob) -> Blob = find_blob;
const _: for<'a> fn(&'a Element, &Prop, ElementType) -> Option<&'a Element> = get_prop_element;
const _: for<'a> fn(&'a Element, &str, ElementType) -> Option<&'a Element> = find_prop_element;
const _: for<'a> fn(&'a Scene, ElementType, &str) -> Option<&'a Element> = find_element;
const _: for<'a> fn(&'a Scene, &str) -> Option<&'a Node> = find_node;
const _: for<'a> fn(&'a Scene, &str) -> Option<&'a AnimStack> = find_anim_stack;
const _: for<'a> fn(&'a Scene, &str) -> Option<&'a Material> = find_material;
const _: for<'a> fn(&'a AnimLayer, &'a Element, &str) -> Option<&'a AnimProp> = find_anim_prop;
const _: for<'a> fn(&'a AnimLayer, &'a Element) -> &'a [AnimProp] = find_anim_props;
const _: fn(&Node) -> Matrix = get_compatible_matrix_for_normals;
const _: fn(&mut [u8], &InflateInput, &mut InflateRetain) -> isize = inflate;
const _: fn(&AnimCurve, f64, Real) -> Real = evaluate_curve;
const _: fn(&AnimCurve, f64, Real, u32) -> Real = evaluate_curve_flags;
const _: fn(&AnimValue, f64) -> Real = evaluate_anim_value_real;
const _: fn(&AnimValue, f64) -> Vec3 = evaluate_anim_value_vec3;
const _: fn(&AnimValue, f64, u32) -> Real = evaluate_anim_value_real_flags;
const _: fn(&AnimValue, f64, u32) -> Vec3 = evaluate_anim_value_vec3_flags;
const _: for<'a> fn(&'a Anim, &'a Element, &'a str, f64) -> ExternalRef<'a, Prop> = evaluate_prop;
const _: fn(&Anim, &Element, &str, f64, u32) -> Prop = evaluate_prop_flags;
const _: for<'a> fn(&'a Anim, &'a Element, f64, &'a mut [ExternalRef<'a, Prop>]) -> ExternalRef<'a, Props> = evaluate_props;
const _: fn(&Anim, &Element, f64, &mut Prop, usize, u32) -> Props = evaluate_props_flags;
const _: fn(&Anim, &Node, f64) -> Transform = evaluate_transform;
const _: fn(&Anim, &Node, f64, u32) -> Transform = evaluate_transform_flags;
const _: fn(&Anim, &BlendChannel, f64) -> Real = evaluate_blend_weight;
const _: fn(&Anim, &BlendChannel, f64, u32) -> Real = evaluate_blend_weight_flags;
const _: fn(&Scene, &Anim, f64, EvaluateOpts) -> Result<SceneRoot> = evaluate_scene;
const _: fn(&Scene, AnimOpts) -> Result<AnimRoot> = create_anim;
const _: fn(&Scene, &Anim, BakeOpts) -> Result<BakedAnimRoot> = bake_anim;
const _: fn(&mut BakedAnim, u32) -> Option<&'static BakedNode> = find_baked_node_by_typed_id;
const _: for<'a> fn(&mut BakedAnim, &'a mut Node) -> Option<&'a BakedNode> = find_baked_node;
const _: fn(&mut BakedAnim, u32) -> Option<&'static BakedElement> = find_baked_element_by_element_id;
const _: for<'a> fn(&mut BakedAnim, &'a mut Element) -> Option<&'a BakedElement> = find_baked_element;
const _: fn(&[BakedVec3], f64) -> Vec3 = evaluate_baked_vec3;
const _: fn(&[BakedQuat], f64) -> Quat = evaluate_baked_quat;
const _: for<'a> fn(&'a Pose, &'a Node) -> Option<&'a BonePose> = get_bone_pose;
const _: for<'a> fn(&'a Material, &str) -> Option<&'a Texture> = find_prop_texture;
const _: for<'a> fn(&'a Shader, &'a str) -> &'a str = find_shader_prop;
const _: for<'a> fn(&'a Shader, &str) -> &'a [ShaderPropBinding] = find_shader_prop_bindings;
const _: fn(&ShaderTexture, &str) -> Option<&'static ShaderTextureInput> = find_shader_texture_input;
const _: fn(CoordinateAxes) -> bool = coordinate_axes_valid;
const _: fn(Vec3) -> Vec3 = vec3_normalize;
const _: fn(Quat, Quat) -> Real = quat_dot;
const _: fn(Quat, Quat) -> Quat = quat_mul;
const _: fn(Quat) -> Quat = quat_normalize;
const _: fn(Quat, Quat) -> Quat = quat_fix_antipodal;
const _: fn(Quat, Quat, Real) -> Quat = quat_slerp;
const _: fn(Quat, Vec3) -> Vec3 = quat_rotate_vec3;
const _: fn(Quat, RotationOrder) -> Vec3 = quat_to_euler;
const _: fn(Vec3, RotationOrder) -> Quat = euler_to_quat;
const _: fn(&Matrix, &Matrix) -> Matrix = matrix_mul;
const _: fn(&Matrix) -> Real = matrix_determinant;
const _: fn(&Matrix) -> Matrix = matrix_invert;
const _: fn(&Matrix) -> Matrix = matrix_for_normals;
const _: fn(&Matrix, Vec3) -> Vec3 = transform_position;
const _: fn(&Matrix, Vec3) -> Vec3 = transform_direction;
const _: fn(&Transform) -> Matrix = transform_to_matrix;
const _: fn(&Matrix) -> Transform = matrix_to_transform;
const _: fn(&SkinDeformer, usize, &Matrix) -> Matrix = get_skin_vertex_matrix;
const _: fn(&BlendShape, usize) -> u32 = get_blend_shape_offset_index;
const _: fn(&BlendShape, usize) -> Vec3 = get_blend_shape_vertex_offset;
const _: fn(&BlendDeformer, usize) -> Vec3 = get_blend_vertex_offset;
const _: fn(&BlendShape, &mut [Vec3], Real) = add_blend_shape_vertex_offsets;
const _: fn(&BlendDeformer, &mut [Vec3], Real) = add_blend_vertex_offsets;
const _: fn(&NurbsBasis, Real, &mut [Real], &mut [Real]) -> usize = evaluate_nurbs_basis;
const _: fn(&NurbsCurve, Real) -> CurvePoint = evaluate_nurbs_curve;
const _: fn(&NurbsSurface, Real, Real) -> SurfacePoint = evaluate_nurbs_surface;
const _: fn(&NurbsCurve, TessellateCurveOpts) -> Result<LineCurveRoot> = tessellate_nurbs_curve;
const _: fn(&NurbsSurface, TessellateSurfaceOpts) -> Result<MeshRoot> = tessellate_nurbs_surface;
const _: fn(&mut Mesh, usize) -> u32 = find_face_index;
const _: fn(&mut [u32], &Mesh, Face) -> u32 = triangulate_face;
const _: fn(&Mesh, &mut [TopoEdge]) = compute_topology;
const _: fn(&[TopoEdge], u32) -> u32 = topo_next_vertex_edge;
const _: fn(&[TopoEdge], u32) -> u32 = topo_prev_vertex_edge;
const _: fn(&VertexVec3, Face) -> Vec3 = get_weighted_face_normal;
const _: fn(&Mesh, &[TopoEdge], &mut [u32], bool) -> usize = generate_normal_mapping;
const _: fn(&Mesh, &VertexVec3, &[u32], &mut [Vec3]) = compute_normals;
const _: fn(&Mesh, usize, SubdivideOpts) -> Result<MeshRoot> = subdivide_mesh;
const _: fn(&str, GeometryCacheOpts) -> Result<GeometryCacheRoot> = load_geometry_cache;
const _: fn(&CacheFrame, &mut [Real], GeometryCacheDataOpts) -> usize = read_geometry_cache_real;
const _: fn(&CacheFrame, &mut [Vec3], GeometryCacheDataOpts) -> usize = read_geometry_cache_vec3;
const _: fn(&CacheChannel, f64, &mut [Real], GeometryCacheDataOpts) -> usize = sample_geometry_cache_real;
const _: fn(&CacheChannel, f64, &mut [Vec3], GeometryCacheDataOpts) -> usize = sample_geometry_cache_vec3;
const _: fn(&DomNode, &str) -> Option<&'static DomNode> = dom_find;
const _: fn(&mut [VertexStream], &mut [u32], AllocatorOpts) -> Result<usize> = generate_indices;
const _: fn(&VertexReal, usize) -> Real = get_vertex_real;
const _: fn(&VertexVec2, usize) -> Vec2 = get_vertex_vec2;
const _: fn(&VertexVec3, usize) -> Vec3 = get_vertex_vec3;
const _: fn(&VertexVec4, usize) -> Vec4 = get_vertex_vec4;
const _: fn(&VertexVec3, usize) -> Real = get_vertex_w_vec3;
const _: for<'a> fn(&'a Element) -> Option<&'a Unknown> = as_unknown;
const _: for<'a> fn(&'a Element) -> Option<&'a Node> = as_node;
const _: for<'a> fn(&'a Element) -> Option<&'a Mesh> = as_mesh;
const _: for<'a> fn(&'a Element) -> Option<&'a Light> = as_light;
const _: for<'a> fn(&'a Element) -> Option<&'a Camera> = as_camera;
const _: for<'a> fn(&'a Element) -> Option<&'a Bone> = as_bone;
const _: for<'a> fn(&'a Element) -> Option<&'a Empty> = as_empty;
const _: for<'a> fn(&'a Element) -> Option<&'a LineCurve> = as_line_curve;
const _: for<'a> fn(&'a Element) -> Option<&'a NurbsCurve> = as_nurbs_curve;
const _: for<'a> fn(&'a Element) -> Option<&'a NurbsSurface> = as_nurbs_surface;
const _: for<'a> fn(&'a Element) -> Option<&'a NurbsTrimSurface> = as_nurbs_trim_surface;
const _: for<'a> fn(&'a Element) -> Option<&'a NurbsTrimBoundary> = as_nurbs_trim_boundary;
const _: for<'a> fn(&'a Element) -> Option<&'a ProceduralGeometry> = as_procedural_geometry;
const _: for<'a> fn(&'a Element) -> Option<&'a StereoCamera> = as_stereo_camera;
const _: for<'a> fn(&'a Element) -> Option<&'a CameraSwitcher> = as_camera_switcher;
const _: for<'a> fn(&'a Element) -> Option<&'a Marker> = as_marker;
const _: for<'a> fn(&'a Element) -> Option<&'a LodGroup> = as_lod_group;
const _: for<'a> fn(&'a Element) -> Option<&'a SkinDeformer> = as_skin_deformer;
const _: for<'a> fn(&'a Element) -> Option<&'a SkinCluster> = as_skin_cluster;
const _: for<'a> fn(&'a Element) -> Option<&'a BlendDeformer> = as_blend_deformer;
const _: for<'a> fn(&'a Element) -> Option<&'a BlendChannel> = as_blend_channel;
const _: for<'a> fn(&'a Element) -> Option<&'a BlendShape> = as_blend_shape;
const _: for<'a> fn(&'a Element) -> Option<&'a CacheDeformer> = as_cache_deformer;
const _: for<'a> fn(&'a Element) -> Option<&'a CacheFile> = as_cache_file;
const _: for<'a> fn(&'a Element) -> Option<&'a Material> = as_material;
const _: for<'a> fn(&'a Element) -> Option<&'a Texture> = as_texture;
const _: for<'a> fn(&'a Element) -> Option<&'a Video> = as_video;
const _: for<'a> fn(&'a Element) -> Option<&'a Shader> = as_shader;
const _: for<'a> fn(&'a Element) -> Option<&'a ShaderBinding> = as_shader_binding;
const _: for<'a> fn(&'a Element) -> Option<&'a AnimStack> = as_anim_stack;
const _: for<'a> fn(&'a Element) -> Option<&'a AnimLayer> = as_anim_layer;
const _: for<'a> fn(&'a Element) -> Option<&'a AnimValue> = as_anim_value;
const _: for<'a> fn(&'a Element) -> Option<&'a AnimCurve> = as_anim_curve;
const _: for<'a> fn(&'a Element) -> Option<&'a DisplayLayer> = as_display_layer;
const _: for<'a> fn(&'a Element) -> Option<&'a SelectionSet> = as_selection_set;
const _: for<'a> fn(&'a Element) -> Option<&'a SelectionNode> = as_selection_node;
const _: for<'a> fn(&'a Element) -> Option<&'a Character> = as_character;
const _: for<'a> fn(&'a Element) -> Option<&'a Constraint> = as_constraint;
const _: for<'a> fn(&'a Element) -> Option<&'a AudioLayer> = as_audio_layer;
const _: for<'a> fn(&'a Element) -> Option<&'a AudioClip> = as_audio_clip;
const _: for<'a> fn(&'a Element) -> Option<&'a Pose> = as_pose;
const _: for<'a> fn(&'a Element) -> Option<&'a MetadataObject> = as_metadata_object;
const _: fn(&DomNode) -> bool = dom_is_array;
const _: fn(&DomNode) -> usize = dom_array_size;
const _: fn(&DomNode) -> &'static [i32] = dom_as_int32_list;
const _: fn(&DomNode) -> &'static [i64] = dom_as_int64_list;
const _: fn(&DomNode) -> &'static [f32] = dom_as_float_list;
const _: fn(&DomNode) -> &'static [f64] = dom_as_double_list;
const _: fn(&DomNode) -> &'static [Real] = dom_as_real_list;
const _: fn(&DomNode) -> &'static [Blob] = dom_as_blob_list;
const _: fn() -> Matrix = identity_matrix;
const _: fn() -> Transform = identity_transform;
const _: fn() -> Vec2 = zero_vec2;
const _: fn() -> Vec3 = zero_vec3;
const _: fn() -> Vec4 = zero_vec4;
const _: fn() -> Quat = identity_quat;
const _: fn() -> CoordinateAxes = axes_right_handed_y_up;
const _: fn() -> CoordinateAxes = axes_right_handed_z_up;
const _: fn() -> CoordinateAxes = axes_left_handed_y_up;
const _: fn() -> CoordinateAxes = axes_left_handed_z_up;
const _: fn() -> u32 = source_version;

const _: fn(&Error) -> &str = Error::info;
const _: fn(&Panic) -> &str = Panic::message;
const _: fn() -> Vec2 = Vec2::zero;
const _: fn() -> Vec3 = Vec3::zero;
const _: fn() -> Vec4 = Vec4::zero;
const _: fn() -> Quat = Quat::identity;
const _: fn() -> Transform = Transform::identity;
const _: fn() -> Matrix = Matrix::identity;
const _: for<'a> fn(&'a DomNode, &str) -> Option<&'a DomNode> = DomNode::find;
const _: fn(&DomNode) -> bool = DomNode::is_array;
const _: fn(&DomNode) -> usize = DomNode::array_size;
const _: for<'a> fn(&'a DomNode) -> &'a [i32] = DomNode::as_int32_list;
const _: for<'a> fn(&'a DomNode) -> &'a [i64] = DomNode::as_int64_list;
const _: for<'a> fn(&'a DomNode) -> &'a [f32] = DomNode::as_float_list;
const _: for<'a> fn(&'a DomNode) -> &'a [f64] = DomNode::as_double_list;
const _: for<'a> fn(&'a DomNode) -> &'a [Real] = DomNode::as_real_list;
const _: for<'a> fn(&'a DomNode) -> &'a [Blob] = DomNode::as_blob_list;
const _: for<'a> fn(&'a Props, &str) -> Option<&'a Prop> = Props::find_prop;
const _: fn(&Node) -> Matrix = Node::get_compatible_matrix_for_normals;
const _: fn(&Node, &Anim, f64) -> Transform = Node::evaluate_transform;
const _: fn(&Mesh, &mut [u32], Face) -> u32 = Mesh::triangulate_face;
const _: fn(&Mesh, usize, SubdivideOpts) -> Result<MeshRoot> = Mesh::subdivide;
const _: fn() -> CoordinateAxes = CoordinateAxes::right_handed_y_up;
const _: fn() -> CoordinateAxes = CoordinateAxes::right_handed_z_up;
const _: fn() -> CoordinateAxes = CoordinateAxes::left_handed_y_up;
const _: fn() -> CoordinateAxes = CoordinateAxes::left_handed_z_up;
const _: fn(&NurbsBasis, Real, &mut [Real], &mut [Real]) -> usize = NurbsBasis::evaluate;
const _: fn(&NurbsCurve, Real) -> CurvePoint = NurbsCurve::evaluate;
const _: fn(&NurbsCurve, TessellateCurveOpts) -> Result<LineCurveRoot> = NurbsCurve::tessellate;
const _: fn(&NurbsSurface, Real, Real) -> SurfacePoint = NurbsSurface::evaluate;
const _: fn(&NurbsSurface, TessellateSurfaceOpts) -> Result<MeshRoot> = NurbsSurface::tessellate;
const _: fn(&SkinDeformer, usize, &Matrix) -> Matrix = SkinDeformer::get_skin_vertex_matrix;
const _: fn(&BlendDeformer, usize) -> Vec3 = BlendDeformer::get_vertex_offset;
const _: fn(&BlendDeformer, &mut [Vec3], Real) = BlendDeformer::add_vertex_offsets;
const _: fn(&BlendChannel, &Anim, f64) -> Real = BlendChannel::evaluate_blend_weight;
const _: fn(&BlendShape, usize) -> Vec3 = BlendShape::get_vertex_offset;
const _: fn(&BlendShape, &mut [Vec3], Real) = BlendShape::add_vertex_offsets;
const _: fn(&CacheFrame, &mut [Real], GeometryCacheDataOpts) -> usize = CacheFrame::read_real;
const _: fn(&CacheFrame, &mut [Vec3], GeometryCacheDataOpts) -> usize = CacheFrame::read_vec3;
const _: fn(&CacheChannel, f64, &mut [Real], GeometryCacheDataOpts) -> usize = CacheChannel::sample_real;
const _: fn(&CacheChannel, f64, &mut [Vec3], GeometryCacheDataOpts) -> usize = CacheChannel::sample_vec3;
const _: for<'a> fn(&'a Material, &str) -> Option<&'a Texture> = Material::find_prop_texture;
const _: for<'a> fn(&'a Shader, &'a str) -> &'a str = Shader::find_shader_prop;
const _: for<'a> fn(&'a AnimLayer, &'a Element, &str) -> Option<&'a AnimProp> = AnimLayer::find_anim_prop;
const _: for<'a> fn(&'a AnimLayer, &'a Element) -> &'a [AnimProp] = AnimLayer::find_anim_props;
const _: fn(&AnimValue, f64) -> Real = AnimValue::evaluate_real;
const _: fn(&AnimValue, f64) -> Vec3 = AnimValue::evaluate_vec3;
const _: fn(&AnimCurve, f64, Real) -> Real = AnimCurve::evaluate;
const _: for<'a> fn(&'a Scene, ElementType, &str) -> Option<&'a Element> = Scene::find_element;
const _: for<'a> fn(&'a Scene, &str) -> Option<&'a Node> = Scene::find_node;
const _: for<'a> fn(&'a Scene, &str) -> Option<&'a AnimStack> = Scene::find_anim_stack;
const _: fn(&Scene, &Anim, f64, EvaluateOpts) -> Result<SceneRoot> = Scene::evaluate;
const _: fn(&Element) -> ElementData<'_> = Element::as_data;

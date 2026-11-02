// Written by oracle/gen_compat.py from ufbx-rust's generated.rs and load3d's types.rs; do not edit.
#![allow(non_snake_case, dead_code, unused_parens, clippy::all)]
use super::*;
use crate as f;

pub use crate::{
    ApertureMode, AspectMode, BakeStepHandling, CacheDataEncoding, CacheDataFormat, CacheFileFormat, CacheInterpretation, ConstraintAimUpType,
    ConstraintIkPoleType, ConstraintType, CoordinateAxis, DomValueType, ElementType, ErrorType, Exporter, ExtrapolationMode, GateFit,
    GeometryTransformHandling, IndexErrorHandling, InheritMode, InheritModeHandling, Interpolation, LightAreaShape, LightDecay, LightType, LodDisplay,
    MarkerType, MaterialFbxMap, MaterialFeature, MaterialPbrMap, MirrorAxis, NurbsTopology, OpenFileType, PivotHandling, ProjectionMode, PropType,
    RotationOrder, ShaderTextureType, SkinningMethod, SnapMode, SpaceConversion, SubdivisionBoundary, SubdivisionDisplayMode, TextureType, ThumbnailFormat,
    TimeProtocol, UnicodeErrorHandling, WarningType, WrapMode,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u32)]
pub enum ApertureFormat {
    Custom = 0,
    E16MmTheatrical = 1,
    Super16Mm = 2,
    E35MmAcademy = 3,
    E35MmTvProjection = 4,
    E35MmFullAperture = 5,
    E35Mm185Projection = 6,
    E35MmAnamorphic = 7,
    E70MmProjection = 8,
    Vistavision = 9,
    Dynavision = 10,
    Imax = 11,
}
impl Default for ApertureFormat {
    fn default() -> Self {
        Self::Custom
    }
}
pub(crate) fn from_enum_ApertureFormat(p: f::ApertureFormat) -> ApertureFormat {
    #[allow(unreachable_patterns)]
    match p {
        f::ApertureFormat::Custom => ApertureFormat::Custom,
        f::ApertureFormat::Theatrical16mm => ApertureFormat::E16MmTheatrical,
        f::ApertureFormat::Super16mm => ApertureFormat::Super16Mm,
        f::ApertureFormat::Academy35mm => ApertureFormat::E35MmAcademy,
        f::ApertureFormat::TvProjection35mm => ApertureFormat::E35MmTvProjection,
        f::ApertureFormat::FullAperture35mm => ApertureFormat::E35MmFullAperture,
        f::ApertureFormat::Projection185_35mm => ApertureFormat::E35Mm185Projection,
        f::ApertureFormat::Anamorphic35mm => ApertureFormat::E35MmAnamorphic,
        f::ApertureFormat::Projection70mm => ApertureFormat::E70MmProjection,
        f::ApertureFormat::Vistavision => ApertureFormat::Vistavision,
        f::ApertureFormat::Dynavision => ApertureFormat::Dynavision,
        f::ApertureFormat::Imax => ApertureFormat::Imax,
        _ => ApertureFormat::default(),
    }
}
pub(crate) fn to_enum_ApertureFormat(c: ApertureFormat) -> f::ApertureFormat {
    match c {
        ApertureFormat::Custom => f::ApertureFormat::Custom,
        ApertureFormat::E16MmTheatrical => f::ApertureFormat::Theatrical16mm,
        ApertureFormat::Super16Mm => f::ApertureFormat::Super16mm,
        ApertureFormat::E35MmAcademy => f::ApertureFormat::Academy35mm,
        ApertureFormat::E35MmTvProjection => f::ApertureFormat::TvProjection35mm,
        ApertureFormat::E35MmFullAperture => f::ApertureFormat::FullAperture35mm,
        ApertureFormat::E35Mm185Projection => f::ApertureFormat::Projection185_35mm,
        ApertureFormat::E35MmAnamorphic => f::ApertureFormat::Anamorphic35mm,
        ApertureFormat::E70MmProjection => f::ApertureFormat::Projection70mm,
        ApertureFormat::Vistavision => f::ApertureFormat::Vistavision,
        ApertureFormat::Dynavision => f::ApertureFormat::Dynavision,
        ApertureFormat::Imax => f::ApertureFormat::Imax,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u32)]
pub enum BlendMode {
    Translucent = 0,
    Additive = 1,
    Multiply = 2,
    Multiply2X = 3,
    Over = 4,
    Replace = 5,
    Dissolve = 6,
    Darken = 7,
    ColorBurn = 8,
    LinearBurn = 9,
    DarkerColor = 10,
    Lighten = 11,
    Screen = 12,
    ColorDodge = 13,
    LinearDodge = 14,
    LighterColor = 15,
    SoftLight = 16,
    HardLight = 17,
    VividLight = 18,
    LinearLight = 19,
    PinLight = 20,
    HardMix = 21,
    Difference = 22,
    Exclusion = 23,
    Subtract = 24,
    Divide = 25,
    Hue = 26,
    Saturation = 27,
    Color = 28,
    Luminosity = 29,
    Overlay = 30,
}
impl Default for BlendMode {
    fn default() -> Self {
        Self::Translucent
    }
}
pub(crate) fn from_enum_BlendMode(p: f::BlendMode) -> BlendMode {
    #[allow(unreachable_patterns)]
    match p {
        f::BlendMode::Translucent => BlendMode::Translucent,
        f::BlendMode::Additive => BlendMode::Additive,
        f::BlendMode::Multiply => BlendMode::Multiply,
        f::BlendMode::Multiply2x => BlendMode::Multiply2X,
        f::BlendMode::Over => BlendMode::Over,
        f::BlendMode::Replace => BlendMode::Replace,
        f::BlendMode::Dissolve => BlendMode::Dissolve,
        f::BlendMode::Darken => BlendMode::Darken,
        f::BlendMode::ColorBurn => BlendMode::ColorBurn,
        f::BlendMode::LinearBurn => BlendMode::LinearBurn,
        f::BlendMode::DarkerColor => BlendMode::DarkerColor,
        f::BlendMode::Lighten => BlendMode::Lighten,
        f::BlendMode::Screen => BlendMode::Screen,
        f::BlendMode::ColorDodge => BlendMode::ColorDodge,
        f::BlendMode::LinearDodge => BlendMode::LinearDodge,
        f::BlendMode::LighterColor => BlendMode::LighterColor,
        f::BlendMode::SoftLight => BlendMode::SoftLight,
        f::BlendMode::HardLight => BlendMode::HardLight,
        f::BlendMode::VividLight => BlendMode::VividLight,
        f::BlendMode::LinearLight => BlendMode::LinearLight,
        f::BlendMode::PinLight => BlendMode::PinLight,
        f::BlendMode::HardMix => BlendMode::HardMix,
        f::BlendMode::Difference => BlendMode::Difference,
        f::BlendMode::Exclusion => BlendMode::Exclusion,
        f::BlendMode::Subtract => BlendMode::Subtract,
        f::BlendMode::Divide => BlendMode::Divide,
        f::BlendMode::Hue => BlendMode::Hue,
        f::BlendMode::Saturation => BlendMode::Saturation,
        f::BlendMode::Color => BlendMode::Color,
        f::BlendMode::Luminosity => BlendMode::Luminosity,
        f::BlendMode::Overlay => BlendMode::Overlay,
        _ => BlendMode::default(),
    }
}
pub(crate) fn to_enum_BlendMode(c: BlendMode) -> f::BlendMode {
    match c {
        BlendMode::Translucent => f::BlendMode::Translucent,
        BlendMode::Additive => f::BlendMode::Additive,
        BlendMode::Multiply => f::BlendMode::Multiply,
        BlendMode::Multiply2X => f::BlendMode::Multiply2x,
        BlendMode::Over => f::BlendMode::Over,
        BlendMode::Replace => f::BlendMode::Replace,
        BlendMode::Dissolve => f::BlendMode::Dissolve,
        BlendMode::Darken => f::BlendMode::Darken,
        BlendMode::ColorBurn => f::BlendMode::ColorBurn,
        BlendMode::LinearBurn => f::BlendMode::LinearBurn,
        BlendMode::DarkerColor => f::BlendMode::DarkerColor,
        BlendMode::Lighten => f::BlendMode::Lighten,
        BlendMode::Screen => f::BlendMode::Screen,
        BlendMode::ColorDodge => f::BlendMode::ColorDodge,
        BlendMode::LinearDodge => f::BlendMode::LinearDodge,
        BlendMode::LighterColor => f::BlendMode::LighterColor,
        BlendMode::SoftLight => f::BlendMode::SoftLight,
        BlendMode::HardLight => f::BlendMode::HardLight,
        BlendMode::VividLight => f::BlendMode::VividLight,
        BlendMode::LinearLight => f::BlendMode::LinearLight,
        BlendMode::PinLight => f::BlendMode::PinLight,
        BlendMode::HardMix => f::BlendMode::HardMix,
        BlendMode::Difference => f::BlendMode::Difference,
        BlendMode::Exclusion => f::BlendMode::Exclusion,
        BlendMode::Subtract => f::BlendMode::Subtract,
        BlendMode::Divide => f::BlendMode::Divide,
        BlendMode::Hue => f::BlendMode::Hue,
        BlendMode::Saturation => f::BlendMode::Saturation,
        BlendMode::Color => f::BlendMode::Color,
        BlendMode::Luminosity => f::BlendMode::Luminosity,
        BlendMode::Overlay => f::BlendMode::Overlay,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u32)]
pub enum FileFormat {
    Unknown = 0,
    Fbx = 1,
    Obj = 2,
    Mtl = 3,
}
impl Default for FileFormat {
    fn default() -> Self {
        Self::Unknown
    }
}
pub(crate) fn from_enum_FileFormat(p: f::FileFormat) -> FileFormat {
    #[allow(unreachable_patterns)]
    match p {
        f::FileFormat::Unknown => FileFormat::Unknown,
        f::FileFormat::Fbx => FileFormat::Fbx,
        f::FileFormat::Obj => FileFormat::Obj,
        f::FileFormat::Mtl => FileFormat::Mtl,
        _ => FileFormat::default(),
    }
}
pub(crate) fn to_enum_FileFormat(c: FileFormat) -> f::FileFormat {
    match c {
        FileFormat::Unknown => f::FileFormat::Unknown,
        FileFormat::Fbx => f::FileFormat::Fbx,
        FileFormat::Obj => f::FileFormat::Obj,
        FileFormat::Mtl => f::FileFormat::Mtl,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u32)]
pub enum ProgressResult {
    Continue = 256,
    Cancel = 512,
}
impl Default for ProgressResult {
    fn default() -> Self {
        Self::Continue
    }
}
pub(crate) fn from_enum_ProgressResult(p: f::ProgressResult) -> ProgressResult {
    #[allow(unreachable_patterns)]
    match p {
        f::ProgressResult::Continue => ProgressResult::Continue,
        f::ProgressResult::Cancel => ProgressResult::Cancel,
        _ => ProgressResult::default(),
    }
}
pub(crate) fn to_enum_ProgressResult(c: ProgressResult) -> f::ProgressResult {
    match c {
        ProgressResult::Continue => f::ProgressResult::Continue,
        ProgressResult::Cancel => f::ProgressResult::Cancel,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u32)]
pub enum ShaderType {
    Unknown = 0,
    FbxLambert = 1,
    FbxPhong = 2,
    OslStandardSurface = 3,
    ArnoldStandardSurface = 4,
    E3DsMaxPhysicalMaterial = 5,
    E3DsMaxPbrMetalRough = 6,
    E3DsMaxPbrSpecGloss = 7,
    GltfMaterial = 8,
    OpenpbrMaterial = 9,
    ShaderfxGraph = 10,
    BlenderPhong = 11,
    WavefrontMtl = 12,
}
impl Default for ShaderType {
    fn default() -> Self {
        Self::Unknown
    }
}
pub(crate) fn from_enum_ShaderType(p: f::ShaderType) -> ShaderType {
    #[allow(unreachable_patterns)]
    match p {
        f::ShaderType::Unknown => ShaderType::Unknown,
        f::ShaderType::FbxLambert => ShaderType::FbxLambert,
        f::ShaderType::FbxPhong => ShaderType::FbxPhong,
        f::ShaderType::OslStandardSurface => ShaderType::OslStandardSurface,
        f::ShaderType::ArnoldStandardSurface => ShaderType::ArnoldStandardSurface,
        f::ShaderType::Max3dsPhysicalMaterial => ShaderType::E3DsMaxPhysicalMaterial,
        f::ShaderType::Max3dsPbrMetalRough => ShaderType::E3DsMaxPbrMetalRough,
        f::ShaderType::Max3dsPbrSpecGloss => ShaderType::E3DsMaxPbrSpecGloss,
        f::ShaderType::GltfMaterial => ShaderType::GltfMaterial,
        f::ShaderType::OpenpbrMaterial => ShaderType::OpenpbrMaterial,
        f::ShaderType::ShaderfxGraph => ShaderType::ShaderfxGraph,
        f::ShaderType::BlenderPhong => ShaderType::BlenderPhong,
        f::ShaderType::WavefrontMtl => ShaderType::WavefrontMtl,
        _ => ShaderType::default(),
    }
}
pub(crate) fn to_enum_ShaderType(c: ShaderType) -> f::ShaderType {
    match c {
        ShaderType::Unknown => f::ShaderType::Unknown,
        ShaderType::FbxLambert => f::ShaderType::FbxLambert,
        ShaderType::FbxPhong => f::ShaderType::FbxPhong,
        ShaderType::OslStandardSurface => f::ShaderType::OslStandardSurface,
        ShaderType::ArnoldStandardSurface => f::ShaderType::ArnoldStandardSurface,
        ShaderType::E3DsMaxPhysicalMaterial => f::ShaderType::Max3dsPhysicalMaterial,
        ShaderType::E3DsMaxPbrMetalRough => f::ShaderType::Max3dsPbrMetalRough,
        ShaderType::E3DsMaxPbrSpecGloss => f::ShaderType::Max3dsPbrSpecGloss,
        ShaderType::GltfMaterial => f::ShaderType::GltfMaterial,
        ShaderType::OpenpbrMaterial => f::ShaderType::OpenpbrMaterial,
        ShaderType::ShaderfxGraph => f::ShaderType::ShaderfxGraph,
        ShaderType::BlenderPhong => f::ShaderType::BlenderPhong,
        ShaderType::WavefrontMtl => f::ShaderType::WavefrontMtl,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u32)]
pub enum TimeMode {
    Default = 0,
    E120Fps = 1,
    E100Fps = 2,
    E60Fps = 3,
    E50Fps = 4,
    E48Fps = 5,
    E30Fps = 6,
    E30FpsDrop = 7,
    NtscDropFrame = 8,
    NtscFullFrame = 9,
    Pal = 10,
    E24Fps = 11,
    E1000Fps = 12,
    FilmFullFrame = 13,
    Custom = 14,
    E96Fps = 15,
    E72Fps = 16,
    E5994Fps = 17,
}
impl Default for TimeMode {
    fn default() -> Self {
        Self::Default
    }
}
pub(crate) fn from_enum_TimeMode(p: f::TimeMode) -> TimeMode {
    #[allow(unreachable_patterns)]
    match p {
        f::TimeMode::Default => TimeMode::Default,
        f::TimeMode::Fps120 => TimeMode::E120Fps,
        f::TimeMode::Fps100 => TimeMode::E100Fps,
        f::TimeMode::Fps60 => TimeMode::E60Fps,
        f::TimeMode::Fps50 => TimeMode::E50Fps,
        f::TimeMode::Fps48 => TimeMode::E48Fps,
        f::TimeMode::Fps30 => TimeMode::E30Fps,
        f::TimeMode::Fps30Drop => TimeMode::E30FpsDrop,
        f::TimeMode::NtscDropFrame => TimeMode::NtscDropFrame,
        f::TimeMode::NtscFullFrame => TimeMode::NtscFullFrame,
        f::TimeMode::Pal => TimeMode::Pal,
        f::TimeMode::Fps24 => TimeMode::E24Fps,
        f::TimeMode::Fps1000 => TimeMode::E1000Fps,
        f::TimeMode::FilmFullFrame => TimeMode::FilmFullFrame,
        f::TimeMode::Custom => TimeMode::Custom,
        f::TimeMode::Fps96 => TimeMode::E96Fps,
        f::TimeMode::Fps72 => TimeMode::E72Fps,
        f::TimeMode::Fps59_94 => TimeMode::E5994Fps,
        _ => TimeMode::default(),
    }
}
pub(crate) fn to_enum_TimeMode(c: TimeMode) -> f::TimeMode {
    match c {
        TimeMode::Default => f::TimeMode::Default,
        TimeMode::E120Fps => f::TimeMode::Fps120,
        TimeMode::E100Fps => f::TimeMode::Fps100,
        TimeMode::E60Fps => f::TimeMode::Fps60,
        TimeMode::E50Fps => f::TimeMode::Fps50,
        TimeMode::E48Fps => f::TimeMode::Fps48,
        TimeMode::E30Fps => f::TimeMode::Fps30,
        TimeMode::E30FpsDrop => f::TimeMode::Fps30Drop,
        TimeMode::NtscDropFrame => f::TimeMode::NtscDropFrame,
        TimeMode::NtscFullFrame => f::TimeMode::NtscFullFrame,
        TimeMode::Pal => f::TimeMode::Pal,
        TimeMode::E24Fps => f::TimeMode::Fps24,
        TimeMode::E1000Fps => f::TimeMode::Fps1000,
        TimeMode::FilmFullFrame => f::TimeMode::FilmFullFrame,
        TimeMode::Custom => f::TimeMode::Custom,
        TimeMode::E96Fps => f::TimeMode::Fps96,
        TimeMode::E72Fps => f::TimeMode::Fps72,
        TimeMode::E5994Fps => f::TimeMode::Fps59_94,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BakedKeyFlags(pub(crate) u32);
impl BakedKeyFlags {
    pub const NONE: BakedKeyFlags = BakedKeyFlags(0);
    pub const STEP_LEFT: BakedKeyFlags = BakedKeyFlags(0x1);
    pub const STEP_RIGHT: BakedKeyFlags = BakedKeyFlags(0x2);
    pub const STEP_KEY: BakedKeyFlags = BakedKeyFlags(0x4);
    pub const KEYFRAME: BakedKeyFlags = BakedKeyFlags(0x8);
    pub const REDUCED: BakedKeyFlags = BakedKeyFlags(0x10);
}
flags_impl!(
    BakedKeyFlags,
    [
        ("NONE", BakedKeyFlags::NONE.0),
        ("STEP_LEFT", BakedKeyFlags::STEP_LEFT.0),
        ("STEP_RIGHT", BakedKeyFlags::STEP_RIGHT.0),
        ("STEP_KEY", BakedKeyFlags::STEP_KEY.0),
        ("KEYFRAME", BakedKeyFlags::KEYFRAME.0),
        ("REDUCED", BakedKeyFlags::REDUCED.0)
    ]
);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EvaluateFlags(pub(crate) u32);
impl EvaluateFlags {
    pub const NONE: EvaluateFlags = EvaluateFlags(0);
    pub const NO_EXTRAPOLATION: EvaluateFlags = EvaluateFlags(0x1);
}
flags_impl!(EvaluateFlags, [("NONE", EvaluateFlags::NONE.0), ("NO_EXTRAPOLATION", EvaluateFlags::NO_EXTRAPOLATION.0)]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PropFlags(pub(crate) u32);
impl PropFlags {
    pub const NONE: PropFlags = PropFlags(0);
    pub const ANIMATABLE: PropFlags = PropFlags(0x1);
    pub const USER_DEFINED: PropFlags = PropFlags(0x2);
    pub const HIDDEN: PropFlags = PropFlags(0x4);
    pub const LOCK_X: PropFlags = PropFlags(0x10);
    pub const LOCK_Y: PropFlags = PropFlags(0x20);
    pub const LOCK_Z: PropFlags = PropFlags(0x40);
    pub const LOCK_W: PropFlags = PropFlags(0x80);
    pub const MUTE_X: PropFlags = PropFlags(0x100);
    pub const MUTE_Y: PropFlags = PropFlags(0x200);
    pub const MUTE_Z: PropFlags = PropFlags(0x400);
    pub const MUTE_W: PropFlags = PropFlags(0x800);
    pub const SYNTHETIC: PropFlags = PropFlags(0x1000);
    pub const ANIMATED: PropFlags = PropFlags(0x2000);
    pub const NOT_FOUND: PropFlags = PropFlags(0x4000);
    pub const CONNECTED: PropFlags = PropFlags(0x8000);
    pub const NO_VALUE: PropFlags = PropFlags(0x10000);
    pub const OVERRIDDEN: PropFlags = PropFlags(0x20000);
    pub const VALUE_REAL: PropFlags = PropFlags(0x100000);
    pub const VALUE_VEC2: PropFlags = PropFlags(0x200000);
    pub const VALUE_VEC3: PropFlags = PropFlags(0x400000);
    pub const VALUE_VEC4: PropFlags = PropFlags(0x800000);
    pub const VALUE_INT: PropFlags = PropFlags(0x1000000);
    pub const VALUE_STR: PropFlags = PropFlags(0x2000000);
    pub const VALUE_BLOB: PropFlags = PropFlags(0x4000000);
}
flags_impl!(
    PropFlags,
    [
        ("NONE", PropFlags::NONE.0),
        ("ANIMATABLE", PropFlags::ANIMATABLE.0),
        ("USER_DEFINED", PropFlags::USER_DEFINED.0),
        ("HIDDEN", PropFlags::HIDDEN.0),
        ("LOCK_X", PropFlags::LOCK_X.0),
        ("LOCK_Y", PropFlags::LOCK_Y.0),
        ("LOCK_Z", PropFlags::LOCK_Z.0),
        ("LOCK_W", PropFlags::LOCK_W.0),
        ("MUTE_X", PropFlags::MUTE_X.0),
        ("MUTE_Y", PropFlags::MUTE_Y.0),
        ("MUTE_Z", PropFlags::MUTE_Z.0),
        ("MUTE_W", PropFlags::MUTE_W.0),
        ("SYNTHETIC", PropFlags::SYNTHETIC.0),
        ("ANIMATED", PropFlags::ANIMATED.0),
        ("NOT_FOUND", PropFlags::NOT_FOUND.0),
        ("CONNECTED", PropFlags::CONNECTED.0),
        ("NO_VALUE", PropFlags::NO_VALUE.0),
        ("OVERRIDDEN", PropFlags::OVERRIDDEN.0),
        ("VALUE_REAL", PropFlags::VALUE_REAL.0),
        ("VALUE_VEC2", PropFlags::VALUE_VEC2.0),
        ("VALUE_VEC3", PropFlags::VALUE_VEC3.0),
        ("VALUE_VEC4", PropFlags::VALUE_VEC4.0),
        ("VALUE_INT", PropFlags::VALUE_INT.0),
        ("VALUE_STR", PropFlags::VALUE_STR.0),
        ("VALUE_BLOB", PropFlags::VALUE_BLOB.0)
    ]
);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TopoFlags(pub(crate) u32);
impl TopoFlags {
    pub const NONE: TopoFlags = TopoFlags(0);
    pub const NON_MANIFOLD: TopoFlags = TopoFlags(0x1);
}
flags_impl!(TopoFlags, [("NONE", TopoFlags::NONE.0), ("NON_MANIFOLD", TopoFlags::NON_MANIFOLD.0)]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TransformFlags(pub(crate) u32);
impl TransformFlags {
    pub const NONE: TransformFlags = TransformFlags(0);
    pub const IGNORE_SCALE_HELPER: TransformFlags = TransformFlags(0x1);
    pub const IGNORE_COMPONENTWISE_SCALE: TransformFlags = TransformFlags(0x2);
    pub const EXPLICIT_INCLUDES: TransformFlags = TransformFlags(0x4);
    pub const INCLUDE_TRANSLATION: TransformFlags = TransformFlags(0x10);
    pub const INCLUDE_ROTATION: TransformFlags = TransformFlags(0x20);
    pub const INCLUDE_SCALE: TransformFlags = TransformFlags(0x40);
    pub const NO_EXTRAPOLATION: TransformFlags = TransformFlags(0x80);
}
flags_impl!(
    TransformFlags,
    [
        ("NONE", TransformFlags::NONE.0),
        ("IGNORE_SCALE_HELPER", TransformFlags::IGNORE_SCALE_HELPER.0),
        ("IGNORE_COMPONENTWISE_SCALE", TransformFlags::IGNORE_COMPONENTWISE_SCALE.0),
        ("EXPLICIT_INCLUDES", TransformFlags::EXPLICIT_INCLUDES.0),
        ("INCLUDE_TRANSLATION", TransformFlags::INCLUDE_TRANSLATION.0),
        ("INCLUDE_ROTATION", TransformFlags::INCLUDE_ROTATION.0),
        ("INCLUDE_SCALE", TransformFlags::INCLUDE_SCALE.0),
        ("NO_EXTRAPOLATION", TransformFlags::NO_EXTRAPOLATION.0)
    ]
);

pub use crate::{
    BakedAnimMetadata, CoordinateAxes, CurvePoint, Edge, Extrapolation, Face, Keyframe, LineSegment, LodLevel, MaterialFeatureInfo, Matrix, Progress, Quat,
    SkinVertex, SkinWeight, SubdivisionWeight, SubdivisionWeightRange, SurfacePoint, Tangent, Transform, TransformOverride, Vec2, Vec3, Vec4,
};

pub struct Anim {
    pub time_begin: f64,
    pub time_end: f64,
    pub layers: RefList<AnimLayer>,
    pub override_layer_weights: List<Real>,
    pub prop_overrides: List<PropOverride>,
    pub transform_overrides: List<TransformOverride>,
    pub ignore_connections: bool,
    pub custom: bool,
    pub(crate) port: Port<f::Anim>,
}
pub(crate) fn from_Anim(k: &Kx, p: &f::Anim) -> Anim {
    let _ = &k;
    Anim {
        time_begin: *(&p.time_begin),
        time_end: *(&p.time_end),
        layers: k.reflist((&p.layers).iter().map(|&i| Ref::new(k.b.anim_layers.wrapping_add(i as usize))).collect()),
        override_layer_weights: List::view(&(&p.override_layer_weights)[..]),
        prop_overrides: k.list((&p.prop_overrides).iter().map(|x| from_PropOverride(k, x)).collect()),
        transform_overrides: List::view(&(&p.transform_overrides)[..]),
        ignore_connections: *(&p.ignore_connections),
        custom: *(&p.custom),
        port: Port::new(p),
    }
}

pub struct AnimCurve {
    pub element: Element,
    pub keyframes: List<Keyframe>,
    pub pre_extrapolation: Extrapolation,
    pub post_extrapolation: Extrapolation,
    pub min_value: Real,
    pub max_value: Real,
    pub min_time: f64,
    pub max_time: f64,
    pub(crate) port: Port<f::AnimCurve>,
}
pub(crate) fn from_AnimCurve(k: &Kx, p: &f::AnimCurve) -> AnimCurve {
    let _ = &k;
    AnimCurve {
        element: from_Element(k, (&p.element)),
        keyframes: List::view(&(&p.keyframes)[..]),
        pre_extrapolation: (&p.pre_extrapolation).clone(),
        post_extrapolation: (&p.post_extrapolation).clone(),
        min_value: *(&p.min_value),
        max_value: *(&p.max_value),
        min_time: *(&p.min_time),
        max_time: *(&p.max_time),
        port: Port::new(p),
    }
}

pub struct AnimLayer {
    pub element: Element,
    pub weight: Real,
    pub weight_is_animated: bool,
    pub blended: bool,
    pub additive: bool,
    pub compose_rotation: bool,
    pub compose_scale: bool,
    pub anim_values: RefList<AnimValue>,
    pub anim_props: List<AnimProp>,
    pub anim: Ref<Anim>,
    pub(crate) _min_element_id: u32,
    pub(crate) _max_element_id: u32,
    pub(crate) _element_id_bitmask: [u32; 4],
    pub(crate) port: Port<f::AnimLayer>,
}
pub(crate) fn from_AnimLayer(k: &Kx, p: &f::AnimLayer) -> AnimLayer {
    let _ = &k;
    AnimLayer {
        element: from_Element(k, (&p.element)),
        weight: *(&p.weight),
        weight_is_animated: *(&p.weight_is_animated),
        blended: *(&p.blended),
        additive: *(&p.additive),
        compose_rotation: *(&p.compose_rotation),
        compose_scale: *(&p.compose_scale),
        anim_values: k.reflist((&p.anim_values).iter().map(|&i| Ref::new(k.b.anim_values.wrapping_add(i as usize))).collect()),
        anim_props: k.list((&p.anim_props).iter().map(|x| from_AnimProp(k, x)).collect()),
        anim: k.cached(std::sync::Arc::as_ptr((&p.anim)), || from_Anim(k, &**(&p.anim))),
        _min_element_id: *(&p.min_element_id) as u32,
        _max_element_id: *(&p.max_element_id) as u32,
        _element_id_bitmask: std::array::from_fn(|i| *(&(&p.element_id_bitmask)[i]) as u32),
        port: Port::new(p),
    }
}

pub struct AnimProp {
    pub element: Ref<Element>,
    pub(crate) _internal_key: u32,
    pub prop_name: String,
    pub anim_value: Ref<AnimValue>,
    pub(crate) port: Port<f::AnimProp>,
}
pub(crate) fn from_AnimProp(k: &Kx, p: &f::AnimProp) -> AnimProp {
    let _ = &k;
    AnimProp {
        element: Ref::new(k.element_id(*(&p.element) as usize)),
        _internal_key: *(&p.internal_key) as u32,
        prop_name: String::view((&p.prop_name)),
        anim_value: Ref::new(k.b.anim_values.wrapping_add(*(&p.anim_value) as usize)),
        port: Port::new(p),
    }
}

pub struct AnimStack {
    pub element: Element,
    pub time_begin: f64,
    pub time_end: f64,
    pub layers: RefList<AnimLayer>,
    pub anim: Ref<Anim>,
    pub(crate) port: Port<f::AnimStack>,
}
pub(crate) fn from_AnimStack(k: &Kx, p: &f::AnimStack) -> AnimStack {
    let _ = &k;
    AnimStack {
        element: from_Element(k, (&p.element)),
        time_begin: *(&p.time_begin),
        time_end: *(&p.time_end),
        layers: k.reflist((&p.layers).iter().map(|&i| Ref::new(k.b.anim_layers.wrapping_add(i as usize))).collect()),
        anim: k.cached(std::sync::Arc::as_ptr((&p.anim)), || from_Anim(k, &**(&p.anim))),
        port: Port::new(p),
    }
}

pub struct AnimValue {
    pub element: Element,
    pub default_value: Vec3,
    pub curves: [Option<Ref<AnimCurve>>; 3],
    pub(crate) port: Port<f::AnimValue>,
}
pub(crate) fn from_AnimValue(k: &Kx, p: &f::AnimValue) -> AnimValue {
    let _ = &k;
    AnimValue {
        element: from_Element(k, (&p.element)),
        default_value: (&p.default_value).clone(),
        curves: std::array::from_fn(|i| (&(&p.curves)[i]).map(|i| k.b.anim_curves.wrapping_add(i as usize)).map(Ref::new)),
        port: Port::new(p),
    }
}

pub struct Application {
    pub vendor: String,
    pub name: String,
    pub version: String,
    pub(crate) port: Port<f::Application>,
}
pub(crate) fn from_Application(k: &Kx, p: &f::Application) -> Application {
    let _ = &k;
    Application { vendor: String::view((&p.vendor)), name: String::view((&p.name)), version: String::view((&p.version)), port: Port::new(p) }
}

pub struct AudioClip {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub content: Blob,
    pub(crate) port: Port<f::AudioClip>,
}
pub(crate) fn from_AudioClip(k: &Kx, p: &f::AudioClip) -> AudioClip {
    let _ = &k;
    AudioClip {
        element: from_Element(k, (&p.element)),
        filename: String::view((&p.filename)),
        absolute_filename: String::view((&p.absolute_filename)),
        relative_filename: String::view((&p.relative_filename)),
        raw_filename: Blob::view((&p.raw_filename)),
        raw_absolute_filename: Blob::view((&p.raw_absolute_filename)),
        raw_relative_filename: Blob::view((&p.raw_relative_filename)),
        content: Blob::view((&p.content)),
        port: Port::new(p),
    }
}

pub struct AudioLayer {
    pub element: Element,
    pub clips: RefList<AudioClip>,
    pub(crate) port: Port<f::AudioLayer>,
}
pub(crate) fn from_AudioLayer(k: &Kx, p: &f::AudioLayer) -> AudioLayer {
    let _ = &k;
    AudioLayer {
        element: from_Element(k, (&p.element)),
        clips: k.reflist((&p.clips).iter().map(|&i| Ref::new(k.b.audio_clips.wrapping_add(i as usize))).collect()),
        port: Port::new(p),
    }
}

pub struct BakedAnim {
    pub nodes: List<BakedNode>,
    pub elements: List<BakedElement>,
    pub playback_time_begin: f64,
    pub playback_time_end: f64,
    pub playback_duration: f64,
    pub key_time_min: f64,
    pub key_time_max: f64,
    pub metadata: BakedAnimMetadata,
    pub(crate) port: Port<f::BakedAnim>,
}
pub(crate) fn from_BakedAnim(k: &Kx, p: &f::BakedAnim) -> BakedAnim {
    let _ = &k;
    BakedAnim {
        nodes: k.list((&p.nodes).iter().map(|x| from_BakedNode(k, x)).collect()),
        elements: k.list((&p.elements).iter().map(|x| from_BakedElement(k, x)).collect()),
        playback_time_begin: *(&p.playback_time_begin),
        playback_time_end: *(&p.playback_time_end),
        playback_duration: *(&p.playback_duration),
        key_time_min: *(&p.key_time_min),
        key_time_max: *(&p.key_time_max),
        metadata: (&p.metadata).clone(),
        port: Port::new(p),
    }
}

pub struct BakedElement {
    pub element_id: u32,
    pub props: List<BakedProp>,
    pub(crate) port: Port<f::BakedElement>,
}
pub(crate) fn from_BakedElement(k: &Kx, p: &f::BakedElement) -> BakedElement {
    let _ = &k;
    BakedElement { element_id: *(&p.element_id) as u32, props: k.list((&p.props).iter().map(|x| from_BakedProp(k, x)).collect()), port: Port::new(p) }
}

pub struct BakedNode {
    pub typed_id: u32,
    pub element_id: u32,
    pub constant_translation: bool,
    pub constant_rotation: bool,
    pub constant_scale: bool,
    pub translation_keys: List<BakedVec3>,
    pub rotation_keys: List<BakedQuat>,
    pub scale_keys: List<BakedVec3>,
    pub(crate) port: Port<f::BakedNode>,
}
pub(crate) fn from_BakedNode(k: &Kx, p: &f::BakedNode) -> BakedNode {
    let _ = &k;
    BakedNode {
        typed_id: *(&p.typed_id) as u32,
        element_id: *(&p.element_id) as u32,
        constant_translation: *(&p.constant_translation),
        constant_rotation: *(&p.constant_rotation),
        constant_scale: *(&p.constant_scale),
        translation_keys: k.list((&p.translation_keys).iter().map(|x| from_BakedVec3(k, x)).collect()),
        rotation_keys: k.list((&p.rotation_keys).iter().map(|x| from_BakedQuat(k, x)).collect()),
        scale_keys: k.list((&p.scale_keys).iter().map(|x| from_BakedVec3(k, x)).collect()),
        port: Port::new(p),
    }
}

pub struct BakedProp {
    pub name: String,
    pub constant_value: bool,
    pub keys: List<BakedVec3>,
    pub(crate) port: Port<f::BakedProp>,
}
pub(crate) fn from_BakedProp(k: &Kx, p: &f::BakedProp) -> BakedProp {
    let _ = &k;
    BakedProp {
        name: String::view((&p.name)),
        constant_value: *(&p.constant_value),
        keys: k.list((&p.keys).iter().map(|x| from_BakedVec3(k, x)).collect()),
        port: Port::new(p),
    }
}

pub struct BakedQuat {
    pub time: f64,
    pub value: Quat,
    pub flags: BakedKeyFlags,
    pub(crate) port: Port<f::BakedQuat>,
}
pub(crate) fn from_BakedQuat(k: &Kx, p: &f::BakedQuat) -> BakedQuat {
    let _ = &k;
    BakedQuat { time: *(&p.time), value: (&p.value).clone(), flags: BakedKeyFlags(*(&p.flags) as u32), port: Port::new(p) }
}

pub struct BakedVec3 {
    pub time: f64,
    pub value: Vec3,
    pub flags: BakedKeyFlags,
    pub(crate) port: Port<f::BakedVec3>,
}
pub(crate) fn from_BakedVec3(k: &Kx, p: &f::BakedVec3) -> BakedVec3 {
    let _ = &k;
    BakedVec3 { time: *(&p.time), value: (&p.value).clone(), flags: BakedKeyFlags(*(&p.flags) as u32), port: Port::new(p) }
}

pub struct BlendChannel {
    pub element: Element,
    pub weight: Real,
    pub keyframes: List<BlendKeyframe>,
    pub target_shape: Option<Ref<BlendShape>>,
    pub(crate) port: Port<f::BlendChannel>,
}
pub(crate) fn from_BlendChannel(k: &Kx, p: &f::BlendChannel) -> BlendChannel {
    let _ = &k;
    BlendChannel {
        element: from_Element(k, (&p.element)),
        weight: *(&p.weight),
        keyframes: k.list((&p.keyframes).iter().map(|x| from_BlendKeyframe(k, x)).collect()),
        target_shape: (&p.target_shape).map(|i| k.b.blend_shapes.wrapping_add(i as usize)).map(Ref::new),
        port: Port::new(p),
    }
}

pub struct BlendDeformer {
    pub element: Element,
    pub channels: RefList<BlendChannel>,
    pub(crate) port: Port<f::BlendDeformer>,
}
pub(crate) fn from_BlendDeformer(k: &Kx, p: &f::BlendDeformer) -> BlendDeformer {
    let _ = &k;
    BlendDeformer {
        element: from_Element(k, (&p.element)),
        channels: k.reflist((&p.channels).iter().map(|&i| Ref::new(k.b.blend_channels.wrapping_add(i as usize))).collect()),
        port: Port::new(p),
    }
}

pub struct BlendKeyframe {
    pub shape: Ref<BlendShape>,
    pub target_weight: Real,
    pub effective_weight: Real,
    pub(crate) port: Port<f::BlendKeyframe>,
}
pub(crate) fn from_BlendKeyframe(k: &Kx, p: &f::BlendKeyframe) -> BlendKeyframe {
    let _ = &k;
    BlendKeyframe {
        shape: Ref::new(k.b.blend_shapes.wrapping_add(*(&p.shape) as usize)),
        target_weight: *(&p.target_weight),
        effective_weight: *(&p.effective_weight),
        port: Port::new(p),
    }
}

pub struct BlendShape {
    pub element: Element,
    pub num_offsets: usize,
    pub offset_vertices: List<u32>,
    pub position_offsets: List<Vec3>,
    pub normal_offsets: List<Vec3>,
    pub offset_weights: List<Real>,
    pub(crate) port: Port<f::BlendShape>,
}
pub(crate) fn from_BlendShape(k: &Kx, p: &f::BlendShape) -> BlendShape {
    let _ = &k;
    BlendShape {
        element: from_Element(k, (&p.element)),
        num_offsets: *(&p.num_offsets) as usize,
        offset_vertices: List::view(&(&p.offset_vertices)[..]),
        position_offsets: List::view(&(&p.position_offsets)[..]),
        normal_offsets: List::view(&(&p.normal_offsets)[..]),
        offset_weights: List::view(&(&p.offset_weights)[..]),
        port: Port::new(p),
    }
}

pub struct Bone {
    pub element: Element,
    pub radius: Real,
    pub relative_length: Real,
    pub is_root: bool,
    pub(crate) port: Port<f::Bone>,
}
pub(crate) fn from_Bone(k: &Kx, p: &f::Bone) -> Bone {
    let _ = &k;
    Bone { element: from_Element(k, (&p.element)), radius: *(&p.radius), relative_length: *(&p.relative_length), is_root: *(&p.is_root), port: Port::new(p) }
}

pub struct BonePose {
    pub bone_node: Ref<Node>,
    pub bone_to_world: Matrix,
    pub bone_to_parent: Matrix,
    pub(crate) port: Port<f::BonePose>,
}
pub(crate) fn from_BonePose(k: &Kx, p: &f::BonePose) -> BonePose {
    let _ = &k;
    BonePose {
        bone_node: Ref::new(k.b.nodes.wrapping_add(*(&p.bone_node) as usize)),
        bone_to_world: (&p.bone_to_world).clone(),
        bone_to_parent: (&p.bone_to_parent).clone(),
        port: Port::new(p),
    }
}

pub struct CacheChannel {
    pub name: String,
    pub interpretation: CacheInterpretation,
    pub interpretation_name: String,
    pub frames: List<CacheFrame>,
    pub mirror_axis: MirrorAxis,
    pub scale_factor: Real,
    pub(crate) port: Port<f::CacheChannel>,
}
pub(crate) fn from_CacheChannel(k: &Kx, p: &f::CacheChannel) -> CacheChannel {
    let _ = &k;
    CacheChannel {
        name: String::view((&p.name)),
        interpretation: *(&p.interpretation),
        interpretation_name: String::view((&p.interpretation_name)),
        frames: k.list((&p.frames).iter().map(|x| from_CacheFrame(k, x)).collect()),
        mirror_axis: *(&p.mirror_axis),
        scale_factor: *(&p.scale_factor),
        port: Port::new(p),
    }
}

pub struct CacheDeformer {
    pub element: Element,
    pub channel: String,
    pub file: Option<Ref<CacheFile>>,
    pub external_cache: Option<Ref<GeometryCache>>,
    pub external_channel: Option<Ref<CacheChannel>>,
    pub(crate) port: Port<f::CacheDeformer>,
}
pub(crate) fn from_CacheDeformer(k: &Kx, p: &f::CacheDeformer) -> CacheDeformer {
    let _ = &k;
    CacheDeformer {
        element: from_Element(k, (&p.element)),
        channel: String::view((&p.channel)),
        file: (&p.file).map(|i| k.b.cache_files.wrapping_add(i as usize)).map(Ref::new),
        external_cache: (&p.external_cache).as_ref().map(|x| k.cached(std::sync::Arc::as_ptr(x), || from_GeometryCache(k, &**x))),
        external_channel: k.channel(&p.external_cache, p.external_channel),
        port: Port::new(p),
    }
}

pub struct CacheFile {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub format: CacheFileFormat,
    pub external_cache: Option<Ref<GeometryCache>>,
    pub(crate) port: Port<f::CacheFile>,
}
pub(crate) fn from_CacheFile(k: &Kx, p: &f::CacheFile) -> CacheFile {
    let _ = &k;
    CacheFile {
        element: from_Element(k, (&p.element)),
        filename: String::view((&p.filename)),
        absolute_filename: String::view((&p.absolute_filename)),
        relative_filename: String::view((&p.relative_filename)),
        raw_filename: Blob::view((&p.raw_filename)),
        raw_absolute_filename: Blob::view((&p.raw_absolute_filename)),
        raw_relative_filename: Blob::view((&p.raw_relative_filename)),
        format: *(&p.format),
        external_cache: (&p.external_cache).as_ref().map(|x| k.cached(std::sync::Arc::as_ptr(x), || from_GeometryCache(k, &**x))),
        port: Port::new(p),
    }
}

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
    pub(crate) port: Port<f::CacheFrame>,
}
pub(crate) fn from_CacheFrame(k: &Kx, p: &f::CacheFrame) -> CacheFrame {
    let _ = &k;
    CacheFrame {
        channel: String::view((&p.channel)),
        time: *(&p.time),
        filename: String::view((&p.filename)),
        file_format: *(&p.file_format),
        mirror_axis: *(&p.mirror_axis),
        scale_factor: *(&p.scale_factor),
        data_format: *(&p.data_format),
        data_encoding: *(&p.data_encoding),
        data_offset: *(&p.data_offset) as u64,
        data_count: *(&p.data_count) as u32,
        data_element_bytes: *(&p.data_element_bytes) as u32,
        data_total_bytes: *(&p.data_total_bytes) as u64,
        port: Port::new(p),
    }
}

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
    pub(crate) port: Port<f::Camera>,
}
pub(crate) fn from_Camera(k: &Kx, p: &f::Camera) -> Camera {
    let _ = &k;
    Camera {
        element: from_Element(k, (&p.element)),
        projection_mode: *(&p.projection_mode),
        resolution_is_pixels: *(&p.resolution_is_pixels),
        resolution: (&p.resolution).clone(),
        field_of_view_deg: (&p.field_of_view_deg).clone(),
        field_of_view_tan: (&p.field_of_view_tan).clone(),
        orthographic_extent: *(&p.orthographic_extent),
        orthographic_size: (&p.orthographic_size).clone(),
        projection_plane: (&p.projection_plane).clone(),
        aspect_ratio: *(&p.aspect_ratio),
        near_plane: *(&p.near_plane),
        far_plane: *(&p.far_plane),
        projection_axes: (&p.projection_axes).clone(),
        aspect_mode: *(&p.aspect_mode),
        aperture_mode: *(&p.aperture_mode),
        gate_fit: *(&p.gate_fit),
        aperture_format: from_enum_ApertureFormat(*(&p.aperture_format)),
        focal_length_mm: *(&p.focal_length_mm),
        film_size_inch: (&p.film_size_inch).clone(),
        aperture_size_inch: (&p.aperture_size_inch).clone(),
        squeeze_ratio: *(&p.squeeze_ratio),
        port: Port::new(p),
    }
}

pub struct CameraSwitcher {
    pub element: Element,
    pub(crate) port: Port<f::CameraSwitcher>,
}
pub(crate) fn from_CameraSwitcher(k: &Kx, p: &f::CameraSwitcher) -> CameraSwitcher {
    let _ = &k;
    CameraSwitcher { element: from_Element(k, (&p.element)), port: Port::new(p) }
}

pub struct Character {
    pub element: Element,
    pub(crate) port: Port<f::Character>,
}
pub(crate) fn from_Character(k: &Kx, p: &f::Character) -> Character {
    let _ = &k;
    Character { element: from_Element(k, (&p.element)), port: Port::new(p) }
}

pub struct ColorSet {
    pub name: String,
    pub index: u32,
    pub vertex_color: VertexVec4,
    pub(crate) port: Port<f::ColorSet>,
}
pub(crate) fn from_ColorSet(k: &Kx, p: &f::ColorSet) -> ColorSet {
    let _ = &k;
    ColorSet { name: String::view((&p.name)), index: *(&p.index) as u32, vertex_color: from_VertexVec4(k, (&p.vertex_color)), port: Port::new(p) }
}

pub struct Connection {
    pub src: Ref<Element>,
    pub dst: Ref<Element>,
    pub src_prop: String,
    pub dst_prop: String,
    pub(crate) port: Port<f::Connection>,
}
pub(crate) fn from_Connection(k: &Kx, p: &f::Connection) -> Connection {
    let _ = &k;
    Connection {
        src: Ref::new(k.element_id(*(&p.src) as usize)),
        dst: Ref::new(k.element_id(*(&p.dst) as usize)),
        src_prop: String::view((&p.src_prop)),
        dst_prop: String::view((&p.dst_prop)),
        port: Port::new(p),
    }
}

pub struct Constraint {
    pub element: Element,
    pub type_: ConstraintType,
    pub type_name: String,
    pub node: Option<Ref<Node>>,
    pub targets: List<ConstraintTarget>,
    pub weight: Real,
    pub active: bool,
    pub constrain_translation: [bool; 3],
    pub constrain_rotation: [bool; 3],
    pub constrain_scale: [bool; 3],
    pub transform_offset: Transform,
    pub aim_vector: Vec3,
    pub aim_up_type: ConstraintAimUpType,
    pub aim_up_node: Option<Ref<Node>>,
    pub aim_up_vector: Vec3,
    pub ik_effector: Option<Ref<Node>>,
    pub ik_end_node: Option<Ref<Node>>,
    pub ik_pole_vector: Vec3,
    pub(crate) port: Port<f::Constraint>,
}
pub(crate) fn from_Constraint(k: &Kx, p: &f::Constraint) -> Constraint {
    let _ = &k;
    Constraint {
        element: from_Element(k, (&p.element)),
        type_: *(&p.constraint_type),
        type_name: String::view((&p.type_name)),
        node: (&p.node).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        targets: k.list((&p.targets).iter().map(|x| from_ConstraintTarget(k, x)).collect()),
        weight: *(&p.weight),
        active: *(&p.active),
        constrain_translation: std::array::from_fn(|i| *(&(&p.constrain_translation)[i])),
        constrain_rotation: std::array::from_fn(|i| *(&(&p.constrain_rotation)[i])),
        constrain_scale: std::array::from_fn(|i| *(&(&p.constrain_scale)[i])),
        transform_offset: (&p.transform_offset).clone(),
        aim_vector: (&p.aim_vector).clone(),
        aim_up_type: *(&p.aim_up_type),
        aim_up_node: (&p.aim_up_node).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        aim_up_vector: (&p.aim_up_vector).clone(),
        ik_effector: (&p.ik_effector).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        ik_end_node: (&p.ik_end_node).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        ik_pole_vector: (&p.ik_pole_vector).clone(),
        port: Port::new(p),
    }
}

pub struct ConstraintTarget {
    pub node: Ref<Node>,
    pub weight: Real,
    pub transform: Transform,
    pub(crate) port: Port<f::ConstraintTarget>,
}
pub(crate) fn from_ConstraintTarget(k: &Kx, p: &f::ConstraintTarget) -> ConstraintTarget {
    let _ = &k;
    ConstraintTarget {
        node: Ref::new(k.b.nodes.wrapping_add(*(&p.node) as usize)),
        weight: *(&p.weight),
        transform: (&p.transform).clone(),
        port: Port::new(p),
    }
}

pub struct DisplayLayer {
    pub element: Element,
    pub nodes: RefList<Node>,
    pub visible: bool,
    pub frozen: bool,
    pub ui_color: Vec3,
    pub(crate) port: Port<f::DisplayLayer>,
}
pub(crate) fn from_DisplayLayer(k: &Kx, p: &f::DisplayLayer) -> DisplayLayer {
    let _ = &k;
    DisplayLayer {
        element: from_Element(k, (&p.element)),
        nodes: k.reflist((&p.nodes).iter().map(|&i| Ref::new(k.b.nodes.wrapping_add(i as usize))).collect()),
        visible: *(&p.visible),
        frozen: *(&p.frozen),
        ui_color: (&p.ui_color).clone(),
        port: Port::new(p),
    }
}

pub struct DomNode {
    pub name: String,
    pub children: RefList<DomNode>,
    pub values: List<DomValue>,
    pub(crate) blobs: List<Blob>,
    pub(crate) port: Port<f::DomNode>,
}
pub(crate) fn from_DomNode(k: &Kx, p: &f::DomNode) -> DomNode {
    let _ = &k;
    DomNode {
        name: String::view((&p.name)),
        children: k.reflist((&p.children).iter().map(|x| k.boxed(from_DomNode(k, x))).collect()),
        values: k.list((&p.values).iter().map(|x| from_DomValue(k, x)).collect()),
        blobs: k.dom_blobs(p),
        port: Port::new(p),
    }
}

pub struct Element {
    pub name: String,
    pub props: Props,
    pub element_id: u32,
    pub typed_id: u32,
    pub instances: RefList<Node>,
    pub type_: ElementType,
    pub connections_src: List<Connection>,
    pub connections_dst: List<Connection>,
    pub dom_node: Option<Ref<DomNode>>,
    pub scene: Ref<Scene>,
    pub(crate) port: Port<f::Element>,
}
pub(crate) fn from_Element(k: &Kx, p: &f::Element) -> Element {
    let _ = &k;
    Element {
        name: String::view((&p.name)),
        props: from_Props(k, (&p.props)),
        element_id: *(&p.element_id) as u32,
        typed_id: *(&p.typed_id) as u32,
        instances: k.reflist((&p.instances).iter().map(|&i| Ref::new(k.b.nodes.wrapping_add(i as usize))).collect()),
        type_: *(&p.element_type),
        connections_src: k.list((&p.connections_src).iter().map(|x| from_Connection(k, x)).collect()),
        connections_dst: k.list((&p.connections_dst).iter().map(|x| from_Connection(k, x)).collect()),
        dom_node: k.dom_ref((&p.dom_node)),
        scene: k.scene_ref(),
        port: Port::new(p),
    }
}

pub struct Empty {
    pub element: Element,
    pub(crate) port: Port<f::Empty>,
}
pub(crate) fn from_Empty(k: &Kx, p: &f::Empty) -> Empty {
    let _ = &k;
    Empty { element: from_Element(k, (&p.element)), port: Port::new(p) }
}

pub struct FaceGroup {
    pub id: i32,
    pub name: String,
    pub(crate) port: Port<f::FaceGroup>,
}
pub(crate) fn from_FaceGroup(k: &Kx, p: &f::FaceGroup) -> FaceGroup {
    let _ = &k;
    FaceGroup { id: *(&p.id) as i32, name: String::view((&p.name)), port: Port::new(p) }
}

pub struct GeometryCache {
    pub root_filename: String,
    pub channels: List<CacheChannel>,
    pub frames: List<CacheFrame>,
    pub extra_info: List<String>,
    pub(crate) port: Port<f::GeometryCache>,
}
pub(crate) fn from_GeometryCache(k: &Kx, p: &f::GeometryCache) -> GeometryCache {
    let _ = &k;
    GeometryCache {
        root_filename: String::view((&p.root_filename)),
        channels: k.list((&p.channels).iter().map(|x| from_CacheChannel(k, x)).collect()),
        frames: k.list((&p.frames).iter().map(|x| from_CacheFrame(k, x)).collect()),
        extra_info: k.list((&p.extra_info).iter().map(|x| String::view(x)).collect()),
        port: Port::new(p),
    }
}

pub struct Light {
    pub element: Element,
    pub color: Vec3,
    pub intensity: Real,
    pub local_direction: Vec3,
    pub type_: LightType,
    pub decay: LightDecay,
    pub area_shape: LightAreaShape,
    pub inner_angle: Real,
    pub outer_angle: Real,
    pub cast_light: bool,
    pub cast_shadows: bool,
    pub(crate) port: Port<f::Light>,
}
pub(crate) fn from_Light(k: &Kx, p: &f::Light) -> Light {
    let _ = &k;
    Light {
        element: from_Element(k, (&p.element)),
        color: (&p.color).clone(),
        intensity: *(&p.intensity),
        local_direction: (&p.local_direction).clone(),
        type_: *(&p.light_type),
        decay: *(&p.decay),
        area_shape: *(&p.area_shape),
        inner_angle: *(&p.inner_angle),
        outer_angle: *(&p.outer_angle),
        cast_light: *(&p.cast_light),
        cast_shadows: *(&p.cast_shadows),
        port: Port::new(p),
    }
}

pub struct LineCurve {
    pub element: Element,
    pub color: Vec3,
    pub control_points: List<Vec3>,
    pub point_indices: List<u32>,
    pub segments: List<LineSegment>,
    pub from_tessellated_nurbs: bool,
    pub(crate) port: Port<f::LineCurve>,
}
pub(crate) fn from_LineCurve(k: &Kx, p: &f::LineCurve) -> LineCurve {
    let _ = &k;
    LineCurve {
        element: from_Element(k, (&p.element)),
        color: (&p.color).clone(),
        control_points: List::view(&(&p.control_points)[..]),
        point_indices: List::view(&(&p.point_indices)[..]),
        segments: List::view(&(&p.segments)[..]),
        from_tessellated_nurbs: *(&p.from_tessellated_nurbs),
        port: Port::new(p),
    }
}

pub struct LodGroup {
    pub element: Element,
    pub relative_distances: bool,
    pub lod_levels: List<LodLevel>,
    pub ignore_parent_transform: bool,
    pub use_distance_limit: bool,
    pub distance_limit_min: Real,
    pub distance_limit_max: Real,
    pub(crate) port: Port<f::LodGroup>,
}
pub(crate) fn from_LodGroup(k: &Kx, p: &f::LodGroup) -> LodGroup {
    let _ = &k;
    LodGroup {
        element: from_Element(k, (&p.element)),
        relative_distances: *(&p.relative_distances),
        lod_levels: List::view(&(&p.lod_levels)[..]),
        ignore_parent_transform: *(&p.ignore_parent_transform),
        use_distance_limit: *(&p.use_distance_limit),
        distance_limit_min: *(&p.distance_limit_min),
        distance_limit_max: *(&p.distance_limit_max),
        port: Port::new(p),
    }
}

pub struct Marker {
    pub element: Element,
    pub type_: MarkerType,
    pub(crate) port: Port<f::Marker>,
}
pub(crate) fn from_Marker(k: &Kx, p: &f::Marker) -> Marker {
    let _ = &k;
    Marker { element: from_Element(k, (&p.element)), type_: *(&p.marker_type), port: Port::new(p) }
}

pub struct Material {
    pub element: Element,
    pub fbx: MaterialFbxMaps,
    pub pbr: MaterialPbrMaps,
    pub features: MaterialFeatures,
    pub shader_type: ShaderType,
    pub shader: Option<Ref<Shader>>,
    pub shading_model_name: String,
    pub shader_prop_prefix: String,
    pub textures: List<MaterialTexture>,
    pub(crate) port: Port<f::Material>,
}
pub(crate) fn from_Material(k: &Kx, p: &f::Material) -> Material {
    let _ = &k;
    Material {
        element: from_Element(k, (&p.element)),
        fbx: from_MaterialFbxMaps(k, (&p.fbx)),
        pbr: from_MaterialPbrMaps(k, (&p.pbr)),
        features: from_MaterialFeatures(k, (&p.features)),
        shader_type: from_enum_ShaderType(*(&p.shader_type)),
        shader: (&p.shader).map(|i| k.b.shaders.wrapping_add(i as usize)).map(Ref::new),
        shading_model_name: String::view((&p.shading_model_name)),
        shader_prop_prefix: String::view((&p.shader_prop_prefix)),
        textures: k.list((&p.textures).iter().map(|x| from_MaterialTexture(k, x)).collect()),
        port: Port::new(p),
    }
}

pub struct MaterialFbxMaps {
    pub diffuse_factor: MaterialMap,
    pub diffuse_color: MaterialMap,
    pub specular_factor: MaterialMap,
    pub specular_color: MaterialMap,
    pub specular_exponent: MaterialMap,
    pub reflection_factor: MaterialMap,
    pub reflection_color: MaterialMap,
    pub transparency_factor: MaterialMap,
    pub transparency_color: MaterialMap,
    pub emission_factor: MaterialMap,
    pub emission_color: MaterialMap,
    pub ambient_factor: MaterialMap,
    pub ambient_color: MaterialMap,
    pub normal_map: MaterialMap,
    pub bump: MaterialMap,
    pub bump_factor: MaterialMap,
    pub displacement_factor: MaterialMap,
    pub displacement: MaterialMap,
    pub vector_displacement_factor: MaterialMap,
    pub vector_displacement: MaterialMap,
    pub(crate) port: Port<f::MaterialFbxMaps>,
}
pub(crate) fn from_MaterialFbxMaps(k: &Kx, p: &f::MaterialFbxMaps) -> MaterialFbxMaps {
    let _ = &k;
    MaterialFbxMaps {
        diffuse_factor: from_MaterialMap(k, p.diffuse_factor()),
        diffuse_color: from_MaterialMap(k, p.diffuse_color()),
        specular_factor: from_MaterialMap(k, p.specular_factor()),
        specular_color: from_MaterialMap(k, p.specular_color()),
        specular_exponent: from_MaterialMap(k, p.specular_exponent()),
        reflection_factor: from_MaterialMap(k, p.reflection_factor()),
        reflection_color: from_MaterialMap(k, p.reflection_color()),
        transparency_factor: from_MaterialMap(k, p.transparency_factor()),
        transparency_color: from_MaterialMap(k, p.transparency_color()),
        emission_factor: from_MaterialMap(k, p.emission_factor()),
        emission_color: from_MaterialMap(k, p.emission_color()),
        ambient_factor: from_MaterialMap(k, p.ambient_factor()),
        ambient_color: from_MaterialMap(k, p.ambient_color()),
        normal_map: from_MaterialMap(k, p.normal_map()),
        bump: from_MaterialMap(k, p.bump()),
        bump_factor: from_MaterialMap(k, p.bump_factor()),
        displacement_factor: from_MaterialMap(k, p.displacement_factor()),
        displacement: from_MaterialMap(k, p.displacement()),
        vector_displacement_factor: from_MaterialMap(k, p.vector_displacement_factor()),
        vector_displacement: from_MaterialMap(k, p.vector_displacement()),
        port: Port::new(p),
    }
}

pub struct MaterialFeatures {
    pub pbr: MaterialFeatureInfo,
    pub metalness: MaterialFeatureInfo,
    pub diffuse: MaterialFeatureInfo,
    pub specular: MaterialFeatureInfo,
    pub emission: MaterialFeatureInfo,
    pub transmission: MaterialFeatureInfo,
    pub coat: MaterialFeatureInfo,
    pub sheen: MaterialFeatureInfo,
    pub opacity: MaterialFeatureInfo,
    pub ambient_occlusion: MaterialFeatureInfo,
    pub matte: MaterialFeatureInfo,
    pub unlit: MaterialFeatureInfo,
    pub ior: MaterialFeatureInfo,
    pub diffuse_roughness: MaterialFeatureInfo,
    pub transmission_roughness: MaterialFeatureInfo,
    pub thin_walled: MaterialFeatureInfo,
    pub caustics: MaterialFeatureInfo,
    pub exit_to_background: MaterialFeatureInfo,
    pub internal_reflections: MaterialFeatureInfo,
    pub double_sided: MaterialFeatureInfo,
    pub roughness_as_glossiness: MaterialFeatureInfo,
    pub coat_roughness_as_glossiness: MaterialFeatureInfo,
    pub transmission_roughness_as_glossiness: MaterialFeatureInfo,
    pub(crate) port: Port<f::MaterialFeatures>,
}
pub(crate) fn from_MaterialFeatures(k: &Kx, p: &f::MaterialFeatures) -> MaterialFeatures {
    let _ = &k;
    MaterialFeatures {
        pbr: (&p[f::MaterialFeature::Pbr]).clone(),
        metalness: (&p[f::MaterialFeature::Metalness]).clone(),
        diffuse: (&p[f::MaterialFeature::Diffuse]).clone(),
        specular: (&p[f::MaterialFeature::Specular]).clone(),
        emission: (&p[f::MaterialFeature::Emission]).clone(),
        transmission: (&p[f::MaterialFeature::Transmission]).clone(),
        coat: (&p[f::MaterialFeature::Coat]).clone(),
        sheen: (&p[f::MaterialFeature::Sheen]).clone(),
        opacity: (&p[f::MaterialFeature::Opacity]).clone(),
        ambient_occlusion: (&p[f::MaterialFeature::AmbientOcclusion]).clone(),
        matte: (&p[f::MaterialFeature::Matte]).clone(),
        unlit: (&p[f::MaterialFeature::Unlit]).clone(),
        ior: (&p[f::MaterialFeature::Ior]).clone(),
        diffuse_roughness: (&p[f::MaterialFeature::DiffuseRoughness]).clone(),
        transmission_roughness: (&p[f::MaterialFeature::TransmissionRoughness]).clone(),
        thin_walled: (&p[f::MaterialFeature::ThinWalled]).clone(),
        caustics: (&p[f::MaterialFeature::Caustics]).clone(),
        exit_to_background: (&p[f::MaterialFeature::ExitToBackground]).clone(),
        internal_reflections: (&p[f::MaterialFeature::InternalReflections]).clone(),
        double_sided: (&p[f::MaterialFeature::DoubleSided]).clone(),
        roughness_as_glossiness: (&p[f::MaterialFeature::RoughnessAsGlossiness]).clone(),
        coat_roughness_as_glossiness: (&p[f::MaterialFeature::CoatRoughnessAsGlossiness]).clone(),
        transmission_roughness_as_glossiness: (&p[f::MaterialFeature::TransmissionRoughnessAsGlossiness]).clone(),
        port: Port::new(p),
    }
}

pub struct MaterialMap {
    pub value_vec4: Vec4,
    pub value_int: i64,
    pub texture: Option<Ref<Texture>>,
    pub has_value: bool,
    pub texture_enabled: bool,
    pub feature_disabled: bool,
    pub value_components: u8,
    pub(crate) port: Port<f::MaterialMap>,
}
pub(crate) fn from_MaterialMap(k: &Kx, p: &f::MaterialMap) -> MaterialMap {
    let _ = &k;
    MaterialMap {
        value_vec4: (&p.value_vec4).clone(),
        value_int: *(&p.value_int) as i64,
        texture: (&p.texture).map(|i| k.b.textures.wrapping_add(i as usize)).map(Ref::new),
        has_value: *(&p.has_value),
        texture_enabled: *(&p.texture_enabled),
        feature_disabled: *(&p.feature_disabled),
        value_components: *(&p.value_components) as u8,
        port: Port::new(p),
    }
}

pub struct MaterialPbrMaps {
    pub base_factor: MaterialMap,
    pub base_color: MaterialMap,
    pub roughness: MaterialMap,
    pub metalness: MaterialMap,
    pub diffuse_roughness: MaterialMap,
    pub specular_factor: MaterialMap,
    pub specular_color: MaterialMap,
    pub specular_ior: MaterialMap,
    pub specular_anisotropy: MaterialMap,
    pub specular_rotation: MaterialMap,
    pub transmission_factor: MaterialMap,
    pub transmission_color: MaterialMap,
    pub transmission_depth: MaterialMap,
    pub transmission_scatter: MaterialMap,
    pub transmission_scatter_anisotropy: MaterialMap,
    pub transmission_dispersion: MaterialMap,
    pub transmission_roughness: MaterialMap,
    pub transmission_extra_roughness: MaterialMap,
    pub transmission_priority: MaterialMap,
    pub transmission_enable_in_aov: MaterialMap,
    pub subsurface_factor: MaterialMap,
    pub subsurface_color: MaterialMap,
    pub subsurface_radius: MaterialMap,
    pub subsurface_scale: MaterialMap,
    pub subsurface_anisotropy: MaterialMap,
    pub subsurface_tint_color: MaterialMap,
    pub subsurface_type: MaterialMap,
    pub sheen_factor: MaterialMap,
    pub sheen_color: MaterialMap,
    pub sheen_roughness: MaterialMap,
    pub coat_factor: MaterialMap,
    pub coat_color: MaterialMap,
    pub coat_roughness: MaterialMap,
    pub coat_ior: MaterialMap,
    pub coat_anisotropy: MaterialMap,
    pub coat_rotation: MaterialMap,
    pub coat_normal: MaterialMap,
    pub coat_affect_base_color: MaterialMap,
    pub coat_affect_base_roughness: MaterialMap,
    pub thin_film_factor: MaterialMap,
    pub thin_film_thickness: MaterialMap,
    pub thin_film_ior: MaterialMap,
    pub emission_factor: MaterialMap,
    pub emission_color: MaterialMap,
    pub opacity: MaterialMap,
    pub indirect_diffuse: MaterialMap,
    pub indirect_specular: MaterialMap,
    pub normal_map: MaterialMap,
    pub tangent_map: MaterialMap,
    pub displacement_map: MaterialMap,
    pub matte_factor: MaterialMap,
    pub matte_color: MaterialMap,
    pub ambient_occlusion: MaterialMap,
    pub glossiness: MaterialMap,
    pub coat_glossiness: MaterialMap,
    pub transmission_glossiness: MaterialMap,
    pub(crate) port: Port<f::MaterialPbrMaps>,
}
pub(crate) fn from_MaterialPbrMaps(k: &Kx, p: &f::MaterialPbrMaps) -> MaterialPbrMaps {
    let _ = &k;
    MaterialPbrMaps {
        base_factor: from_MaterialMap(k, p.base_factor()),
        base_color: from_MaterialMap(k, p.base_color()),
        roughness: from_MaterialMap(k, p.roughness()),
        metalness: from_MaterialMap(k, p.metalness()),
        diffuse_roughness: from_MaterialMap(k, p.diffuse_roughness()),
        specular_factor: from_MaterialMap(k, p.specular_factor()),
        specular_color: from_MaterialMap(k, p.specular_color()),
        specular_ior: from_MaterialMap(k, p.specular_ior()),
        specular_anisotropy: from_MaterialMap(k, p.specular_anisotropy()),
        specular_rotation: from_MaterialMap(k, p.specular_rotation()),
        transmission_factor: from_MaterialMap(k, p.transmission_factor()),
        transmission_color: from_MaterialMap(k, p.transmission_color()),
        transmission_depth: from_MaterialMap(k, p.transmission_depth()),
        transmission_scatter: from_MaterialMap(k, p.transmission_scatter()),
        transmission_scatter_anisotropy: from_MaterialMap(k, p.transmission_scatter_anisotropy()),
        transmission_dispersion: from_MaterialMap(k, p.transmission_dispersion()),
        transmission_roughness: from_MaterialMap(k, p.transmission_roughness()),
        transmission_extra_roughness: from_MaterialMap(k, p.transmission_extra_roughness()),
        transmission_priority: from_MaterialMap(k, p.transmission_priority()),
        transmission_enable_in_aov: from_MaterialMap(k, p.transmission_enable_in_aov()),
        subsurface_factor: from_MaterialMap(k, p.subsurface_factor()),
        subsurface_color: from_MaterialMap(k, p.subsurface_color()),
        subsurface_radius: from_MaterialMap(k, p.subsurface_radius()),
        subsurface_scale: from_MaterialMap(k, p.subsurface_scale()),
        subsurface_anisotropy: from_MaterialMap(k, p.subsurface_anisotropy()),
        subsurface_tint_color: from_MaterialMap(k, p.subsurface_tint_color()),
        subsurface_type: from_MaterialMap(k, p.subsurface_type()),
        sheen_factor: from_MaterialMap(k, p.sheen_factor()),
        sheen_color: from_MaterialMap(k, p.sheen_color()),
        sheen_roughness: from_MaterialMap(k, p.sheen_roughness()),
        coat_factor: from_MaterialMap(k, p.coat_factor()),
        coat_color: from_MaterialMap(k, p.coat_color()),
        coat_roughness: from_MaterialMap(k, p.coat_roughness()),
        coat_ior: from_MaterialMap(k, p.coat_ior()),
        coat_anisotropy: from_MaterialMap(k, p.coat_anisotropy()),
        coat_rotation: from_MaterialMap(k, p.coat_rotation()),
        coat_normal: from_MaterialMap(k, p.coat_normal()),
        coat_affect_base_color: from_MaterialMap(k, p.coat_affect_base_color()),
        coat_affect_base_roughness: from_MaterialMap(k, p.coat_affect_base_roughness()),
        thin_film_factor: from_MaterialMap(k, p.thin_film_factor()),
        thin_film_thickness: from_MaterialMap(k, p.thin_film_thickness()),
        thin_film_ior: from_MaterialMap(k, p.thin_film_ior()),
        emission_factor: from_MaterialMap(k, p.emission_factor()),
        emission_color: from_MaterialMap(k, p.emission_color()),
        opacity: from_MaterialMap(k, p.opacity()),
        indirect_diffuse: from_MaterialMap(k, p.indirect_diffuse()),
        indirect_specular: from_MaterialMap(k, p.indirect_specular()),
        normal_map: from_MaterialMap(k, p.normal_map()),
        tangent_map: from_MaterialMap(k, p.tangent_map()),
        displacement_map: from_MaterialMap(k, p.displacement_map()),
        matte_factor: from_MaterialMap(k, p.matte_factor()),
        matte_color: from_MaterialMap(k, p.matte_color()),
        ambient_occlusion: from_MaterialMap(k, p.ambient_occlusion()),
        glossiness: from_MaterialMap(k, p.glossiness()),
        coat_glossiness: from_MaterialMap(k, p.coat_glossiness()),
        transmission_glossiness: from_MaterialMap(k, p.transmission_glossiness()),
        port: Port::new(p),
    }
}

pub struct MaterialTexture {
    pub material_prop: String,
    pub shader_prop: String,
    pub texture: Ref<Texture>,
    pub(crate) port: Port<f::MaterialTexture>,
}
pub(crate) fn from_MaterialTexture(k: &Kx, p: &f::MaterialTexture) -> MaterialTexture {
    let _ = &k;
    MaterialTexture {
        material_prop: String::view((&p.material_prop)),
        shader_prop: String::view((&p.shader_prop)),
        texture: Ref::new(k.b.textures.wrapping_add(*(&p.texture) as usize)),
        port: Port::new(p),
    }
}

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
    pub faces: List<Face>,
    pub face_smoothing: List<bool>,
    pub face_material: List<u32>,
    pub face_group: List<u32>,
    pub face_hole: List<bool>,
    pub edges: List<Edge>,
    pub edge_smoothing: List<bool>,
    pub edge_crease: List<Real>,
    pub edge_visibility: List<bool>,
    pub vertex_indices: List<u32>,
    pub vertices: List<Vec3>,
    pub vertex_first_index: List<u32>,
    pub vertex_position: VertexVec3,
    pub vertex_normal: VertexVec3,
    pub vertex_uv: VertexVec2,
    pub vertex_tangent: VertexVec3,
    pub vertex_bitangent: VertexVec3,
    pub vertex_color: VertexVec4,
    pub vertex_crease: VertexReal,
    pub uv_sets: List<UvSet>,
    pub color_sets: List<ColorSet>,
    pub materials: RefList<Material>,
    pub face_groups: List<FaceGroup>,
    pub material_parts: List<MeshPart>,
    pub face_group_parts: List<MeshPart>,
    pub material_part_usage_order: List<u32>,
    pub skinned_is_local: bool,
    pub skinned_position: VertexVec3,
    pub skinned_normal: VertexVec3,
    pub skin_deformers: RefList<SkinDeformer>,
    pub blend_deformers: RefList<BlendDeformer>,
    pub cache_deformers: RefList<CacheDeformer>,
    pub all_deformers: RefList<Element>,
    pub subdivision_preview_levels: u32,
    pub subdivision_render_levels: u32,
    pub subdivision_display_mode: SubdivisionDisplayMode,
    pub subdivision_boundary: SubdivisionBoundary,
    pub subdivision_uv_boundary: SubdivisionBoundary,
    pub reversed_winding: bool,
    pub generated_normals: bool,
    pub subdivision_evaluated: bool,
    pub subdivision_result: Option<Ref<SubdivisionResult>>,
    pub from_tessellated_nurbs: bool,
    pub(crate) port: Port<f::Mesh>,
}
pub(crate) fn from_Mesh(k: &Kx, p: &f::Mesh) -> Mesh {
    let _ = &k;
    Mesh {
        element: from_Element(k, (&p.element)),
        num_vertices: *(&p.num_vertices) as usize,
        num_indices: *(&p.num_indices) as usize,
        num_faces: *(&p.num_faces) as usize,
        num_triangles: *(&p.num_triangles) as usize,
        num_edges: *(&p.num_edges) as usize,
        max_face_triangles: *(&p.max_face_triangles) as usize,
        num_empty_faces: *(&p.num_empty_faces) as usize,
        num_point_faces: *(&p.num_point_faces) as usize,
        num_line_faces: *(&p.num_line_faces) as usize,
        faces: List::view(&(&p.faces)[..]),
        face_smoothing: List::view(&(&p.face_smoothing)[..]),
        face_material: List::view(&(&p.face_material)[..]),
        face_group: List::view(&(&p.face_group)[..]),
        face_hole: List::view(&(&p.face_hole)[..]),
        edges: List::view(&(&p.edges)[..]),
        edge_smoothing: List::view(&(&p.edge_smoothing)[..]),
        edge_crease: List::view(&(&p.edge_crease)[..]),
        edge_visibility: List::view(&(&p.edge_visibility)[..]),
        vertex_indices: List::view(&(&p.vertex_indices)[..]),
        vertices: List::view(&(&p.vertices)[..]),
        vertex_first_index: List::view(&(&p.vertex_first_index)[..]),
        vertex_position: from_VertexVec3(k, (&p.vertex_position)),
        vertex_normal: from_VertexVec3(k, (&p.vertex_normal)),
        vertex_uv: from_VertexVec2(k, (&p.vertex_uv)),
        vertex_tangent: from_VertexVec3(k, (&p.vertex_tangent)),
        vertex_bitangent: from_VertexVec3(k, (&p.vertex_bitangent)),
        vertex_color: from_VertexVec4(k, (&p.vertex_color)),
        vertex_crease: from_VertexReal(k, (&p.vertex_crease)),
        uv_sets: k.list((&p.uv_sets).iter().map(|x| from_UvSet(k, x)).collect()),
        color_sets: k.list((&p.color_sets).iter().map(|x| from_ColorSet(k, x)).collect()),
        materials: k.reflist((&p.materials).iter().map(|x| Ref::opt(x.map(|i| k.b.materials.wrapping_add(i as usize)))).collect()),
        face_groups: k.list((&p.face_groups).iter().map(|x| from_FaceGroup(k, x)).collect()),
        material_parts: k.list((&p.material_parts).iter().map(|x| from_MeshPart(k, x)).collect()),
        face_group_parts: k.list((&p.face_group_parts).iter().map(|x| from_MeshPart(k, x)).collect()),
        material_part_usage_order: List::view(&(&p.material_part_usage_order)[..]),
        skinned_is_local: *(&p.skinned_is_local),
        skinned_position: from_VertexVec3(k, (&p.skinned_position)),
        skinned_normal: from_VertexVec3(k, (&p.skinned_normal)),
        skin_deformers: k.reflist((&p.skin_deformers).iter().map(|&i| Ref::new(k.b.skin_deformers.wrapping_add(i as usize))).collect()),
        blend_deformers: k.reflist((&p.blend_deformers).iter().map(|&i| Ref::new(k.b.blend_deformers.wrapping_add(i as usize))).collect()),
        cache_deformers: k.reflist((&p.cache_deformers).iter().map(|&i| Ref::new(k.b.cache_deformers.wrapping_add(i as usize))).collect()),
        all_deformers: k.reflist((&p.all_deformers).iter().map(|&i| Ref::new(k.element_id(i as usize))).collect()),
        subdivision_preview_levels: *(&p.subdivision_preview_levels) as u32,
        subdivision_render_levels: *(&p.subdivision_render_levels) as u32,
        subdivision_display_mode: *(&p.subdivision_display_mode),
        subdivision_boundary: *(&p.subdivision_boundary),
        subdivision_uv_boundary: *(&p.subdivision_uv_boundary),
        reversed_winding: *(&p.reversed_winding),
        generated_normals: *(&p.generated_normals),
        subdivision_evaluated: *(&p.subdivision_evaluated),
        subdivision_result: (&p.subdivision_result).as_deref().map(|x| k.boxed(from_SubdivisionResult(k, x))),
        from_tessellated_nurbs: *(&p.from_tessellated_nurbs),
        port: Port::new(p),
    }
}

pub struct MeshPart {
    pub index: u32,
    pub num_faces: usize,
    pub num_triangles: usize,
    pub num_empty_faces: usize,
    pub num_point_faces: usize,
    pub num_line_faces: usize,
    pub face_indices: List<u32>,
    pub(crate) port: Port<f::MeshPart>,
}
pub(crate) fn from_MeshPart(k: &Kx, p: &f::MeshPart) -> MeshPart {
    let _ = &k;
    MeshPart {
        index: *(&p.index) as u32,
        num_faces: *(&p.num_faces) as usize,
        num_triangles: *(&p.num_triangles) as usize,
        num_empty_faces: *(&p.num_empty_faces) as usize,
        num_point_faces: *(&p.num_point_faces) as usize,
        num_line_faces: *(&p.num_line_faces) as usize,
        face_indices: List::view(&(&p.face_indices)[..]),
        port: Port::new(p),
    }
}

pub struct Metadata {
    pub warnings: List<Warning>,
    pub ascii: bool,
    pub version: u32,
    pub file_format: FileFormat,
    pub may_contain_no_index: bool,
    pub may_contain_missing_vertex_position: bool,
    pub may_contain_broken_elements: bool,
    pub is_unsafe: bool,
    pub has_warning: [bool; 15],
    pub creator: String,
    pub big_endian: bool,
    pub filename: String,
    pub relative_root: String,
    pub raw_filename: Blob,
    pub raw_relative_root: Blob,
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
    pub raw_original_file_path: Blob,
    pub space_conversion: SpaceConversion,
    pub geometry_transform_handling: GeometryTransformHandling,
    pub inherit_mode_handling: InheritModeHandling,
    pub pivot_handling: PivotHandling,
    pub handedness_conversion_axis: MirrorAxis,
    pub root_rotation: Quat,
    pub root_scale: Real,
    pub mirror_axis: MirrorAxis,
    pub geometry_scale: Real,
    pub(crate) port: Port<f::Metadata>,
}
pub(crate) fn from_Metadata(k: &Kx, p: &f::Metadata) -> Metadata {
    let _ = &k;
    Metadata {
        warnings: k.list((&p.warnings).iter().map(|x| from_Warning(k, x)).collect()),
        ascii: *(&p.ascii),
        version: *(&p.version) as u32,
        file_format: from_enum_FileFormat(*(&p.file_format)),
        may_contain_no_index: *(&p.may_contain_no_index),
        may_contain_missing_vertex_position: *(&p.may_contain_missing_vertex_position),
        may_contain_broken_elements: *(&p.may_contain_broken_elements),
        is_unsafe: *(&p.is_unsafe),
        has_warning: std::array::from_fn(|i| *(&(&p.has_warning)[i])),
        creator: String::view((&p.creator)),
        big_endian: *(&p.big_endian),
        filename: String::view((&p.filename)),
        relative_root: String::view((&p.relative_root)),
        raw_filename: Blob::view((&p.raw_filename)),
        raw_relative_root: Blob::view((&p.raw_relative_root)),
        exporter: *(&p.exporter),
        exporter_version: *(&p.exporter_version) as u32,
        scene_props: from_Props(k, (&p.scene_props)),
        original_application: from_Application(k, (&p.original_application)),
        latest_application: from_Application(k, (&p.latest_application)),
        thumbnail: from_Thumbnail(k, (&p.thumbnail)),
        geometry_ignored: *(&p.geometry_ignored),
        animation_ignored: *(&p.animation_ignored),
        embedded_ignored: *(&p.embedded_ignored),
        max_face_triangles: *(&p.max_face_triangles) as usize,
        result_memory_used: Default::default(),
        temp_memory_used: Default::default(),
        result_allocs: Default::default(),
        temp_allocs: Default::default(),
        element_buffer_size: Default::default(),
        num_shader_textures: *(&p.num_shader_textures) as usize,
        bone_prop_size_unit: *(&p.bone_prop_size_unit),
        bone_prop_limb_length_relative: *(&p.bone_prop_limb_length_relative),
        ortho_size_unit: *(&p.ortho_size_unit),
        ktime_second: *(&p.ktime_second) as i64,
        original_file_path: String::view((&p.original_file_path)),
        raw_original_file_path: Blob::view((&p.raw_original_file_path)),
        space_conversion: *(&p.space_conversion),
        geometry_transform_handling: *(&p.geometry_transform_handling),
        inherit_mode_handling: *(&p.inherit_mode_handling),
        pivot_handling: *(&p.pivot_handling),
        handedness_conversion_axis: *(&p.handedness_conversion_axis),
        root_rotation: (&p.root_rotation).clone(),
        root_scale: *(&p.root_scale),
        mirror_axis: *(&p.mirror_axis),
        geometry_scale: *(&p.geometry_scale),
        port: Port::new(p),
    }
}

pub struct MetadataObject {
    pub element: Element,
    pub(crate) port: Port<f::MetadataObject>,
}
pub(crate) fn from_MetadataObject(k: &Kx, p: &f::MetadataObject) -> MetadataObject {
    let _ = &k;
    MetadataObject { element: from_Element(k, (&p.element)), port: Port::new(p) }
}

pub struct NameElement {
    pub name: String,
    pub type_: ElementType,
    pub(crate) _internal_key: u32,
    pub element: Ref<Element>,
    pub(crate) port: Port<f::NameElement>,
}
pub(crate) fn from_NameElement(k: &Kx, p: &f::NameElement) -> NameElement {
    let _ = &k;
    NameElement {
        name: String::view((&p.name)),
        type_: *(&p.element_type),
        _internal_key: *(&p.internal_key) as u32,
        element: Ref::new(k.element_id(*(&p.element) as usize)),
        port: Port::new(p),
    }
}

pub struct Node {
    pub element: Element,
    pub parent: Option<Ref<Node>>,
    pub children: RefList<Node>,
    pub mesh: Option<Ref<Mesh>>,
    pub light: Option<Ref<Light>>,
    pub camera: Option<Ref<Camera>>,
    pub bone: Option<Ref<Bone>>,
    pub attrib: Option<Ref<Element>>,
    pub geometry_transform_helper: Option<Ref<Node>>,
    pub scale_helper: Option<Ref<Node>>,
    pub attrib_type: ElementType,
    pub all_attribs: RefList<Element>,
    pub inherit_mode: InheritMode,
    pub original_inherit_mode: InheritMode,
    pub local_transform: Transform,
    pub geometry_transform: Transform,
    pub inherit_scale: Vec3,
    pub inherit_scale_node: Option<Ref<Node>>,
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
    pub materials: RefList<Material>,
    pub bind_pose: Option<Ref<Pose>>,
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
    pub(crate) port: Port<f::Node>,
}
pub(crate) fn from_Node(k: &Kx, p: &f::Node) -> Node {
    let _ = &k;
    Node {
        element: from_Element(k, (&p.element)),
        parent: (&p.parent).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        children: k.reflist((&p.children).iter().map(|&i| Ref::new(k.b.nodes.wrapping_add(i as usize))).collect()),
        mesh: (&p.mesh).map(|i| k.b.meshes.wrapping_add(i as usize)).map(Ref::new),
        light: (&p.light).map(|i| k.b.lights.wrapping_add(i as usize)).map(Ref::new),
        camera: (&p.camera).map(|i| k.b.cameras.wrapping_add(i as usize)).map(Ref::new),
        bone: (&p.bone).map(|i| k.b.bones.wrapping_add(i as usize)).map(Ref::new),
        attrib: (&p.attrib).map(|i| k.element_id(i as usize)).map(Ref::new),
        geometry_transform_helper: (&p.geometry_transform_helper).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        scale_helper: (&p.scale_helper).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        attrib_type: *(&p.attrib_type),
        all_attribs: k.reflist((&p.all_attribs).iter().map(|&i| Ref::new(k.element_id(i as usize))).collect()),
        inherit_mode: *(&p.inherit_mode),
        original_inherit_mode: *(&p.original_inherit_mode),
        local_transform: (&p.local_transform).clone(),
        geometry_transform: (&p.geometry_transform).clone(),
        inherit_scale: (&p.inherit_scale).clone(),
        inherit_scale_node: (&p.inherit_scale_node).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        rotation_order: *(&p.rotation_order),
        euler_rotation: (&p.euler_rotation).clone(),
        node_to_parent: (&p.node_to_parent).clone(),
        node_to_world: (&p.node_to_world).clone(),
        geometry_to_node: (&p.geometry_to_node).clone(),
        geometry_to_world: (&p.geometry_to_world).clone(),
        unscaled_node_to_world: (&p.unscaled_node_to_world).clone(),
        adjust_pre_translation: (&p.adjust_pre_translation).clone(),
        adjust_pre_rotation: (&p.adjust_pre_rotation).clone(),
        adjust_pre_scale: *(&p.adjust_pre_scale),
        adjust_post_rotation: (&p.adjust_post_rotation).clone(),
        adjust_post_scale: *(&p.adjust_post_scale),
        adjust_translation_scale: *(&p.adjust_translation_scale),
        adjust_mirror_axis: *(&p.adjust_mirror_axis),
        materials: k.reflist((&p.materials).iter().map(|x| Ref::opt(x.map(|i| k.b.materials.wrapping_add(i as usize)))).collect()),
        bind_pose: (&p.bind_pose).map(|i| k.b.poses.wrapping_add(i as usize)).map(Ref::new),
        visible: *(&p.visible),
        is_root: *(&p.is_root),
        has_geometry_transform: *(&p.has_geometry_transform),
        use_rotation_space: *(&p.use_rotation_space),
        has_adjust_transform: *(&p.has_adjust_transform),
        has_root_adjust_transform: *(&p.has_root_adjust_transform),
        is_geometry_transform_helper: *(&p.is_geometry_transform_helper),
        is_scale_helper: *(&p.is_scale_helper),
        is_scale_compensate_parent: *(&p.is_scale_compensate_parent),
        node_depth: *(&p.node_depth) as u32,
        port: Port::new(p),
    }
}

pub struct NurbsBasis {
    pub order: u32,
    pub topology: NurbsTopology,
    pub knot_vector: List<Real>,
    pub t_min: Real,
    pub t_max: Real,
    pub spans: List<Real>,
    pub is_2d: bool,
    pub num_wrap_control_points: usize,
    pub valid: bool,
    pub(crate) port: Port<f::NurbsBasis>,
}
pub(crate) fn from_NurbsBasis(k: &Kx, p: &f::NurbsBasis) -> NurbsBasis {
    let _ = &k;
    NurbsBasis {
        order: *(&p.order) as u32,
        topology: *(&p.topology),
        knot_vector: List::view(&(&p.knot_vector)[..]),
        t_min: *(&p.t_min),
        t_max: *(&p.t_max),
        spans: List::view(&(&p.spans)[..]),
        is_2d: *(&p.is_2d),
        num_wrap_control_points: *(&p.num_wrap_control_points) as usize,
        valid: *(&p.valid),
        port: Port::new(p),
    }
}

pub struct NurbsCurve {
    pub element: Element,
    pub basis: NurbsBasis,
    pub control_points: List<Vec4>,
    pub(crate) port: Port<f::NurbsCurve>,
}
pub(crate) fn from_NurbsCurve(k: &Kx, p: &f::NurbsCurve) -> NurbsCurve {
    let _ = &k;
    NurbsCurve {
        element: from_Element(k, (&p.element)),
        basis: from_NurbsBasis(k, (&p.basis)),
        control_points: List::view(&(&p.control_points)[..]),
        port: Port::new(p),
    }
}

pub struct NurbsSurface {
    pub element: Element,
    pub basis_u: NurbsBasis,
    pub basis_v: NurbsBasis,
    pub num_control_points_u: usize,
    pub num_control_points_v: usize,
    pub control_points: List<Vec4>,
    pub span_subdivision_u: u32,
    pub span_subdivision_v: u32,
    pub flip_normals: bool,
    pub material: Option<Ref<Material>>,
    pub(crate) port: Port<f::NurbsSurface>,
}
pub(crate) fn from_NurbsSurface(k: &Kx, p: &f::NurbsSurface) -> NurbsSurface {
    let _ = &k;
    NurbsSurface {
        element: from_Element(k, (&p.element)),
        basis_u: from_NurbsBasis(k, (&p.basis_u)),
        basis_v: from_NurbsBasis(k, (&p.basis_v)),
        num_control_points_u: *(&p.num_control_points_u) as usize,
        num_control_points_v: *(&p.num_control_points_v) as usize,
        control_points: List::view(&(&p.control_points)[..]),
        span_subdivision_u: *(&p.span_subdivision_u) as u32,
        span_subdivision_v: *(&p.span_subdivision_v) as u32,
        flip_normals: *(&p.flip_normals),
        material: (&p.material).map(|i| k.b.materials.wrapping_add(i as usize)).map(Ref::new),
        port: Port::new(p),
    }
}

pub struct NurbsTrimBoundary {
    pub element: Element,
    pub(crate) port: Port<f::NurbsTrimBoundary>,
}
pub(crate) fn from_NurbsTrimBoundary(k: &Kx, p: &f::NurbsTrimBoundary) -> NurbsTrimBoundary {
    let _ = &k;
    NurbsTrimBoundary { element: from_Element(k, (&p.element)), port: Port::new(p) }
}

pub struct NurbsTrimSurface {
    pub element: Element,
    pub(crate) port: Port<f::NurbsTrimSurface>,
}
pub(crate) fn from_NurbsTrimSurface(k: &Kx, p: &f::NurbsTrimSurface) -> NurbsTrimSurface {
    let _ = &k;
    NurbsTrimSurface { element: from_Element(k, (&p.element)), port: Port::new(p) }
}

pub struct Pose {
    pub element: Element,
    pub is_bind_pose: bool,
    pub bone_poses: List<BonePose>,
    pub(crate) port: Port<f::Pose>,
}
pub(crate) fn from_Pose(k: &Kx, p: &f::Pose) -> Pose {
    let _ = &k;
    Pose {
        element: from_Element(k, (&p.element)),
        is_bind_pose: *(&p.is_bind_pose),
        bone_poses: k.list((&p.bone_poses).iter().map(|x| from_BonePose(k, x)).collect()),
        port: Port::new(p),
    }
}

pub struct ProceduralGeometry {
    pub element: Element,
    pub(crate) port: Port<f::ProceduralGeometry>,
}
pub(crate) fn from_ProceduralGeometry(k: &Kx, p: &f::ProceduralGeometry) -> ProceduralGeometry {
    let _ = &k;
    ProceduralGeometry { element: from_Element(k, (&p.element)), port: Port::new(p) }
}

pub struct Prop {
    pub name: String,
    pub(crate) _internal_key: u32,
    pub type_: PropType,
    pub flags: PropFlags,
    pub value_str: String,
    pub value_blob: Blob,
    pub value_int: i64,
    pub value_vec4: Vec4,
    pub(crate) own: Option<Arc<f::Prop>>,
    pub(crate) port: Port<f::Prop>,
}
pub(crate) fn from_Prop(k: &Kx, p: &f::Prop) -> Prop {
    let _ = &k;
    Prop {
        name: String::view((&p.name)),
        _internal_key: *(&p.internal_key) as u32,
        type_: *(&p.prop_type),
        flags: PropFlags(*(&p.flags) as u32),
        value_str: String::view((&p.value_str)),
        value_blob: Blob::view((&p.value_blob)),
        value_int: *(&p.value_int) as i64,
        value_vec4: Vec4 { x: (&p.value_real_arr)[0], y: (&p.value_real_arr)[1], z: (&p.value_real_arr)[2], w: (&p.value_real_arr)[3] },
        own: None,
        port: Port::new(p),
    }
}

pub struct PropOverride {
    pub element_id: u32,
    pub(crate) _internal_key: u32,
    pub prop_name: String,
    pub value: Vec4,
    pub value_str: String,
    pub value_int: i64,
    pub(crate) port: Port<f::PropOverride>,
}
pub(crate) fn from_PropOverride(k: &Kx, p: &f::PropOverride) -> PropOverride {
    let _ = &k;
    PropOverride {
        element_id: *(&p.element_id) as u32,
        _internal_key: *(&p.internal_key) as u32,
        prop_name: String::view((&p.prop_name)),
        value: (&p.value).clone(),
        value_str: String::view((&p.value_str)),
        value_int: *(&p.value_int) as i64,
        port: Port::new(p),
    }
}

pub struct Props {
    pub props: List<Prop>,
    pub num_animated: usize,
    pub defaults: Option<Ref<Props>>,
    pub(crate) own: Option<Arc<OwnedProps>>,
    pub(crate) port: Port<f::Props>,
}
pub(crate) fn from_Props(k: &Kx, p: &f::Props) -> Props {
    let _ = &k;
    Props {
        props: k.list((&p.props).iter().map(|x| from_Prop(k, x)).collect()),
        num_animated: *(&p.num_animated) as usize,
        defaults: (&p.defaults).as_ref().map(|x| k.cached(std::sync::Arc::as_ptr(x), || from_Props(k, &**x))),
        own: None,
        port: Port::new(p),
    }
}

pub struct Scene {
    pub metadata: Metadata,
    pub settings: SceneSettings,
    pub root_node: Ref<Node>,
    pub anim: Ref<Anim>,
    pub unknowns: RefList<Unknown>,
    pub nodes: RefList<Node>,
    pub meshes: RefList<Mesh>,
    pub lights: RefList<Light>,
    pub cameras: RefList<Camera>,
    pub bones: RefList<Bone>,
    pub empties: RefList<Empty>,
    pub line_curves: RefList<LineCurve>,
    pub nurbs_curves: RefList<NurbsCurve>,
    pub nurbs_surfaces: RefList<NurbsSurface>,
    pub nurbs_trim_surfaces: RefList<NurbsTrimSurface>,
    pub nurbs_trim_boundaries: RefList<NurbsTrimBoundary>,
    pub procedural_geometries: RefList<ProceduralGeometry>,
    pub stereo_cameras: RefList<StereoCamera>,
    pub camera_switchers: RefList<CameraSwitcher>,
    pub markers: RefList<Marker>,
    pub lod_groups: RefList<LodGroup>,
    pub skin_deformers: RefList<SkinDeformer>,
    pub skin_clusters: RefList<SkinCluster>,
    pub blend_deformers: RefList<BlendDeformer>,
    pub blend_channels: RefList<BlendChannel>,
    pub blend_shapes: RefList<BlendShape>,
    pub cache_deformers: RefList<CacheDeformer>,
    pub cache_files: RefList<CacheFile>,
    pub materials: RefList<Material>,
    pub textures: RefList<Texture>,
    pub videos: RefList<Video>,
    pub shaders: RefList<Shader>,
    pub shader_bindings: RefList<ShaderBinding>,
    pub anim_stacks: RefList<AnimStack>,
    pub anim_layers: RefList<AnimLayer>,
    pub anim_values: RefList<AnimValue>,
    pub anim_curves: RefList<AnimCurve>,
    pub display_layers: RefList<DisplayLayer>,
    pub selection_sets: RefList<SelectionSet>,
    pub selection_nodes: RefList<SelectionNode>,
    pub characters: RefList<Character>,
    pub constraints: RefList<Constraint>,
    pub audio_layers: RefList<AudioLayer>,
    pub audio_clips: RefList<AudioClip>,
    pub poses: RefList<Pose>,
    pub metadata_objects: RefList<MetadataObject>,
    pub texture_files: List<TextureFile>,
    pub elements: RefList<Element>,
    pub connections_src: List<Connection>,
    pub connections_dst: List<Connection>,
    pub elements_by_name: List<NameElement>,
    pub dom_root: Option<Ref<DomNode>>,
    pub(crate) bases: Bases,
    pub(crate) owner: Weak<SceneData>,
    pub(crate) port: Port<f::Scene>,
}
pub(crate) fn from_Scene(k: &Kx, p: &f::Scene) -> Scene {
    let _ = &k;
    Scene {
        metadata: from_Metadata(k, (&p.metadata)),
        settings: from_SceneSettings(k, (&p.settings)),
        root_node: Ref::new(k.b.nodes.wrapping_add(*(&p.root_node) as usize)),
        anim: k.cached(std::sync::Arc::as_ptr((&p.anim)), || from_Anim(k, &**(&p.anim))),
        unknowns: k.arena_reflist(k.b.unknowns, (&p.unknowns).len()),
        nodes: k.arena_reflist(k.b.nodes, (&p.nodes).len()),
        meshes: k.arena_reflist(k.b.meshes, (&p.meshes).len()),
        lights: k.arena_reflist(k.b.lights, (&p.lights).len()),
        cameras: k.arena_reflist(k.b.cameras, (&p.cameras).len()),
        bones: k.arena_reflist(k.b.bones, (&p.bones).len()),
        empties: k.arena_reflist(k.b.empties, (&p.empties).len()),
        line_curves: k.arena_reflist(k.b.line_curves, (&p.line_curves).len()),
        nurbs_curves: k.arena_reflist(k.b.nurbs_curves, (&p.nurbs_curves).len()),
        nurbs_surfaces: k.arena_reflist(k.b.nurbs_surfaces, (&p.nurbs_surfaces).len()),
        nurbs_trim_surfaces: k.arena_reflist(k.b.nurbs_trim_surfaces, (&p.nurbs_trim_surfaces).len()),
        nurbs_trim_boundaries: k.arena_reflist(k.b.nurbs_trim_boundaries, (&p.nurbs_trim_boundaries).len()),
        procedural_geometries: k.arena_reflist(k.b.procedural_geometries, (&p.procedural_geometries).len()),
        stereo_cameras: k.arena_reflist(k.b.stereo_cameras, (&p.stereo_cameras).len()),
        camera_switchers: k.arena_reflist(k.b.camera_switchers, (&p.camera_switchers).len()),
        markers: k.arena_reflist(k.b.markers, (&p.markers).len()),
        lod_groups: k.arena_reflist(k.b.lod_groups, (&p.lod_groups).len()),
        skin_deformers: k.arena_reflist(k.b.skin_deformers, (&p.skin_deformers).len()),
        skin_clusters: k.arena_reflist(k.b.skin_clusters, (&p.skin_clusters).len()),
        blend_deformers: k.arena_reflist(k.b.blend_deformers, (&p.blend_deformers).len()),
        blend_channels: k.arena_reflist(k.b.blend_channels, (&p.blend_channels).len()),
        blend_shapes: k.arena_reflist(k.b.blend_shapes, (&p.blend_shapes).len()),
        cache_deformers: k.arena_reflist(k.b.cache_deformers, (&p.cache_deformers).len()),
        cache_files: k.arena_reflist(k.b.cache_files, (&p.cache_files).len()),
        materials: k.arena_reflist(k.b.materials, (&p.materials).len()),
        textures: k.arena_reflist(k.b.textures, (&p.textures).len()),
        videos: k.arena_reflist(k.b.videos, (&p.videos).len()),
        shaders: k.arena_reflist(k.b.shaders, (&p.shaders).len()),
        shader_bindings: k.arena_reflist(k.b.shader_bindings, (&p.shader_bindings).len()),
        anim_stacks: k.arena_reflist(k.b.anim_stacks, (&p.anim_stacks).len()),
        anim_layers: k.arena_reflist(k.b.anim_layers, (&p.anim_layers).len()),
        anim_values: k.arena_reflist(k.b.anim_values, (&p.anim_values).len()),
        anim_curves: k.arena_reflist(k.b.anim_curves, (&p.anim_curves).len()),
        display_layers: k.arena_reflist(k.b.display_layers, (&p.display_layers).len()),
        selection_sets: k.arena_reflist(k.b.selection_sets, (&p.selection_sets).len()),
        selection_nodes: k.arena_reflist(k.b.selection_nodes, (&p.selection_nodes).len()),
        characters: k.arena_reflist(k.b.characters, (&p.characters).len()),
        constraints: k.arena_reflist(k.b.constraints, (&p.constraints).len()),
        audio_layers: k.arena_reflist(k.b.audio_layers, (&p.audio_layers).len()),
        audio_clips: k.arena_reflist(k.b.audio_clips, (&p.audio_clips).len()),
        poses: k.arena_reflist(k.b.poses, (&p.poses).len()),
        metadata_objects: k.arena_reflist(k.b.metadata_objects, (&p.metadata_objects).len()),
        texture_files: k.list((&p.texture_files).iter().map(|x| from_TextureFile(k, x)).collect()),
        elements: k.reflist((&p.elements).iter().map(|r| Ref::new(k.b.element_at(*r))).collect()),
        connections_src: k.list((&p.connections_src).iter().map(|x| from_Connection(k, x)).collect()),
        connections_dst: k.list((&p.connections_dst).iter().map(|x| from_Connection(k, x)).collect()),
        elements_by_name: k.list((&p.elements_by_name).iter().map(|x| from_NameElement(k, x)).collect()),
        dom_root: k.dom_root(),
        bases: k.b,
        owner: k.owner.clone(),
        port: Port::new(p),
    }
}

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
    pub(crate) port: Port<f::SceneSettings>,
}
pub(crate) fn from_SceneSettings(k: &Kx, p: &f::SceneSettings) -> SceneSettings {
    let _ = &k;
    SceneSettings {
        props: from_Props(k, (&p.props)),
        axes: (&p.axes).clone(),
        unit_meters: *(&p.unit_meters),
        frames_per_second: *(&p.frames_per_second),
        ambient_color: (&p.ambient_color).clone(),
        default_camera: String::view((&p.default_camera)),
        time_mode: from_enum_TimeMode(*(&p.time_mode)),
        time_protocol: *(&p.time_protocol),
        snap_mode: *(&p.snap_mode),
        original_axis_up: *(&p.original_axis_up),
        original_unit_meters: *(&p.original_unit_meters),
        port: Port::new(p),
    }
}

pub struct SelectionNode {
    pub element: Element,
    pub target_node: Option<Ref<Node>>,
    pub target_mesh: Option<Ref<Mesh>>,
    pub include_node: bool,
    pub vertices: List<u32>,
    pub edges: List<u32>,
    pub faces: List<u32>,
    pub(crate) port: Port<f::SelectionNode>,
}
pub(crate) fn from_SelectionNode(k: &Kx, p: &f::SelectionNode) -> SelectionNode {
    let _ = &k;
    SelectionNode {
        element: from_Element(k, (&p.element)),
        target_node: (&p.target_node).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        target_mesh: (&p.target_mesh).map(|i| k.b.meshes.wrapping_add(i as usize)).map(Ref::new),
        include_node: *(&p.include_node),
        vertices: List::view(&(&p.vertices)[..]),
        edges: List::view(&(&p.edges)[..]),
        faces: List::view(&(&p.faces)[..]),
        port: Port::new(p),
    }
}

pub struct SelectionSet {
    pub element: Element,
    pub nodes: RefList<SelectionNode>,
    pub(crate) port: Port<f::SelectionSet>,
}
pub(crate) fn from_SelectionSet(k: &Kx, p: &f::SelectionSet) -> SelectionSet {
    let _ = &k;
    SelectionSet {
        element: from_Element(k, (&p.element)),
        nodes: k.reflist((&p.nodes).iter().map(|&i| Ref::new(k.b.selection_nodes.wrapping_add(i as usize))).collect()),
        port: Port::new(p),
    }
}

pub struct Shader {
    pub element: Element,
    pub type_: ShaderType,
    pub bindings: RefList<ShaderBinding>,
    pub(crate) port: Port<f::Shader>,
}
pub(crate) fn from_Shader(k: &Kx, p: &f::Shader) -> Shader {
    let _ = &k;
    Shader {
        element: from_Element(k, (&p.element)),
        type_: from_enum_ShaderType(*(&p.shader_type)),
        bindings: k.reflist((&p.bindings).iter().map(|&i| Ref::new(k.b.shader_bindings.wrapping_add(i as usize))).collect()),
        port: Port::new(p),
    }
}

pub struct ShaderBinding {
    pub element: Element,
    pub prop_bindings: List<ShaderPropBinding>,
    pub(crate) port: Port<f::ShaderBinding>,
}
pub(crate) fn from_ShaderBinding(k: &Kx, p: &f::ShaderBinding) -> ShaderBinding {
    let _ = &k;
    ShaderBinding {
        element: from_Element(k, (&p.element)),
        prop_bindings: k.list((&p.prop_bindings).iter().map(|x| from_ShaderPropBinding(k, x)).collect()),
        port: Port::new(p),
    }
}

pub struct ShaderPropBinding {
    pub shader_prop: String,
    pub material_prop: String,
    pub(crate) port: Port<f::ShaderPropBinding>,
}
pub(crate) fn from_ShaderPropBinding(k: &Kx, p: &f::ShaderPropBinding) -> ShaderPropBinding {
    let _ = &k;
    ShaderPropBinding { shader_prop: String::view((&p.shader_prop)), material_prop: String::view((&p.material_prop)), port: Port::new(p) }
}

pub struct ShaderTexture {
    pub type_: ShaderTextureType,
    pub shader_name: String,
    pub shader_type_id: u64,
    pub inputs: List<ShaderTextureInput>,
    pub shader_source: String,
    pub raw_shader_source: Blob,
    pub main_texture: Ref<Texture>,
    pub main_texture_output_index: i64,
    pub prop_prefix: String,
    pub(crate) port: Port<f::ShaderTexture>,
}
pub(crate) fn from_ShaderTexture(k: &Kx, p: &f::ShaderTexture) -> ShaderTexture {
    let _ = &k;
    ShaderTexture {
        type_: *(&p.shader_type),
        shader_name: String::view((&p.shader_name)),
        shader_type_id: *(&p.shader_type_id) as u64,
        inputs: k.list((&p.inputs).iter().map(|x| from_ShaderTextureInput(k, x)).collect()),
        shader_source: String::view((&p.shader_source)),
        raw_shader_source: Blob::view((&p.raw_shader_source)),
        main_texture: Ref::opt((&p.main_texture).map(|i| k.b.textures.wrapping_add(i as usize))),
        main_texture_output_index: *(&p.main_texture_output_index) as i64,
        prop_prefix: String::view((&p.prop_prefix)),
        port: Port::new(p),
    }
}

pub struct ShaderTextureInput {
    pub name: String,
    pub value_vec4: Vec4,
    pub value_int: i64,
    pub value_str: String,
    pub value_blob: Blob,
    pub texture: Option<Ref<Texture>>,
    pub texture_output_index: i64,
    pub texture_enabled: bool,
    pub prop: Ref<Prop>,
    pub texture_prop: Option<Ref<Prop>>,
    pub texture_enabled_prop: Option<Ref<Prop>>,
    pub(crate) port: Port<f::ShaderTextureInput>,
}
pub(crate) fn from_ShaderTextureInput(k: &Kx, p: &f::ShaderTextureInput) -> ShaderTextureInput {
    let _ = &k;
    ShaderTextureInput {
        name: String::view((&p.name)),
        value_vec4: (&p.value_vec4).clone(),
        value_int: *(&p.value_int) as i64,
        value_str: String::view((&p.value_str)),
        value_blob: Blob::view((&p.value_blob)),
        texture: (&p.texture).map(|i| k.b.textures.wrapping_add(i as usize)).map(Ref::new),
        texture_output_index: *(&p.texture_output_index) as i64,
        texture_enabled: *(&p.texture_enabled),
        prop: k.prop_named((&p.prop)),
        texture_prop: (&p.texture_prop).as_ref().map(|s| k.prop_named(s)),
        texture_enabled_prop: (&p.texture_enabled_prop).as_ref().map(|s| k.prop_named(s)),
        port: Port::new(p),
    }
}

pub struct SkinCluster {
    pub element: Element,
    pub bone_node: Option<Ref<Node>>,
    pub geometry_to_bone: Matrix,
    pub mesh_node_to_bone: Matrix,
    pub bind_to_world: Matrix,
    pub geometry_to_world: Matrix,
    pub geometry_to_world_transform: Transform,
    pub num_weights: usize,
    pub vertices: List<u32>,
    pub weights: List<Real>,
    pub(crate) port: Port<f::SkinCluster>,
}
pub(crate) fn from_SkinCluster(k: &Kx, p: &f::SkinCluster) -> SkinCluster {
    let _ = &k;
    SkinCluster {
        element: from_Element(k, (&p.element)),
        bone_node: (&p.bone_node).map(|i| k.b.nodes.wrapping_add(i as usize)).map(Ref::new),
        geometry_to_bone: (&p.geometry_to_bone).clone(),
        mesh_node_to_bone: (&p.mesh_node_to_bone).clone(),
        bind_to_world: (&p.bind_to_world).clone(),
        geometry_to_world: (&p.geometry_to_world).clone(),
        geometry_to_world_transform: (&p.geometry_to_world_transform).clone(),
        num_weights: *(&p.num_weights) as usize,
        vertices: List::view(&(&p.vertices)[..]),
        weights: List::view(&(&p.weights)[..]),
        port: Port::new(p),
    }
}

pub struct SkinDeformer {
    pub element: Element,
    pub skinning_method: SkinningMethod,
    pub clusters: RefList<SkinCluster>,
    pub vertices: List<SkinVertex>,
    pub weights: List<SkinWeight>,
    pub max_weights_per_vertex: usize,
    pub num_dq_weights: usize,
    pub dq_vertices: List<u32>,
    pub dq_weights: List<Real>,
    pub(crate) port: Port<f::SkinDeformer>,
}
pub(crate) fn from_SkinDeformer(k: &Kx, p: &f::SkinDeformer) -> SkinDeformer {
    let _ = &k;
    SkinDeformer {
        element: from_Element(k, (&p.element)),
        skinning_method: *(&p.skinning_method),
        clusters: k.reflist((&p.clusters).iter().map(|&i| Ref::new(k.b.skin_clusters.wrapping_add(i as usize))).collect()),
        vertices: List::view(&(&p.vertices)[..]),
        weights: List::view(&(&p.weights)[..]),
        max_weights_per_vertex: *(&p.max_weights_per_vertex) as usize,
        num_dq_weights: *(&p.num_dq_weights) as usize,
        dq_vertices: List::view(&(&p.dq_vertices)[..]),
        dq_weights: List::view(&(&p.dq_weights)[..]),
        port: Port::new(p),
    }
}

pub struct StereoCamera {
    pub element: Element,
    pub left: Option<Ref<Camera>>,
    pub right: Option<Ref<Camera>>,
    pub(crate) port: Port<f::StereoCamera>,
}
pub(crate) fn from_StereoCamera(k: &Kx, p: &f::StereoCamera) -> StereoCamera {
    let _ = &k;
    StereoCamera {
        element: from_Element(k, (&p.element)),
        left: (&p.left).map(|i| k.b.cameras.wrapping_add(i as usize)).map(Ref::new),
        right: (&p.right).map(|i| k.b.cameras.wrapping_add(i as usize)).map(Ref::new),
        port: Port::new(p),
    }
}

pub struct SubdivisionResult {
    pub result_memory_used: usize,
    pub temp_memory_used: usize,
    pub result_allocs: usize,
    pub temp_allocs: usize,
    pub source_vertex_ranges: List<SubdivisionWeightRange>,
    pub source_vertex_weights: List<SubdivisionWeight>,
    pub skin_cluster_ranges: List<SubdivisionWeightRange>,
    pub skin_cluster_weights: List<SubdivisionWeight>,
    pub(crate) port: Port<f::SubdivisionResult>,
}
pub(crate) fn from_SubdivisionResult(k: &Kx, p: &f::SubdivisionResult) -> SubdivisionResult {
    let _ = &k;
    SubdivisionResult {
        result_memory_used: Default::default(),
        temp_memory_used: Default::default(),
        result_allocs: Default::default(),
        temp_allocs: Default::default(),
        source_vertex_ranges: List::view(&(&p.source_vertex_ranges)[..]),
        source_vertex_weights: List::view(&(&p.source_vertex_weights)[..]),
        skin_cluster_ranges: List::view(&(&p.skin_cluster_ranges)[..]),
        skin_cluster_weights: List::view(&(&p.skin_cluster_weights)[..]),
        port: Port::new(p),
    }
}

pub struct Texture {
    pub element: Element,
    pub type_: TextureType,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub content: Blob,
    pub video: Option<Ref<Video>>,
    pub file_index: u32,
    pub has_file: bool,
    pub layers: List<TextureLayer>,
    pub shader: Option<Ref<ShaderTexture>>,
    pub file_textures: RefList<Texture>,
    pub uv_set: String,
    pub wrap_u: WrapMode,
    pub wrap_v: WrapMode,
    pub has_uv_transform: bool,
    pub uv_transform: Transform,
    pub texture_to_uv: Matrix,
    pub uv_to_texture: Matrix,
    pub(crate) port: Port<f::Texture>,
}
pub(crate) fn from_Texture(k: &Kx, p: &f::Texture) -> Texture {
    let _ = &k;
    k.set_current(&p.element);
    Texture {
        element: from_Element(k, (&p.element)),
        type_: *(&p.texture_type),
        filename: String::view((&p.filename)),
        absolute_filename: String::view((&p.absolute_filename)),
        relative_filename: String::view((&p.relative_filename)),
        raw_filename: Blob::view((&p.raw_filename)),
        raw_absolute_filename: Blob::view((&p.raw_absolute_filename)),
        raw_relative_filename: Blob::view((&p.raw_relative_filename)),
        content: Blob::view((&p.content)),
        video: (&p.video).map(|i| k.b.videos.wrapping_add(i as usize)).map(Ref::new),
        file_index: *(&p.file_index) as u32,
        has_file: *(&p.has_file),
        layers: k.list((&p.layers).iter().map(|x| from_TextureLayer(k, x)).collect()),
        shader: (&p.shader).as_deref().map(|x| k.boxed(from_ShaderTexture(k, x))),
        file_textures: k.reflist((&p.file_textures).iter().map(|&i| Ref::new(k.b.textures.wrapping_add(i as usize))).collect()),
        uv_set: String::view((&p.uv_set)),
        wrap_u: *(&p.wrap_u),
        wrap_v: *(&p.wrap_v),
        has_uv_transform: *(&p.has_uv_transform),
        uv_transform: (&p.uv_transform).clone(),
        texture_to_uv: (&p.texture_to_uv).clone(),
        uv_to_texture: (&p.uv_to_texture).clone(),
        port: Port::new(p),
    }
}

pub struct TextureFile {
    pub index: u32,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub content: Blob,
    pub(crate) port: Port<f::TextureFile>,
}
pub(crate) fn from_TextureFile(k: &Kx, p: &f::TextureFile) -> TextureFile {
    let _ = &k;
    TextureFile {
        index: *(&p.index) as u32,
        filename: String::view((&p.filename)),
        absolute_filename: String::view((&p.absolute_filename)),
        relative_filename: String::view((&p.relative_filename)),
        raw_filename: Blob::view((&p.raw_filename)),
        raw_absolute_filename: Blob::view((&p.raw_absolute_filename)),
        raw_relative_filename: Blob::view((&p.raw_relative_filename)),
        content: Blob::view((&p.content)),
        port: Port::new(p),
    }
}

pub struct TextureLayer {
    pub texture: Ref<Texture>,
    pub blend_mode: BlendMode,
    pub alpha: Real,
    pub(crate) port: Port<f::TextureLayer>,
}
pub(crate) fn from_TextureLayer(k: &Kx, p: &f::TextureLayer) -> TextureLayer {
    let _ = &k;
    TextureLayer {
        texture: Ref::new(k.b.textures.wrapping_add(*(&p.texture) as usize)),
        blend_mode: from_enum_BlendMode(*(&p.blend_mode)),
        alpha: *(&p.alpha),
        port: Port::new(p),
    }
}

pub struct Thumbnail {
    pub props: Props,
    pub width: u32,
    pub height: u32,
    pub format: ThumbnailFormat,
    pub data: Blob,
    pub(crate) port: Port<f::Thumbnail>,
}
pub(crate) fn from_Thumbnail(k: &Kx, p: &f::Thumbnail) -> Thumbnail {
    let _ = &k;
    Thumbnail {
        props: from_Props(k, (&p.props)),
        width: *(&p.width) as u32,
        height: *(&p.height) as u32,
        format: *(&p.format),
        data: Blob::view((&p.data)),
        port: Port::new(p),
    }
}

pub struct TopoEdge {
    pub index: u32,
    pub next: u32,
    pub prev: u32,
    pub twin: u32,
    pub face: u32,
    pub edge: u32,
    pub flags: TopoFlags,
    pub(crate) port: Port<f::TopoEdge>,
}
pub(crate) fn from_TopoEdge(k: &Kx, p: &f::TopoEdge) -> TopoEdge {
    let _ = &k;
    TopoEdge {
        index: *(&p.index) as u32,
        next: *(&p.next) as u32,
        prev: *(&p.prev) as u32,
        twin: *(&p.twin) as u32,
        face: *(&p.face) as u32,
        edge: *(&p.edge) as u32,
        flags: TopoFlags(*(&p.flags) as u32),
        port: Port::new(p),
    }
}

pub struct Unknown {
    pub element: Element,
    pub type_: String,
    pub super_type: String,
    pub sub_type: String,
    pub(crate) port: Port<f::Unknown>,
}
pub(crate) fn from_Unknown(k: &Kx, p: &f::Unknown) -> Unknown {
    let _ = &k;
    Unknown {
        element: from_Element(k, (&p.element)),
        type_: String::view((&p.type_)),
        super_type: String::view((&p.super_type)),
        sub_type: String::view((&p.sub_type)),
        port: Port::new(p),
    }
}

pub struct UvSet {
    pub name: String,
    pub index: u32,
    pub vertex_uv: VertexVec2,
    pub vertex_tangent: VertexVec3,
    pub vertex_bitangent: VertexVec3,
    pub(crate) port: Port<f::UvSet>,
}
pub(crate) fn from_UvSet(k: &Kx, p: &f::UvSet) -> UvSet {
    let _ = &k;
    UvSet {
        name: String::view((&p.name)),
        index: *(&p.index) as u32,
        vertex_uv: from_VertexVec2(k, (&p.vertex_uv)),
        vertex_tangent: from_VertexVec3(k, (&p.vertex_tangent)),
        vertex_bitangent: from_VertexVec3(k, (&p.vertex_bitangent)),
        port: Port::new(p),
    }
}

pub struct VertexReal {
    pub exists: bool,
    pub values: List<Real>,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
    pub(crate) port: Port<f::VertexAttrib<f64>>,
}
pub(crate) fn from_VertexReal(k: &Kx, p: &f::VertexAttrib<f64>) -> VertexReal {
    let _ = &k;
    VertexReal {
        exists: *(&p.exists),
        values: List::view(&(&p.values)[..]),
        indices: List::view(&(&p.indices)[..]),
        value_reals: *(&p.value_reals) as usize,
        unique_per_vertex: *(&p.unique_per_vertex),
        values_w: List::view(&(&p.values_w)[..]),
        port: Port::new(p),
    }
}

pub struct VertexVec2 {
    pub exists: bool,
    pub values: List<Vec2>,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
    pub(crate) port: Port<f::VertexAttrib<f::Vec2>>,
}
pub(crate) fn from_VertexVec2(k: &Kx, p: &f::VertexAttrib<f::Vec2>) -> VertexVec2 {
    let _ = &k;
    VertexVec2 {
        exists: *(&p.exists),
        values: List::view(&(&p.values)[..]),
        indices: List::view(&(&p.indices)[..]),
        value_reals: *(&p.value_reals) as usize,
        unique_per_vertex: *(&p.unique_per_vertex),
        values_w: List::view(&(&p.values_w)[..]),
        port: Port::new(p),
    }
}

pub struct VertexVec3 {
    pub exists: bool,
    pub values: List<Vec3>,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
    pub(crate) port: Port<f::VertexAttrib<f::Vec3>>,
}
pub(crate) fn from_VertexVec3(k: &Kx, p: &f::VertexAttrib<f::Vec3>) -> VertexVec3 {
    let _ = &k;
    VertexVec3 {
        exists: *(&p.exists),
        values: List::view(&(&p.values)[..]),
        indices: List::view(&(&p.indices)[..]),
        value_reals: *(&p.value_reals) as usize,
        unique_per_vertex: *(&p.unique_per_vertex),
        values_w: List::view(&(&p.values_w)[..]),
        port: Port::new(p),
    }
}

pub struct VertexVec4 {
    pub exists: bool,
    pub values: List<Vec4>,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
    pub(crate) port: Port<f::VertexAttrib<f::Vec4>>,
}
pub(crate) fn from_VertexVec4(k: &Kx, p: &f::VertexAttrib<f::Vec4>) -> VertexVec4 {
    let _ = &k;
    VertexVec4 {
        exists: *(&p.exists),
        values: List::view(&(&p.values)[..]),
        indices: List::view(&(&p.indices)[..]),
        value_reals: *(&p.value_reals) as usize,
        unique_per_vertex: *(&p.unique_per_vertex),
        values_w: List::view(&(&p.values_w)[..]),
        port: Port::new(p),
    }
}

pub struct Video {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub content: Blob,
    pub(crate) port: Port<f::Video>,
}
pub(crate) fn from_Video(k: &Kx, p: &f::Video) -> Video {
    let _ = &k;
    Video {
        element: from_Element(k, (&p.element)),
        filename: String::view((&p.filename)),
        absolute_filename: String::view((&p.absolute_filename)),
        relative_filename: String::view((&p.relative_filename)),
        raw_filename: Blob::view((&p.raw_filename)),
        raw_absolute_filename: Blob::view((&p.raw_absolute_filename)),
        raw_relative_filename: Blob::view((&p.raw_relative_filename)),
        content: Blob::view((&p.content)),
        port: Port::new(p),
    }
}

pub struct Warning {
    pub type_: WarningType,
    pub description: String,
    pub element_id: u32,
    pub count: usize,
    pub(crate) port: Port<f::Warning>,
}
pub(crate) fn from_Warning(k: &Kx, p: &f::Warning) -> Warning {
    let _ = &k;
    Warning {
        type_: *(&p.kind),
        description: String::view((&p.description)),
        element_id: *(&p.element_id) as u32,
        count: *(&p.count) as usize,
        port: Port::new(p),
    }
}

#[derive(Default)]
pub struct AllocatorOpts {
    pub allocator: Allocator,
    pub memory_limit: usize,
    pub allocation_limit: usize,
    pub huge_threshold: usize,
    pub max_chunk_size: usize,
}
impl AllocatorOpts {
    pub(crate) fn to_port(&mut self) -> f::AllocatorOpts {
        let mut o = f::AllocatorOpts::default();
        o.memory_limit = self.memory_limit;
        o.allocation_limit = self.allocation_limit;
        o.huge_threshold = self.huge_threshold;
        o.max_chunk_size = self.max_chunk_size;
        o
    }
}

#[derive(Default)]
pub struct OpenFileOpts {
    pub allocator: AllocatorOpts,
    pub filename_null_terminated: Unsafe<bool>,
}

#[derive(Default)]
pub struct OpenMemoryOpts<'a> {
    pub allocator: AllocatorOpts,
    pub no_copy: Unsafe<bool>,
    pub close_cb: CloseMemoryCb<'a>,
}

#[derive(Default)]
pub struct ThreadOpts {
    pub pool: ThreadPool,
    pub num_tasks: usize,
    pub memory_limit: usize,
}

#[derive(Default)]
pub struct LoadOpts<'a> {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub thread_opts: ThreadOpts,
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
    pub allow_unsafe: Unsafe<bool>,
    pub index_error_handling: IndexErrorHandling,
    pub connect_broken_elements: bool,
    pub allow_nodes_out_of_root: bool,
    pub allow_missing_vertex_position: bool,
    pub allow_empty_faces: bool,
    pub generate_missing_normals: bool,
    pub open_main_file_with_default: bool,
    pub path_separator: u8,
    pub node_depth_limit: u32,
    pub file_size_estimate: u64,
    pub read_buffer_size: usize,
    pub filename: StringOpt<'a>,
    pub raw_filename: BlobOpt<'a>,
    pub progress_cb: ProgressCb<'a>,
    pub progress_interval_hint: u64,
    pub open_file_cb: OpenFileCb<'a>,
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
    pub geometry_transform_helper_name: StringOpt<'a>,
    pub scale_helper_name: StringOpt<'a>,
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
    pub obj_mtl_path: StringOpt<'a>,
    pub obj_mtl_data: BlobOpt<'a>,
    pub obj_unit_meters: Real,
    pub obj_axes: CoordinateAxes,
}
impl<'a> LoadOpts<'a> {
    pub(crate) fn to_port(&mut self) -> f::LoadOpts {
        let mut o = f::LoadOpts::default();
        o.temp_allocator = self.temp_allocator.to_port();
        o.result_allocator = self.result_allocator.to_port();
        o.ignore_geometry = self.ignore_geometry;
        o.ignore_animation = self.ignore_animation;
        o.ignore_embedded = self.ignore_embedded;
        o.ignore_all_content = self.ignore_all_content;
        o.evaluate_skinning = self.evaluate_skinning;
        o.evaluate_caches = self.evaluate_caches;
        o.load_external_files = self.load_external_files;
        o.ignore_missing_external_files = self.ignore_missing_external_files;
        o.skip_skin_vertices = self.skip_skin_vertices;
        o.skip_mesh_parts = self.skip_mesh_parts;
        o.clean_skin_weights = self.clean_skin_weights;
        o.use_blender_pbr_material = self.use_blender_pbr_material;
        o.disable_quirks = self.disable_quirks;
        o.strict = self.strict;
        o.force_single_thread_ascii_parsing = self.force_single_thread_ascii_parsing;
        o.allow_unsafe = self.allow_unsafe.0;
        o.index_error_handling = self.index_error_handling;
        o.connect_broken_elements = self.connect_broken_elements;
        o.allow_nodes_out_of_root = self.allow_nodes_out_of_root;
        o.allow_missing_vertex_position = self.allow_missing_vertex_position;
        o.allow_empty_faces = self.allow_empty_faces;
        o.generate_missing_normals = self.generate_missing_normals;
        o.open_main_file_with_default = self.open_main_file_with_default;
        o.path_separator = self.path_separator;
        o.node_depth_limit = self.node_depth_limit;
        o.file_size_estimate = self.file_size_estimate;
        o.read_buffer_size = self.read_buffer_size;
        o.filename = self.filename.to_port();
        o.raw_filename = self.raw_filename.to_port();
        o.progress_cb = self.progress_cb.to_port();
        o.progress_interval_hint = self.progress_interval_hint;
        o.open_file_cb = self.open_file_cb.to_port();
        o.geometry_transform_handling = self.geometry_transform_handling;
        o.inherit_mode_handling = self.inherit_mode_handling;
        o.space_conversion = self.space_conversion;
        o.pivot_handling = self.pivot_handling;
        o.pivot_handling_retain_empties = self.pivot_handling_retain_empties;
        o.handedness_conversion_axis = self.handedness_conversion_axis;
        o.handedness_conversion_retain_winding = self.handedness_conversion_retain_winding;
        o.reverse_winding = self.reverse_winding;
        o.target_axes = self.target_axes.clone();
        o.target_unit_meters = self.target_unit_meters;
        o.target_camera_axes = self.target_camera_axes.clone();
        o.target_light_axes = self.target_light_axes.clone();
        o.geometry_transform_helper_name = self.geometry_transform_helper_name.to_port();
        o.scale_helper_name = self.scale_helper_name.to_port();
        o.normalize_normals = self.normalize_normals;
        o.normalize_tangents = self.normalize_tangents;
        o.use_root_transform = self.use_root_transform;
        o.root_transform = self.root_transform.clone();
        o.key_clamp_threshold = self.key_clamp_threshold;
        o.unicode_error_handling = self.unicode_error_handling;
        o.retain_vertex_attrib_w = self.retain_vertex_attrib_w;
        o.retain_dom = self.retain_dom;
        o.file_format = to_enum_FileFormat(self.file_format);
        o.file_format_lookahead = self.file_format_lookahead;
        o.no_format_from_content = self.no_format_from_content;
        o.no_format_from_extension = self.no_format_from_extension;
        o.obj_search_mtl_by_filename = self.obj_search_mtl_by_filename;
        o.obj_merge_objects = self.obj_merge_objects;
        o.obj_merge_groups = self.obj_merge_groups;
        o.obj_split_groups = self.obj_split_groups;
        o.obj_mtl_path = self.obj_mtl_path.to_port();
        o.obj_mtl_data = self.obj_mtl_data.to_port();
        o.obj_unit_meters = self.obj_unit_meters;
        o.obj_axes = self.obj_axes.clone();
        o
    }
}

#[derive(Default)]
pub struct EvaluateOpts<'a> {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub evaluate_skinning: bool,
    pub evaluate_caches: bool,
    pub evaluate_flags: u32,
    pub load_external_files: bool,
    pub open_file_cb: OpenFileCb<'a>,
}
impl<'a> EvaluateOpts<'a> {
    pub(crate) fn to_port(&mut self) -> f::EvaluateOpts {
        let mut o = f::EvaluateOpts::default();
        o.evaluate_skinning = self.evaluate_skinning;
        o.evaluate_caches = self.evaluate_caches;
        o.evaluate_flags = self.evaluate_flags;
        o.load_external_files = self.load_external_files;
        o.open_file_cb = self.open_file_cb.to_port();
        o
    }
}

#[derive(Default)]
pub struct PropOverrideDesc<'a> {
    pub element_id: u32,
    pub prop_name: StringOpt<'a>,
    pub value: Vec4,
    pub value_str: StringOpt<'a>,
    pub value_int: i64,
}
impl<'a> PropOverrideDesc<'a> {
    pub(crate) fn to_port(&mut self) -> f::PropOverrideDesc {
        let mut o = f::PropOverrideDesc::default();
        o.element_id = self.element_id;
        o.prop_name = self.prop_name.to_port();
        o.value = self.value.clone();
        o.value_str = self.value_str.to_port();
        o.value_int = self.value_int;
        o
    }
}

#[derive(Default)]
pub struct AnimOpts<'a> {
    pub layer_ids: ListOpt<'a, u32>,
    pub override_layer_weights: ListOpt<'a, Real>,
    pub prop_overrides: ListOpt<'a, PropOverrideDesc<'a>>,
    pub transform_overrides: ListOpt<'a, TransformOverride>,
    pub ignore_connections: bool,
    pub result_allocator: AllocatorOpts,
}
impl<'a> AnimOpts<'a> {
    pub(crate) fn to_port(&mut self) -> f::AnimOpts {
        let mut o = f::AnimOpts::default();
        o.layer_ids = self.layer_ids.as_slice().to_vec();
        o.override_layer_weights = self.override_layer_weights.as_slice().to_vec();
        o.prop_overrides = self.prop_overrides.as_slice().iter().map(prop_override_to_port).collect();
        o.transform_overrides = self.transform_overrides.as_slice().to_vec();
        o.ignore_connections = self.ignore_connections;
        o
    }
}

#[derive(Default)]
pub struct BakeOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
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
impl BakeOpts {
    pub(crate) fn to_port(&mut self) -> f::BakeOpts {
        let mut o = f::BakeOpts::default();
        o.trim_start_time = self.trim_start_time;
        o.resample_rate = self.resample_rate;
        o.minimum_sample_rate = self.minimum_sample_rate;
        o.maximum_sample_rate = self.maximum_sample_rate;
        o.bake_transform_props = self.bake_transform_props;
        o.skip_node_transforms = self.skip_node_transforms;
        o.no_resample_rotation = self.no_resample_rotation;
        o.ignore_layer_weight_animation = self.ignore_layer_weight_animation;
        o.max_keyframe_segments = self.max_keyframe_segments;
        o.step_handling = self.step_handling;
        o.step_custom_duration = self.step_custom_duration;
        o.step_custom_epsilon = self.step_custom_epsilon;
        o.evaluate_flags = self.evaluate_flags;
        o.key_reduction_enabled = self.key_reduction_enabled;
        o.key_reduction_rotation = self.key_reduction_rotation;
        o.key_reduction_threshold = self.key_reduction_threshold;
        o.key_reduction_passes = self.key_reduction_passes;
        o
    }
}

#[derive(Default)]
pub struct TessellateCurveOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub span_subdivision: usize,
}
impl TessellateCurveOpts {
    pub(crate) fn to_port(&mut self) -> f::TessellateCurveOpts {
        let mut o = f::TessellateCurveOpts::default();
        o.span_subdivision = self.span_subdivision;
        o
    }
}

#[derive(Default)]
pub struct TessellateSurfaceOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub span_subdivision_u: usize,
    pub span_subdivision_v: usize,
    pub skip_mesh_parts: bool,
}
impl TessellateSurfaceOpts {
    pub(crate) fn to_port(&mut self) -> f::TessellateSurfaceOpts {
        let mut o = f::TessellateSurfaceOpts::default();
        o.span_subdivision_u = self.span_subdivision_u;
        o.span_subdivision_v = self.span_subdivision_v;
        o.skip_mesh_parts = self.skip_mesh_parts;
        o
    }
}

#[derive(Default)]
pub struct SubdivideOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
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
impl SubdivideOpts {
    pub(crate) fn to_port(&mut self) -> f::SubdivideOpts {
        let mut o = f::SubdivideOpts::default();
        o.boundary = self.boundary;
        o.uv_boundary = self.uv_boundary;
        o.ignore_normals = self.ignore_normals;
        o.interpolate_normals = self.interpolate_normals;
        o.interpolate_tangents = self.interpolate_tangents;
        o.evaluate_source_vertices = self.evaluate_source_vertices;
        o.max_source_vertices = self.max_source_vertices;
        o.evaluate_skin_weights = self.evaluate_skin_weights;
        o.max_skin_weights = self.max_skin_weights;
        o.skin_deformer_index = self.skin_deformer_index;
        o
    }
}

#[derive(Default)]
pub struct GeometryCacheOpts<'a> {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub open_file_cb: OpenFileCb<'a>,
    pub frames_per_second: f64,
    pub mirror_axis: MirrorAxis,
    pub use_scale_factor: bool,
    pub scale_factor: Real,
}
impl<'a> GeometryCacheOpts<'a> {
    pub(crate) fn to_port(&mut self) -> f::GeometryCacheOpts {
        let mut o = f::GeometryCacheOpts::default();
        o.open_file_cb = self.open_file_cb.to_port();
        o.frames_per_second = self.frames_per_second;
        o.mirror_axis = self.mirror_axis;
        o.use_scale_factor = self.use_scale_factor;
        o.scale_factor = self.scale_factor;
        o
    }
}

#[derive(Default)]
pub struct GeometryCacheDataOpts<'a> {
    pub open_file_cb: OpenFileCb<'a>,
    pub additive: bool,
    pub use_weight: bool,
    pub weight: Real,
    pub ignore_transform: bool,
}
impl<'a> GeometryCacheDataOpts<'a> {
    pub(crate) fn to_port(&mut self) -> f::GeometryCacheDataOpts {
        let mut o = f::GeometryCacheDataOpts::default();
        o.open_file_cb = self.open_file_cb.to_port();
        o.additive = self.additive;
        o.use_weight = self.use_weight;
        o.weight = self.weight;
        o.ignore_transform = self.ignore_transform;
        o
    }
}

pub(crate) struct Arenas {
    pub(crate) unknowns: Vec<Unknown>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) meshes: Vec<Mesh>,
    pub(crate) lights: Vec<Light>,
    pub(crate) cameras: Vec<Camera>,
    pub(crate) bones: Vec<Bone>,
    pub(crate) empties: Vec<Empty>,
    pub(crate) line_curves: Vec<LineCurve>,
    pub(crate) nurbs_curves: Vec<NurbsCurve>,
    pub(crate) nurbs_surfaces: Vec<NurbsSurface>,
    pub(crate) nurbs_trim_surfaces: Vec<NurbsTrimSurface>,
    pub(crate) nurbs_trim_boundaries: Vec<NurbsTrimBoundary>,
    pub(crate) procedural_geometries: Vec<ProceduralGeometry>,
    pub(crate) stereo_cameras: Vec<StereoCamera>,
    pub(crate) camera_switchers: Vec<CameraSwitcher>,
    pub(crate) markers: Vec<Marker>,
    pub(crate) lod_groups: Vec<LodGroup>,
    pub(crate) skin_deformers: Vec<SkinDeformer>,
    pub(crate) skin_clusters: Vec<SkinCluster>,
    pub(crate) blend_deformers: Vec<BlendDeformer>,
    pub(crate) blend_channels: Vec<BlendChannel>,
    pub(crate) blend_shapes: Vec<BlendShape>,
    pub(crate) cache_deformers: Vec<CacheDeformer>,
    pub(crate) cache_files: Vec<CacheFile>,
    pub(crate) materials: Vec<Material>,
    pub(crate) textures: Vec<Texture>,
    pub(crate) videos: Vec<Video>,
    pub(crate) shaders: Vec<Shader>,
    pub(crate) shader_bindings: Vec<ShaderBinding>,
    pub(crate) anim_stacks: Vec<AnimStack>,
    pub(crate) anim_layers: Vec<AnimLayer>,
    pub(crate) anim_values: Vec<AnimValue>,
    pub(crate) anim_curves: Vec<AnimCurve>,
    pub(crate) display_layers: Vec<DisplayLayer>,
    pub(crate) selection_sets: Vec<SelectionSet>,
    pub(crate) selection_nodes: Vec<SelectionNode>,
    pub(crate) characters: Vec<Character>,
    pub(crate) constraints: Vec<Constraint>,
    pub(crate) audio_layers: Vec<AudioLayer>,
    pub(crate) audio_clips: Vec<AudioClip>,
    pub(crate) poses: Vec<Pose>,
    pub(crate) metadata_objects: Vec<MetadataObject>,
}

#[derive(Clone, Copy)]
pub(crate) struct Bases {
    pub(crate) unknowns: *const Unknown,
    pub(crate) nodes: *const Node,
    pub(crate) meshes: *const Mesh,
    pub(crate) lights: *const Light,
    pub(crate) cameras: *const Camera,
    pub(crate) bones: *const Bone,
    pub(crate) empties: *const Empty,
    pub(crate) line_curves: *const LineCurve,
    pub(crate) nurbs_curves: *const NurbsCurve,
    pub(crate) nurbs_surfaces: *const NurbsSurface,
    pub(crate) nurbs_trim_surfaces: *const NurbsTrimSurface,
    pub(crate) nurbs_trim_boundaries: *const NurbsTrimBoundary,
    pub(crate) procedural_geometries: *const ProceduralGeometry,
    pub(crate) stereo_cameras: *const StereoCamera,
    pub(crate) camera_switchers: *const CameraSwitcher,
    pub(crate) markers: *const Marker,
    pub(crate) lod_groups: *const LodGroup,
    pub(crate) skin_deformers: *const SkinDeformer,
    pub(crate) skin_clusters: *const SkinCluster,
    pub(crate) blend_deformers: *const BlendDeformer,
    pub(crate) blend_channels: *const BlendChannel,
    pub(crate) blend_shapes: *const BlendShape,
    pub(crate) cache_deformers: *const CacheDeformer,
    pub(crate) cache_files: *const CacheFile,
    pub(crate) materials: *const Material,
    pub(crate) textures: *const Texture,
    pub(crate) videos: *const Video,
    pub(crate) shaders: *const Shader,
    pub(crate) shader_bindings: *const ShaderBinding,
    pub(crate) anim_stacks: *const AnimStack,
    pub(crate) anim_layers: *const AnimLayer,
    pub(crate) anim_values: *const AnimValue,
    pub(crate) anim_curves: *const AnimCurve,
    pub(crate) display_layers: *const DisplayLayer,
    pub(crate) selection_sets: *const SelectionSet,
    pub(crate) selection_nodes: *const SelectionNode,
    pub(crate) characters: *const Character,
    pub(crate) constraints: *const Constraint,
    pub(crate) audio_layers: *const AudioLayer,
    pub(crate) audio_clips: *const AudioClip,
    pub(crate) poses: *const Pose,
    pub(crate) metadata_objects: *const MetadataObject,
}

impl Bases {
    pub(crate) fn none() -> Bases {
        Bases {
            unknowns: std::ptr::null(),
            nodes: std::ptr::null(),
            meshes: std::ptr::null(),
            lights: std::ptr::null(),
            cameras: std::ptr::null(),
            bones: std::ptr::null(),
            empties: std::ptr::null(),
            line_curves: std::ptr::null(),
            nurbs_curves: std::ptr::null(),
            nurbs_surfaces: std::ptr::null(),
            nurbs_trim_surfaces: std::ptr::null(),
            nurbs_trim_boundaries: std::ptr::null(),
            procedural_geometries: std::ptr::null(),
            stereo_cameras: std::ptr::null(),
            camera_switchers: std::ptr::null(),
            markers: std::ptr::null(),
            lod_groups: std::ptr::null(),
            skin_deformers: std::ptr::null(),
            skin_clusters: std::ptr::null(),
            blend_deformers: std::ptr::null(),
            blend_channels: std::ptr::null(),
            blend_shapes: std::ptr::null(),
            cache_deformers: std::ptr::null(),
            cache_files: std::ptr::null(),
            materials: std::ptr::null(),
            textures: std::ptr::null(),
            videos: std::ptr::null(),
            shaders: std::ptr::null(),
            shader_bindings: std::ptr::null(),
            anim_stacks: std::ptr::null(),
            anim_layers: std::ptr::null(),
            anim_values: std::ptr::null(),
            anim_curves: std::ptr::null(),
            display_layers: std::ptr::null(),
            selection_sets: std::ptr::null(),
            selection_nodes: std::ptr::null(),
            characters: std::ptr::null(),
            constraints: std::ptr::null(),
            audio_layers: std::ptr::null(),
            audio_clips: std::ptr::null(),
            poses: std::ptr::null(),
            metadata_objects: std::ptr::null(),
        }
    }
    pub(crate) fn element_at(&self, r: f::ElementRef) -> *const Element {
        match r.element_type {
            ElementType::Unknown => field(self.unknowns, r.index, std::mem::offset_of!(Unknown, element)),
            ElementType::Node => field(self.nodes, r.index, std::mem::offset_of!(Node, element)),
            ElementType::Mesh => field(self.meshes, r.index, std::mem::offset_of!(Mesh, element)),
            ElementType::Light => field(self.lights, r.index, std::mem::offset_of!(Light, element)),
            ElementType::Camera => field(self.cameras, r.index, std::mem::offset_of!(Camera, element)),
            ElementType::Bone => field(self.bones, r.index, std::mem::offset_of!(Bone, element)),
            ElementType::Empty => field(self.empties, r.index, std::mem::offset_of!(Empty, element)),
            ElementType::LineCurve => field(self.line_curves, r.index, std::mem::offset_of!(LineCurve, element)),
            ElementType::NurbsCurve => field(self.nurbs_curves, r.index, std::mem::offset_of!(NurbsCurve, element)),
            ElementType::NurbsSurface => field(self.nurbs_surfaces, r.index, std::mem::offset_of!(NurbsSurface, element)),
            ElementType::NurbsTrimSurface => field(self.nurbs_trim_surfaces, r.index, std::mem::offset_of!(NurbsTrimSurface, element)),
            ElementType::NurbsTrimBoundary => field(self.nurbs_trim_boundaries, r.index, std::mem::offset_of!(NurbsTrimBoundary, element)),
            ElementType::ProceduralGeometry => field(self.procedural_geometries, r.index, std::mem::offset_of!(ProceduralGeometry, element)),
            ElementType::StereoCamera => field(self.stereo_cameras, r.index, std::mem::offset_of!(StereoCamera, element)),
            ElementType::CameraSwitcher => field(self.camera_switchers, r.index, std::mem::offset_of!(CameraSwitcher, element)),
            ElementType::Marker => field(self.markers, r.index, std::mem::offset_of!(Marker, element)),
            ElementType::LodGroup => field(self.lod_groups, r.index, std::mem::offset_of!(LodGroup, element)),
            ElementType::SkinDeformer => field(self.skin_deformers, r.index, std::mem::offset_of!(SkinDeformer, element)),
            ElementType::SkinCluster => field(self.skin_clusters, r.index, std::mem::offset_of!(SkinCluster, element)),
            ElementType::BlendDeformer => field(self.blend_deformers, r.index, std::mem::offset_of!(BlendDeformer, element)),
            ElementType::BlendChannel => field(self.blend_channels, r.index, std::mem::offset_of!(BlendChannel, element)),
            ElementType::BlendShape => field(self.blend_shapes, r.index, std::mem::offset_of!(BlendShape, element)),
            ElementType::CacheDeformer => field(self.cache_deformers, r.index, std::mem::offset_of!(CacheDeformer, element)),
            ElementType::CacheFile => field(self.cache_files, r.index, std::mem::offset_of!(CacheFile, element)),
            ElementType::Material => field(self.materials, r.index, std::mem::offset_of!(Material, element)),
            ElementType::Texture => field(self.textures, r.index, std::mem::offset_of!(Texture, element)),
            ElementType::Video => field(self.videos, r.index, std::mem::offset_of!(Video, element)),
            ElementType::Shader => field(self.shaders, r.index, std::mem::offset_of!(Shader, element)),
            ElementType::ShaderBinding => field(self.shader_bindings, r.index, std::mem::offset_of!(ShaderBinding, element)),
            ElementType::AnimStack => field(self.anim_stacks, r.index, std::mem::offset_of!(AnimStack, element)),
            ElementType::AnimLayer => field(self.anim_layers, r.index, std::mem::offset_of!(AnimLayer, element)),
            ElementType::AnimValue => field(self.anim_values, r.index, std::mem::offset_of!(AnimValue, element)),
            ElementType::AnimCurve => field(self.anim_curves, r.index, std::mem::offset_of!(AnimCurve, element)),
            ElementType::DisplayLayer => field(self.display_layers, r.index, std::mem::offset_of!(DisplayLayer, element)),
            ElementType::SelectionSet => field(self.selection_sets, r.index, std::mem::offset_of!(SelectionSet, element)),
            ElementType::SelectionNode => field(self.selection_nodes, r.index, std::mem::offset_of!(SelectionNode, element)),
            ElementType::Character => field(self.characters, r.index, std::mem::offset_of!(Character, element)),
            ElementType::Constraint => field(self.constraints, r.index, std::mem::offset_of!(Constraint, element)),
            ElementType::AudioLayer => field(self.audio_layers, r.index, std::mem::offset_of!(AudioLayer, element)),
            ElementType::AudioClip => field(self.audio_clips, r.index, std::mem::offset_of!(AudioClip, element)),
            ElementType::Pose => field(self.poses, r.index, std::mem::offset_of!(Pose, element)),
            ElementType::MetadataObject => field(self.metadata_objects, r.index, std::mem::offset_of!(MetadataObject, element)),
            #[allow(unreachable_patterns)]
            _ => std::ptr::null(),
        }
    }
}

pub(crate) fn make_arenas(p: &f::Scene) -> (Arenas, Bases) {
    let a = Arenas {
        unknowns: Vec::with_capacity(p.unknowns.len()),
        nodes: Vec::with_capacity(p.nodes.len()),
        meshes: Vec::with_capacity(p.meshes.len()),
        lights: Vec::with_capacity(p.lights.len()),
        cameras: Vec::with_capacity(p.cameras.len()),
        bones: Vec::with_capacity(p.bones.len()),
        empties: Vec::with_capacity(p.empties.len()),
        line_curves: Vec::with_capacity(p.line_curves.len()),
        nurbs_curves: Vec::with_capacity(p.nurbs_curves.len()),
        nurbs_surfaces: Vec::with_capacity(p.nurbs_surfaces.len()),
        nurbs_trim_surfaces: Vec::with_capacity(p.nurbs_trim_surfaces.len()),
        nurbs_trim_boundaries: Vec::with_capacity(p.nurbs_trim_boundaries.len()),
        procedural_geometries: Vec::with_capacity(p.procedural_geometries.len()),
        stereo_cameras: Vec::with_capacity(p.stereo_cameras.len()),
        camera_switchers: Vec::with_capacity(p.camera_switchers.len()),
        markers: Vec::with_capacity(p.markers.len()),
        lod_groups: Vec::with_capacity(p.lod_groups.len()),
        skin_deformers: Vec::with_capacity(p.skin_deformers.len()),
        skin_clusters: Vec::with_capacity(p.skin_clusters.len()),
        blend_deformers: Vec::with_capacity(p.blend_deformers.len()),
        blend_channels: Vec::with_capacity(p.blend_channels.len()),
        blend_shapes: Vec::with_capacity(p.blend_shapes.len()),
        cache_deformers: Vec::with_capacity(p.cache_deformers.len()),
        cache_files: Vec::with_capacity(p.cache_files.len()),
        materials: Vec::with_capacity(p.materials.len()),
        textures: Vec::with_capacity(p.textures.len()),
        videos: Vec::with_capacity(p.videos.len()),
        shaders: Vec::with_capacity(p.shaders.len()),
        shader_bindings: Vec::with_capacity(p.shader_bindings.len()),
        anim_stacks: Vec::with_capacity(p.anim_stacks.len()),
        anim_layers: Vec::with_capacity(p.anim_layers.len()),
        anim_values: Vec::with_capacity(p.anim_values.len()),
        anim_curves: Vec::with_capacity(p.anim_curves.len()),
        display_layers: Vec::with_capacity(p.display_layers.len()),
        selection_sets: Vec::with_capacity(p.selection_sets.len()),
        selection_nodes: Vec::with_capacity(p.selection_nodes.len()),
        characters: Vec::with_capacity(p.characters.len()),
        constraints: Vec::with_capacity(p.constraints.len()),
        audio_layers: Vec::with_capacity(p.audio_layers.len()),
        audio_clips: Vec::with_capacity(p.audio_clips.len()),
        poses: Vec::with_capacity(p.poses.len()),
        metadata_objects: Vec::with_capacity(p.metadata_objects.len()),
    };
    let b = Bases {
        unknowns: a.unknowns.as_ptr(),
        nodes: a.nodes.as_ptr(),
        meshes: a.meshes.as_ptr(),
        lights: a.lights.as_ptr(),
        cameras: a.cameras.as_ptr(),
        bones: a.bones.as_ptr(),
        empties: a.empties.as_ptr(),
        line_curves: a.line_curves.as_ptr(),
        nurbs_curves: a.nurbs_curves.as_ptr(),
        nurbs_surfaces: a.nurbs_surfaces.as_ptr(),
        nurbs_trim_surfaces: a.nurbs_trim_surfaces.as_ptr(),
        nurbs_trim_boundaries: a.nurbs_trim_boundaries.as_ptr(),
        procedural_geometries: a.procedural_geometries.as_ptr(),
        stereo_cameras: a.stereo_cameras.as_ptr(),
        camera_switchers: a.camera_switchers.as_ptr(),
        markers: a.markers.as_ptr(),
        lod_groups: a.lod_groups.as_ptr(),
        skin_deformers: a.skin_deformers.as_ptr(),
        skin_clusters: a.skin_clusters.as_ptr(),
        blend_deformers: a.blend_deformers.as_ptr(),
        blend_channels: a.blend_channels.as_ptr(),
        blend_shapes: a.blend_shapes.as_ptr(),
        cache_deformers: a.cache_deformers.as_ptr(),
        cache_files: a.cache_files.as_ptr(),
        materials: a.materials.as_ptr(),
        textures: a.textures.as_ptr(),
        videos: a.videos.as_ptr(),
        shaders: a.shaders.as_ptr(),
        shader_bindings: a.shader_bindings.as_ptr(),
        anim_stacks: a.anim_stacks.as_ptr(),
        anim_layers: a.anim_layers.as_ptr(),
        anim_values: a.anim_values.as_ptr(),
        anim_curves: a.anim_curves.as_ptr(),
        display_layers: a.display_layers.as_ptr(),
        selection_sets: a.selection_sets.as_ptr(),
        selection_nodes: a.selection_nodes.as_ptr(),
        characters: a.characters.as_ptr(),
        constraints: a.constraints.as_ptr(),
        audio_layers: a.audio_layers.as_ptr(),
        audio_clips: a.audio_clips.as_ptr(),
        poses: a.poses.as_ptr(),
        metadata_objects: a.metadata_objects.as_ptr(),
    };
    (a, b)
}

pub(crate) fn fill_arenas(k: &Kx, p: &f::Scene, a: &mut Arenas) {
    a.unknowns.extend(p.unknowns.iter().map(|x| from_Unknown(k, x)));
    a.nodes.extend(p.nodes.iter().map(|x| from_Node(k, x)));
    a.meshes.extend(p.meshes.iter().map(|x| from_Mesh(k, x)));
    a.lights.extend(p.lights.iter().map(|x| from_Light(k, x)));
    a.cameras.extend(p.cameras.iter().map(|x| from_Camera(k, x)));
    a.bones.extend(p.bones.iter().map(|x| from_Bone(k, x)));
    a.empties.extend(p.empties.iter().map(|x| from_Empty(k, x)));
    a.line_curves.extend(p.line_curves.iter().map(|x| from_LineCurve(k, x)));
    a.nurbs_curves.extend(p.nurbs_curves.iter().map(|x| from_NurbsCurve(k, x)));
    a.nurbs_surfaces.extend(p.nurbs_surfaces.iter().map(|x| from_NurbsSurface(k, x)));
    a.nurbs_trim_surfaces.extend(p.nurbs_trim_surfaces.iter().map(|x| from_NurbsTrimSurface(k, x)));
    a.nurbs_trim_boundaries.extend(p.nurbs_trim_boundaries.iter().map(|x| from_NurbsTrimBoundary(k, x)));
    a.procedural_geometries.extend(p.procedural_geometries.iter().map(|x| from_ProceduralGeometry(k, x)));
    a.stereo_cameras.extend(p.stereo_cameras.iter().map(|x| from_StereoCamera(k, x)));
    a.camera_switchers.extend(p.camera_switchers.iter().map(|x| from_CameraSwitcher(k, x)));
    a.markers.extend(p.markers.iter().map(|x| from_Marker(k, x)));
    a.lod_groups.extend(p.lod_groups.iter().map(|x| from_LodGroup(k, x)));
    a.skin_deformers.extend(p.skin_deformers.iter().map(|x| from_SkinDeformer(k, x)));
    a.skin_clusters.extend(p.skin_clusters.iter().map(|x| from_SkinCluster(k, x)));
    a.blend_deformers.extend(p.blend_deformers.iter().map(|x| from_BlendDeformer(k, x)));
    a.blend_channels.extend(p.blend_channels.iter().map(|x| from_BlendChannel(k, x)));
    a.blend_shapes.extend(p.blend_shapes.iter().map(|x| from_BlendShape(k, x)));
    a.cache_deformers.extend(p.cache_deformers.iter().map(|x| from_CacheDeformer(k, x)));
    a.cache_files.extend(p.cache_files.iter().map(|x| from_CacheFile(k, x)));
    a.materials.extend(p.materials.iter().map(|x| from_Material(k, x)));
    a.textures.extend(p.textures.iter().map(|x| from_Texture(k, x)));
    a.videos.extend(p.videos.iter().map(|x| from_Video(k, x)));
    a.shaders.extend(p.shaders.iter().map(|x| from_Shader(k, x)));
    a.shader_bindings.extend(p.shader_bindings.iter().map(|x| from_ShaderBinding(k, x)));
    a.anim_stacks.extend(p.anim_stacks.iter().map(|x| from_AnimStack(k, x)));
    a.anim_layers.extend(p.anim_layers.iter().map(|x| from_AnimLayer(k, x)));
    a.anim_values.extend(p.anim_values.iter().map(|x| from_AnimValue(k, x)));
    a.anim_curves.extend(p.anim_curves.iter().map(|x| from_AnimCurve(k, x)));
    a.display_layers.extend(p.display_layers.iter().map(|x| from_DisplayLayer(k, x)));
    a.selection_sets.extend(p.selection_sets.iter().map(|x| from_SelectionSet(k, x)));
    a.selection_nodes.extend(p.selection_nodes.iter().map(|x| from_SelectionNode(k, x)));
    a.characters.extend(p.characters.iter().map(|x| from_Character(k, x)));
    a.constraints.extend(p.constraints.iter().map(|x| from_Constraint(k, x)));
    a.audio_layers.extend(p.audio_layers.iter().map(|x| from_AudioLayer(k, x)));
    a.audio_clips.extend(p.audio_clips.iter().map(|x| from_AudioClip(k, x)));
    a.poses.extend(p.poses.iter().map(|x| from_Pose(k, x)));
    a.metadata_objects.extend(p.metadata_objects.iter().map(|x| from_MetadataObject(k, x)));
}

pub fn as_unknown<'a>(element: &'a Element) -> Option<&'a Unknown> {
    if element.type_ == ElementType::Unknown { element.scene.unknowns.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_node<'a>(element: &'a Element) -> Option<&'a Node> {
    if element.type_ == ElementType::Node { element.scene.nodes.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_mesh<'a>(element: &'a Element) -> Option<&'a Mesh> {
    if element.type_ == ElementType::Mesh { element.scene.meshes.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_light<'a>(element: &'a Element) -> Option<&'a Light> {
    if element.type_ == ElementType::Light { element.scene.lights.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_camera<'a>(element: &'a Element) -> Option<&'a Camera> {
    if element.type_ == ElementType::Camera { element.scene.cameras.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_bone<'a>(element: &'a Element) -> Option<&'a Bone> {
    if element.type_ == ElementType::Bone { element.scene.bones.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_empty<'a>(element: &'a Element) -> Option<&'a Empty> {
    if element.type_ == ElementType::Empty { element.scene.empties.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_line_curve<'a>(element: &'a Element) -> Option<&'a LineCurve> {
    if element.type_ == ElementType::LineCurve { element.scene.line_curves.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_nurbs_curve<'a>(element: &'a Element) -> Option<&'a NurbsCurve> {
    if element.type_ == ElementType::NurbsCurve { element.scene.nurbs_curves.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_nurbs_surface<'a>(element: &'a Element) -> Option<&'a NurbsSurface> {
    if element.type_ == ElementType::NurbsSurface { element.scene.nurbs_surfaces.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_nurbs_trim_surface<'a>(element: &'a Element) -> Option<&'a NurbsTrimSurface> {
    if element.type_ == ElementType::NurbsTrimSurface { element.scene.nurbs_trim_surfaces.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_nurbs_trim_boundary<'a>(element: &'a Element) -> Option<&'a NurbsTrimBoundary> {
    if element.type_ == ElementType::NurbsTrimBoundary { element.scene.nurbs_trim_boundaries.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_procedural_geometry<'a>(element: &'a Element) -> Option<&'a ProceduralGeometry> {
    if element.type_ == ElementType::ProceduralGeometry { element.scene.procedural_geometries.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_stereo_camera<'a>(element: &'a Element) -> Option<&'a StereoCamera> {
    if element.type_ == ElementType::StereoCamera { element.scene.stereo_cameras.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_camera_switcher<'a>(element: &'a Element) -> Option<&'a CameraSwitcher> {
    if element.type_ == ElementType::CameraSwitcher { element.scene.camera_switchers.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_marker<'a>(element: &'a Element) -> Option<&'a Marker> {
    if element.type_ == ElementType::Marker { element.scene.markers.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_lod_group<'a>(element: &'a Element) -> Option<&'a LodGroup> {
    if element.type_ == ElementType::LodGroup { element.scene.lod_groups.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_skin_deformer<'a>(element: &'a Element) -> Option<&'a SkinDeformer> {
    if element.type_ == ElementType::SkinDeformer { element.scene.skin_deformers.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_skin_cluster<'a>(element: &'a Element) -> Option<&'a SkinCluster> {
    if element.type_ == ElementType::SkinCluster { element.scene.skin_clusters.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_blend_deformer<'a>(element: &'a Element) -> Option<&'a BlendDeformer> {
    if element.type_ == ElementType::BlendDeformer { element.scene.blend_deformers.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_blend_channel<'a>(element: &'a Element) -> Option<&'a BlendChannel> {
    if element.type_ == ElementType::BlendChannel { element.scene.blend_channels.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_blend_shape<'a>(element: &'a Element) -> Option<&'a BlendShape> {
    if element.type_ == ElementType::BlendShape { element.scene.blend_shapes.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_cache_deformer<'a>(element: &'a Element) -> Option<&'a CacheDeformer> {
    if element.type_ == ElementType::CacheDeformer { element.scene.cache_deformers.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_cache_file<'a>(element: &'a Element) -> Option<&'a CacheFile> {
    if element.type_ == ElementType::CacheFile { element.scene.cache_files.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_material<'a>(element: &'a Element) -> Option<&'a Material> {
    if element.type_ == ElementType::Material { element.scene.materials.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_texture<'a>(element: &'a Element) -> Option<&'a Texture> {
    if element.type_ == ElementType::Texture { element.scene.textures.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_video<'a>(element: &'a Element) -> Option<&'a Video> {
    if element.type_ == ElementType::Video { element.scene.videos.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_shader<'a>(element: &'a Element) -> Option<&'a Shader> {
    if element.type_ == ElementType::Shader { element.scene.shaders.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_shader_binding<'a>(element: &'a Element) -> Option<&'a ShaderBinding> {
    if element.type_ == ElementType::ShaderBinding { element.scene.shader_bindings.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_anim_stack<'a>(element: &'a Element) -> Option<&'a AnimStack> {
    if element.type_ == ElementType::AnimStack { element.scene.anim_stacks.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_anim_layer<'a>(element: &'a Element) -> Option<&'a AnimLayer> {
    if element.type_ == ElementType::AnimLayer { element.scene.anim_layers.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_anim_value<'a>(element: &'a Element) -> Option<&'a AnimValue> {
    if element.type_ == ElementType::AnimValue { element.scene.anim_values.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_anim_curve<'a>(element: &'a Element) -> Option<&'a AnimCurve> {
    if element.type_ == ElementType::AnimCurve { element.scene.anim_curves.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_display_layer<'a>(element: &'a Element) -> Option<&'a DisplayLayer> {
    if element.type_ == ElementType::DisplayLayer { element.scene.display_layers.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_selection_set<'a>(element: &'a Element) -> Option<&'a SelectionSet> {
    if element.type_ == ElementType::SelectionSet { element.scene.selection_sets.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_selection_node<'a>(element: &'a Element) -> Option<&'a SelectionNode> {
    if element.type_ == ElementType::SelectionNode { element.scene.selection_nodes.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_character<'a>(element: &'a Element) -> Option<&'a Character> {
    if element.type_ == ElementType::Character { element.scene.characters.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_constraint<'a>(element: &'a Element) -> Option<&'a Constraint> {
    if element.type_ == ElementType::Constraint { element.scene.constraints.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_audio_layer<'a>(element: &'a Element) -> Option<&'a AudioLayer> {
    if element.type_ == ElementType::AudioLayer { element.scene.audio_layers.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_audio_clip<'a>(element: &'a Element) -> Option<&'a AudioClip> {
    if element.type_ == ElementType::AudioClip { element.scene.audio_clips.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_pose<'a>(element: &'a Element) -> Option<&'a Pose> {
    if element.type_ == ElementType::Pose { element.scene.poses.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub fn as_metadata_object<'a>(element: &'a Element) -> Option<&'a MetadataObject> {
    if element.type_ == ElementType::MetadataObject { element.scene.metadata_objects.get(element.typed_id as usize).map(|r| &**r) } else { None }
}

pub enum ElementData<'a> {
    Unknown(&'a Unknown),
    Node(&'a Node),
    Mesh(&'a Mesh),
    Light(&'a Light),
    Camera(&'a Camera),
    Bone(&'a Bone),
    Empty(&'a Empty),
    LineCurve(&'a LineCurve),
    NurbsCurve(&'a NurbsCurve),
    NurbsSurface(&'a NurbsSurface),
    NurbsTrimSurface(&'a NurbsTrimSurface),
    NurbsTrimBoundary(&'a NurbsTrimBoundary),
    ProceduralGeometry(&'a ProceduralGeometry),
    StereoCamera(&'a StereoCamera),
    CameraSwitcher(&'a CameraSwitcher),
    Marker(&'a Marker),
    LodGroup(&'a LodGroup),
    SkinDeformer(&'a SkinDeformer),
    SkinCluster(&'a SkinCluster),
    BlendDeformer(&'a BlendDeformer),
    BlendChannel(&'a BlendChannel),
    BlendShape(&'a BlendShape),
    CacheDeformer(&'a CacheDeformer),
    CacheFile(&'a CacheFile),
    Material(&'a Material),
    Texture(&'a Texture),
    Video(&'a Video),
    Shader(&'a Shader),
    ShaderBinding(&'a ShaderBinding),
    AnimStack(&'a AnimStack),
    AnimLayer(&'a AnimLayer),
    AnimValue(&'a AnimValue),
    AnimCurve(&'a AnimCurve),
    DisplayLayer(&'a DisplayLayer),
    SelectionSet(&'a SelectionSet),
    SelectionNode(&'a SelectionNode),
    Character(&'a Character),
    Constraint(&'a Constraint),
    AudioLayer(&'a AudioLayer),
    AudioClip(&'a AudioClip),
    Pose(&'a Pose),
    MetadataObject(&'a MetadataObject),
}

impl Element {
    pub fn as_data(&self) -> ElementData<'_> {
        let s = &self.scene;
        let i = self.typed_id as usize;
        match self.type_ {
            ElementType::Unknown => ElementData::Unknown(&s.unknowns[i]),
            ElementType::Node => ElementData::Node(&s.nodes[i]),
            ElementType::Mesh => ElementData::Mesh(&s.meshes[i]),
            ElementType::Light => ElementData::Light(&s.lights[i]),
            ElementType::Camera => ElementData::Camera(&s.cameras[i]),
            ElementType::Bone => ElementData::Bone(&s.bones[i]),
            ElementType::Empty => ElementData::Empty(&s.empties[i]),
            ElementType::LineCurve => ElementData::LineCurve(&s.line_curves[i]),
            ElementType::NurbsCurve => ElementData::NurbsCurve(&s.nurbs_curves[i]),
            ElementType::NurbsSurface => ElementData::NurbsSurface(&s.nurbs_surfaces[i]),
            ElementType::NurbsTrimSurface => ElementData::NurbsTrimSurface(&s.nurbs_trim_surfaces[i]),
            ElementType::NurbsTrimBoundary => ElementData::NurbsTrimBoundary(&s.nurbs_trim_boundaries[i]),
            ElementType::ProceduralGeometry => ElementData::ProceduralGeometry(&s.procedural_geometries[i]),
            ElementType::StereoCamera => ElementData::StereoCamera(&s.stereo_cameras[i]),
            ElementType::CameraSwitcher => ElementData::CameraSwitcher(&s.camera_switchers[i]),
            ElementType::Marker => ElementData::Marker(&s.markers[i]),
            ElementType::LodGroup => ElementData::LodGroup(&s.lod_groups[i]),
            ElementType::SkinDeformer => ElementData::SkinDeformer(&s.skin_deformers[i]),
            ElementType::SkinCluster => ElementData::SkinCluster(&s.skin_clusters[i]),
            ElementType::BlendDeformer => ElementData::BlendDeformer(&s.blend_deformers[i]),
            ElementType::BlendChannel => ElementData::BlendChannel(&s.blend_channels[i]),
            ElementType::BlendShape => ElementData::BlendShape(&s.blend_shapes[i]),
            ElementType::CacheDeformer => ElementData::CacheDeformer(&s.cache_deformers[i]),
            ElementType::CacheFile => ElementData::CacheFile(&s.cache_files[i]),
            ElementType::Material => ElementData::Material(&s.materials[i]),
            ElementType::Texture => ElementData::Texture(&s.textures[i]),
            ElementType::Video => ElementData::Video(&s.videos[i]),
            ElementType::Shader => ElementData::Shader(&s.shaders[i]),
            ElementType::ShaderBinding => ElementData::ShaderBinding(&s.shader_bindings[i]),
            ElementType::AnimStack => ElementData::AnimStack(&s.anim_stacks[i]),
            ElementType::AnimLayer => ElementData::AnimLayer(&s.anim_layers[i]),
            ElementType::AnimValue => ElementData::AnimValue(&s.anim_values[i]),
            ElementType::AnimCurve => ElementData::AnimCurve(&s.anim_curves[i]),
            ElementType::DisplayLayer => ElementData::DisplayLayer(&s.display_layers[i]),
            ElementType::SelectionSet => ElementData::SelectionSet(&s.selection_sets[i]),
            ElementType::SelectionNode => ElementData::SelectionNode(&s.selection_nodes[i]),
            ElementType::Character => ElementData::Character(&s.characters[i]),
            ElementType::Constraint => ElementData::Constraint(&s.constraints[i]),
            ElementType::AudioLayer => ElementData::AudioLayer(&s.audio_layers[i]),
            ElementType::AudioClip => ElementData::AudioClip(&s.audio_clips[i]),
            ElementType::Pose => ElementData::Pose(&s.poses[i]),
            ElementType::MetadataObject => ElementData::MetadataObject(&s.metadata_objects[i]),
            #[allow(unreachable_patterns)]
            _ => panic!("no such element"),
        }
    }
}

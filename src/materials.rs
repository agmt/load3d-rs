use crate::parse::f64_to_i64;
use crate::types::*;

struct Mapping {
    index: u8,
    flags: u8,
    transform: u8,
    prop: &'static str,
}

const fn m(index: u8, flags: u8, transform: u8, prop: &'static str) -> Mapping {
    Mapping { index, flags, transform, prop }
}

const DEFAULT_W_1: u8 = 0x1;
const WIDEN_TO_RGB: u8 = 0x2;
const MULTIPLY_VALUE: u8 = 0x4;

const F_INVERTED: u8 = 0x1;
const F_IF_EXISTS: u8 = 0x2;
const F_IF_TEXTURE: u8 = 0x4;
const F_IF_AROUND_1: u8 = 0x8;
const F_IF_EXISTS_OR_TEXTURE: u8 = F_IF_EXISTS | F_IF_TEXTURE;

const T_IDENTITY: u8 = 0;
const T_INVERT_X: u8 = 1;
const T_UNKNOWN_SHININESS: u8 = 2;
const T_BLENDER_OPACITY: u8 = 3;
const T_BLENDER_SHININESS: u8 = 4;

/// `ufbxi_mat_transform_invert_x`, `ufbxi_mat_transform_unknown_shininess`, `ufbxi_mat_transform_blender_opacity` and
/// `ufbxi_mat_transform_blender_shininess` in ufbx.c
fn mat_transform(transform: u8, v: &mut Vec4) {
    match transform {
        T_INVERT_X | T_BLENDER_OPACITY => v.x = 1.0 - v.x,
        T_UNKNOWN_SHININESS | T_BLENDER_SHININESS => {
            if v.x >= 0.0 {
                v.x = 1.0 - v.x.sqrt() * 0.1;
            }
            if !(v.x >= 0.0) {
                v.x = 0.0;
            }
        }
        _ => {}
    }
}

const BASE_FBX_MAPPING: &[Mapping] = &[
    m(MaterialFbxMap::DiffuseColor as u8, DEFAULT_W_1, T_IDENTITY, "Diffuse"),
    m(MaterialFbxMap::DiffuseColor as u8, DEFAULT_W_1, T_IDENTITY, "DiffuseColor"),
    m(MaterialFbxMap::DiffuseFactor as u8, 0, T_IDENTITY, "DiffuseFactor"),
    m(MaterialFbxMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "Specular"),
    m(MaterialFbxMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "SpecularColor"),
    m(MaterialFbxMap::SpecularFactor as u8, 0, T_IDENTITY, "SpecularFactor"),
    m(MaterialFbxMap::SpecularExponent as u8, 0, T_IDENTITY, "Shininess"),
    m(MaterialFbxMap::SpecularExponent as u8, 0, T_IDENTITY, "ShininessExponent"),
    m(MaterialFbxMap::ReflectionColor as u8, DEFAULT_W_1, T_IDENTITY, "Reflection"),
    m(MaterialFbxMap::ReflectionColor as u8, DEFAULT_W_1, T_IDENTITY, "ReflectionColor"),
    m(MaterialFbxMap::ReflectionFactor as u8, 0, T_IDENTITY, "ReflectionFactor"),
    m(MaterialFbxMap::TransparencyColor as u8, DEFAULT_W_1, T_IDENTITY, "Transparent"),
    m(MaterialFbxMap::TransparencyColor as u8, DEFAULT_W_1, T_IDENTITY, "TransparentColor"),
    m(MaterialFbxMap::TransparencyFactor as u8, 0, T_IDENTITY, "TransparentFactor"),
    m(MaterialFbxMap::TransparencyFactor as u8, 0, T_IDENTITY, "TransparencyFactor"),
    m(MaterialFbxMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "Emissive"),
    m(MaterialFbxMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "EmissiveColor"),
    m(MaterialFbxMap::EmissionFactor as u8, 0, T_IDENTITY, "EmissiveFactor"),
    m(MaterialFbxMap::AmbientColor as u8, DEFAULT_W_1, T_IDENTITY, "Ambient"),
    m(MaterialFbxMap::AmbientColor as u8, DEFAULT_W_1, T_IDENTITY, "AmbientColor"),
    m(MaterialFbxMap::AmbientFactor as u8, 0, T_IDENTITY, "AmbientFactor"),
    m(MaterialFbxMap::NormalMap as u8, 0, T_IDENTITY, "NormalMap"),
    m(MaterialFbxMap::Bump as u8, 0, T_IDENTITY, "Bump"),
    m(MaterialFbxMap::BumpFactor as u8, 0, T_IDENTITY, "BumpFactor"),
    m(MaterialFbxMap::Displacement as u8, 0, T_IDENTITY, "Displacement"),
    m(MaterialFbxMap::DisplacementFactor as u8, 0, T_IDENTITY, "DisplacementFactor"),
    m(MaterialFbxMap::VectorDisplacement as u8, 0, T_IDENTITY, "VectorDisplacement"),
    m(MaterialFbxMap::VectorDisplacementFactor as u8, 0, T_IDENTITY, "VectorDisplacementFactor"),
];

const OBJ_FBX_MAPPING: &[Mapping] = &[
    m(MaterialFbxMap::AmbientColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Ka"),
    m(MaterialFbxMap::DiffuseColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Kd"),
    m(MaterialFbxMap::SpecularColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Ks"),
    m(MaterialFbxMap::EmissionColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Ke"),
    m(MaterialFbxMap::SpecularExponent as u8, 0, T_IDENTITY, "Ns"),
    m(MaterialFbxMap::TransparencyFactor as u8, 0, T_INVERT_X, "d"),
    m(MaterialFbxMap::NormalMap as u8, 0, T_IDENTITY, "norm"),
    m(MaterialFbxMap::Displacement as u8, 0, T_IDENTITY, "disp"),
    m(MaterialFbxMap::Bump as u8, 0, T_IDENTITY, "bump"),
    m(MaterialFbxMap::Bump as u8, 0, T_IDENTITY, "Bump"),
];

const FBX_LAMBERT_SHADER_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "Diffuse"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "DiffuseColor"),
    m(MaterialPbrMap::BaseFactor as u8, 0, T_IDENTITY, "DiffuseFactor"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "Transparent"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "TransparentColor"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "TransparentFactor"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "TransparencyFactor"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "Emissive"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "EmissiveColor"),
    m(MaterialPbrMap::EmissionFactor as u8, 0, T_IDENTITY, "EmissiveFactor"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "NormalMap"),
];

const FBX_PHONG_SHADER_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "Diffuse"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "DiffuseColor"),
    m(MaterialPbrMap::BaseFactor as u8, 0, T_IDENTITY, "DiffuseFactor"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "Specular"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "SpecularColor"),
    m(MaterialPbrMap::SpecularFactor as u8, 0, T_IDENTITY, "SpecularFactor"),
    m(MaterialPbrMap::Roughness as u8, 0, T_UNKNOWN_SHININESS, "Shininess"),
    m(MaterialPbrMap::Roughness as u8, 0, T_UNKNOWN_SHININESS, "ShininessExponent"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "Transparent"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "TransparentColor"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "TransparentFactor"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "TransparencyFactor"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "Emissive"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "EmissiveColor"),
    m(MaterialPbrMap::EmissionFactor as u8, 0, T_IDENTITY, "EmissiveFactor"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "NormalMap"),
];

const OSL_STANDARD_SHADER_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseFactor as u8, 0, T_IDENTITY, "base"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "base_color"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "specular_roughness"),
    m(MaterialPbrMap::DiffuseRoughness as u8, 0, T_IDENTITY, "diffuse_roughness"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "metalness"),
    m(MaterialPbrMap::SpecularFactor as u8, 0, T_IDENTITY, "specular"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "specular_color"),
    m(MaterialPbrMap::SpecularIor as u8, 0, T_IDENTITY, "specular_IOR"),
    m(MaterialPbrMap::SpecularAnisotropy as u8, 0, T_IDENTITY, "specular_anisotropy"),
    m(MaterialPbrMap::SpecularRotation as u8, 0, T_IDENTITY, "specular_rotation"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "transmission"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "transmission_color"),
    m(MaterialPbrMap::TransmissionDepth as u8, 0, T_IDENTITY, "transmission_depth"),
    m(MaterialPbrMap::TransmissionScatter as u8, WIDEN_TO_RGB, T_IDENTITY, "transmission_scatter"),
    m(MaterialPbrMap::TransmissionScatterAnisotropy as u8, 0, T_IDENTITY, "transmission_scatter_anisotropy"),
    m(MaterialPbrMap::TransmissionDispersion as u8, 0, T_IDENTITY, "transmission_dispersion"),
    m(MaterialPbrMap::TransmissionExtraRoughness as u8, 0, T_IDENTITY, "transmission_extra_roughness"),
    m(MaterialPbrMap::SubsurfaceFactor as u8, 0, T_IDENTITY, "subsurface"),
    m(MaterialPbrMap::SubsurfaceColor as u8, DEFAULT_W_1, T_IDENTITY, "subsurface_color"),
    m(MaterialPbrMap::SubsurfaceRadius as u8, WIDEN_TO_RGB, T_IDENTITY, "subsurface_radius"),
    m(MaterialPbrMap::SubsurfaceScale as u8, 0, T_IDENTITY, "subsurface_scale"),
    m(MaterialPbrMap::SubsurfaceAnisotropy as u8, 0, T_IDENTITY, "subsurface_anisotropy"),
    m(MaterialPbrMap::SheenFactor as u8, 0, T_IDENTITY, "sheen"),
    m(MaterialPbrMap::SheenColor as u8, DEFAULT_W_1, T_IDENTITY, "sheen_color"),
    m(MaterialPbrMap::SheenRoughness as u8, 0, T_IDENTITY, "sheen_roughness"),
    m(MaterialPbrMap::CoatFactor as u8, 0, T_IDENTITY, "coat"),
    m(MaterialPbrMap::CoatColor as u8, DEFAULT_W_1, T_IDENTITY, "coat_color"),
    m(MaterialPbrMap::CoatRoughness as u8, 0, T_IDENTITY, "coat_roughness"),
    m(MaterialPbrMap::CoatIor as u8, 0, T_IDENTITY, "coat_IOR"),
    m(MaterialPbrMap::CoatAnisotropy as u8, 0, T_IDENTITY, "coat_anisotropy"),
    m(MaterialPbrMap::CoatRotation as u8, 0, T_IDENTITY, "coat_rotation"),
    m(MaterialPbrMap::CoatNormal as u8, 0, T_IDENTITY, "coat_normal"),
    m(MaterialPbrMap::CoatAffectBaseColor as u8, DEFAULT_W_1, T_IDENTITY, "coat_affect_color"),
    m(MaterialPbrMap::CoatAffectBaseRoughness as u8, 0, T_IDENTITY, "coat_affect_roughness"),
    m(MaterialPbrMap::ThinFilmThickness as u8, 0, T_IDENTITY, "thin_film_thickness"),
    m(MaterialPbrMap::ThinFilmIor as u8, 0, T_IDENTITY, "thin_film_IOR"),
    m(MaterialPbrMap::EmissionFactor as u8, 0, T_IDENTITY, "emission"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "emission_color"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_IDENTITY, "opacity"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "NormalMap"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "normalCamera"),
    m(MaterialPbrMap::TangentMap as u8, 0, T_IDENTITY, "tangent"),
];

const OSL_STANDARD_SHADER_FEATURES: &[Mapping] = &[m(MaterialFeature::ThinWalled as u8, 0, T_IDENTITY, "thin_walled")];

const ARNOLD_SHADER_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseFactor as u8, 0, T_IDENTITY, "base"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "baseColor"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "specularRoughness"),
    m(MaterialPbrMap::DiffuseRoughness as u8, 0, T_IDENTITY, "diffuseRoughness"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "metalness"),
    m(MaterialPbrMap::SpecularFactor as u8, 0, T_IDENTITY, "specular"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "specularColor"),
    m(MaterialPbrMap::SpecularIor as u8, 0, T_IDENTITY, "specularIOR"),
    m(MaterialPbrMap::SpecularAnisotropy as u8, 0, T_IDENTITY, "specularAnisotropy"),
    m(MaterialPbrMap::SpecularRotation as u8, 0, T_IDENTITY, "specularRotation"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "transmission"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "transmissionColor"),
    m(MaterialPbrMap::TransmissionDepth as u8, 0, T_IDENTITY, "transmissionDepth"),
    m(MaterialPbrMap::TransmissionScatter as u8, WIDEN_TO_RGB, T_IDENTITY, "transmissionScatter"),
    m(MaterialPbrMap::TransmissionScatterAnisotropy as u8, 0, T_IDENTITY, "transmissionScatterAnisotropy"),
    m(MaterialPbrMap::TransmissionDispersion as u8, 0, T_IDENTITY, "transmissionDispersion"),
    m(MaterialPbrMap::TransmissionExtraRoughness as u8, 0, T_IDENTITY, "transmissionExtraRoughness"),
    m(MaterialPbrMap::SubsurfaceFactor as u8, 0, T_IDENTITY, "subsurface"),
    m(MaterialPbrMap::SubsurfaceColor as u8, DEFAULT_W_1, T_IDENTITY, "subsurfaceColor"),
    m(MaterialPbrMap::SubsurfaceRadius as u8, WIDEN_TO_RGB, T_IDENTITY, "subsurfaceRadius"),
    m(MaterialPbrMap::SubsurfaceScale as u8, 0, T_IDENTITY, "subsurfaceScale"),
    m(MaterialPbrMap::SubsurfaceAnisotropy as u8, 0, T_IDENTITY, "subsurfaceAnisotropy"),
    m(MaterialPbrMap::SheenFactor as u8, 0, T_IDENTITY, "sheen"),
    m(MaterialPbrMap::SheenColor as u8, DEFAULT_W_1, T_IDENTITY, "sheenColor"),
    m(MaterialPbrMap::SheenRoughness as u8, 0, T_IDENTITY, "sheenRoughness"),
    m(MaterialPbrMap::CoatFactor as u8, 0, T_IDENTITY, "coat"),
    m(MaterialPbrMap::CoatColor as u8, DEFAULT_W_1, T_IDENTITY, "coatColor"),
    m(MaterialPbrMap::CoatRoughness as u8, 0, T_IDENTITY, "coatRoughness"),
    m(MaterialPbrMap::CoatIor as u8, 0, T_IDENTITY, "coatIOR"),
    m(MaterialPbrMap::CoatAnisotropy as u8, 0, T_IDENTITY, "coatAnisotropy"),
    m(MaterialPbrMap::CoatRotation as u8, 0, T_IDENTITY, "coatRotation"),
    m(MaterialPbrMap::CoatNormal as u8, 0, T_IDENTITY, "coatNormal"),
    m(MaterialPbrMap::ThinFilmThickness as u8, 0, T_IDENTITY, "thinFilmThickness"),
    m(MaterialPbrMap::ThinFilmIor as u8, 0, T_IDENTITY, "thinFilmIOR"),
    m(MaterialPbrMap::EmissionFactor as u8, 0, T_IDENTITY, "emission"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "emissionColor"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_IDENTITY, "opacity"),
    m(MaterialPbrMap::IndirectDiffuse as u8, 0, T_IDENTITY, "indirectDiffuse"),
    m(MaterialPbrMap::IndirectSpecular as u8, 0, T_IDENTITY, "indirectSpecular"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "NormalMap"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "normalCamera"),
    m(MaterialPbrMap::TangentMap as u8, 0, T_IDENTITY, "tangent"),
    m(MaterialPbrMap::MatteColor as u8, DEFAULT_W_1, T_IDENTITY, "aiMatteColor"),
    m(MaterialPbrMap::MatteFactor as u8, 0, T_IDENTITY, "aiMatteColorA"),
    m(MaterialPbrMap::SubsurfaceType as u8, 0, T_IDENTITY, "subsurfaceType"),
    m(MaterialPbrMap::TransmissionPriority as u8, 0, T_IDENTITY, "dielectricPriority"),
    m(MaterialPbrMap::TransmissionEnableInAov as u8, 0, T_IDENTITY, "transmitAovs"),
];

const ARNOLD_SHADER_FEATURES: &[Mapping] = &[
    m(MaterialFeature::Matte as u8, 0, T_IDENTITY, "aiEnableMatte"),
    m(MaterialFeature::ThinWalled as u8, 0, T_IDENTITY, "thinWalled"),
    m(MaterialFeature::Caustics as u8, 0, T_IDENTITY, "caustics"),
    m(MaterialFeature::InternalReflections as u8, 0, T_IDENTITY, "internalReflections"),
    m(MaterialFeature::ExitToBackground as u8, 0, T_IDENTITY, "exitToBackground"),
];

const MAX3DS_PHYSICAL_MATERIAL_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseFactor as u8, 0, T_IDENTITY, "base_weight"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "base_color"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "roughness"),
    m(MaterialPbrMap::DiffuseRoughness as u8, 0, T_IDENTITY, "diff_rough"),
    m(MaterialPbrMap::DiffuseRoughness as u8, 0, T_IDENTITY, "diff_roughness"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "metalness"),
    m(MaterialPbrMap::SpecularFactor as u8, 0, T_IDENTITY, "reflectivity"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "refl_color"),
    m(MaterialPbrMap::SpecularAnisotropy as u8, 0, T_IDENTITY, "anisotropy"),
    m(MaterialPbrMap::SpecularRotation as u8, 0, T_IDENTITY, "aniso_angle"),
    m(MaterialPbrMap::SpecularRotation as u8, 0, T_IDENTITY, "anisoangle"),
    m(MaterialPbrMap::SpecularIor as u8, 0, T_IDENTITY, "trans_ior"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "transparency"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "trans_color"),
    m(MaterialPbrMap::TransmissionDepth as u8, 0, T_IDENTITY, "trans_depth"),
    m(MaterialPbrMap::TransmissionRoughness as u8, 0, T_IDENTITY, "trans_rough"),
    m(MaterialPbrMap::TransmissionRoughness as u8, 0, T_IDENTITY, "trans_roughness"),
    m(MaterialPbrMap::SubsurfaceFactor as u8, 0, T_IDENTITY, "scattering"),
    m(MaterialPbrMap::SubsurfaceTintColor as u8, DEFAULT_W_1, T_IDENTITY, "sss_color"),
    m(MaterialPbrMap::SubsurfaceColor as u8, DEFAULT_W_1, T_IDENTITY, "sss_scatter_color"),
    m(MaterialPbrMap::SubsurfaceRadius as u8, WIDEN_TO_RGB, T_IDENTITY, "sss_depth"),
    m(MaterialPbrMap::SubsurfaceScale as u8, 0, T_IDENTITY, "sss_scale"),
    m(MaterialPbrMap::CoatFactor as u8, 0, T_IDENTITY, "coat"),
    m(MaterialPbrMap::CoatFactor as u8, 0, T_IDENTITY, "coating"),
    m(MaterialPbrMap::CoatColor as u8, DEFAULT_W_1, T_IDENTITY, "coat_color"),
    m(MaterialPbrMap::CoatRoughness as u8, 0, T_IDENTITY, "coat_rough"),
    m(MaterialPbrMap::CoatRoughness as u8, 0, T_IDENTITY, "coat_roughness"),
    m(MaterialPbrMap::CoatIor as u8, 0, T_IDENTITY, "coat_ior"),
    m(MaterialPbrMap::CoatNormal as u8, 0, T_IDENTITY, "coat_bump"),
    m(MaterialPbrMap::CoatNormal as u8, 0, T_IDENTITY, "clearcoat_bump_map_amt"),
    m(MaterialPbrMap::CoatAffectBaseColor as u8, DEFAULT_W_1, T_IDENTITY, "coat_affect_color"),
    m(MaterialPbrMap::CoatAffectBaseRoughness as u8, 0, T_IDENTITY, "coat_affect_roughness"),
    m(MaterialPbrMap::EmissionFactor as u8, 0, T_IDENTITY, "emission"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "emit_color"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_IDENTITY, "cutout"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "bump"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "bump_map_amt"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "displacement"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "displacement_map_amt"),
    m(MaterialPbrMap::SubsurfaceType as u8, 0, T_IDENTITY, "subsurfaceType"),
];

const MAX3DS_PHYSICAL_MATERIAL_FEATURES: &[Mapping] = &[
    m(MaterialFeature::ThinWalled as u8, 0, T_IDENTITY, "thin_walled"),
    m(MaterialFeature::Specular as u8, 0, T_IDENTITY, "material_mode"),
    m(MaterialFeature::DiffuseRoughness as u8, 0, T_IDENTITY, "material_mode"),
    m(MaterialFeature::TransmissionRoughness as u8, F_INVERTED, T_IDENTITY, "trans_roughness_lock"),
    m(MaterialFeature::RoughnessAsGlossiness as u8, 0, T_IDENTITY, "roughness_inv"),
    m(MaterialFeature::TransmissionRoughnessAsGlossiness as u8, 0, T_IDENTITY, "trans_roughness_inv"),
    m(MaterialFeature::CoatRoughnessAsGlossiness as u8, 0, T_IDENTITY, "coat_roughness_inv"),
];

const GLTF_MATERIAL_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "main|baseColor"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "main|roughness"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "main|metalness"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "main|normal"),
    m(MaterialPbrMap::AmbientOcclusion as u8, 0, T_IDENTITY, "main|ambientOcclusion"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "main|emission"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "main|emissionColor"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_IDENTITY, "main|Alpha"),
    m(MaterialPbrMap::CoatFactor as u8, 0, T_IDENTITY, "extension|clearcoat"),
    m(MaterialPbrMap::CoatRoughness as u8, 0, T_IDENTITY, "extension|clearcoatRoughness"),
    m(MaterialPbrMap::CoatNormal as u8, 0, T_IDENTITY, "extension|clearcoatNormal"),
    m(MaterialPbrMap::SheenColor as u8, DEFAULT_W_1, T_IDENTITY, "extension|sheenColor"),
    m(MaterialPbrMap::SheenRoughness as u8, 0, T_IDENTITY, "extension|sheenRoughness"),
    m(MaterialPbrMap::SpecularFactor as u8, 0, T_IDENTITY, "extension|specular"),
    m(MaterialPbrMap::SpecularFactor as u8, 0, T_IDENTITY, "extension|Specular"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "extension|specularcolor"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "extension|specularColor"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "extension|transmission"),
    m(MaterialPbrMap::SpecularIor as u8, 0, T_IDENTITY, "extension|indexOfRefraction"),
];

const OPENPBR_MATERIAL_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseFactor as u8, 0, T_IDENTITY, "base_weight"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "base_color"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "specular_roughness"),
    m(MaterialPbrMap::DiffuseRoughness as u8, 0, T_IDENTITY, "base_diffuse_roughness"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "base_metalness"),
    m(MaterialPbrMap::SpecularFactor as u8, 0, T_IDENTITY, "specular_weight"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "specular_color"),
    m(MaterialPbrMap::SpecularAnisotropy as u8, 0, T_IDENTITY, "specular_roughness_anisotropy"),
    m(MaterialPbrMap::SpecularIor as u8, 0, T_IDENTITY, "specular_ior"),
    m(MaterialPbrMap::TransmissionFactor as u8, 0, T_IDENTITY, "transmission_weight"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "transmission_color"),
    m(MaterialPbrMap::TransmissionDepth as u8, 0, T_IDENTITY, "transmission_depth"),
    m(MaterialPbrMap::TransmissionScatter as u8, WIDEN_TO_RGB, T_IDENTITY, "transmission_scatter"),
    m(MaterialPbrMap::TransmissionScatterAnisotropy as u8, 0, T_IDENTITY, "transmission_scatter_anisotropy"),
    m(MaterialPbrMap::TransmissionDispersion as u8, 0, T_IDENTITY, "transmission_dispersion_scale"),
    m(MaterialPbrMap::SubsurfaceFactor as u8, 0, T_IDENTITY, "subsurface_weight"),
    m(MaterialPbrMap::SubsurfaceColor as u8, DEFAULT_W_1, T_IDENTITY, "subsurface_color"),
    m(MaterialPbrMap::SubsurfaceRadius as u8, WIDEN_TO_RGB, T_IDENTITY, "subsurface_radius_scale"),
    m(MaterialPbrMap::SubsurfaceScale as u8, 0, T_IDENTITY, "subsurface_radius"),
    m(MaterialPbrMap::SubsurfaceAnisotropy as u8, 0, T_IDENTITY, "subsurface_scatter_anisotropy"),
    m(MaterialPbrMap::CoatFactor as u8, 0, T_IDENTITY, "coat_weight"),
    m(MaterialPbrMap::CoatColor as u8, DEFAULT_W_1, T_IDENTITY, "coat_color"),
    m(MaterialPbrMap::CoatRoughness as u8, 0, T_IDENTITY, "coat_roughness"),
    m(MaterialPbrMap::CoatAnisotropy as u8, 0, T_IDENTITY, "coat_roughness_anisotropy"),
    m(MaterialPbrMap::CoatIor as u8, 0, T_IDENTITY, "coat_ior"),
    m(MaterialPbrMap::CoatNormal as u8, 0, T_IDENTITY, "coat_normal_map"),
    m(MaterialPbrMap::SheenFactor as u8, 0, T_IDENTITY, "fuzz_weight"),
    m(MaterialPbrMap::SheenColor as u8, DEFAULT_W_1, T_IDENTITY, "fuzz_color"),
    m(MaterialPbrMap::SheenRoughness as u8, 0, T_IDENTITY, "fuzz_roughness"),
    m(MaterialPbrMap::EmissionFactor as u8, 0, T_IDENTITY, "emission_weight"),
    m(MaterialPbrMap::EmissionFactor as u8, MULTIPLY_VALUE, T_IDENTITY, "emission_luminance"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "emission_color"),
    m(MaterialPbrMap::ThinFilmFactor as u8, 0, T_IDENTITY, "thin_film_weight"),
    m(MaterialPbrMap::ThinFilmThickness as u8, 0, T_IDENTITY, "thin_film_thickness"),
    m(MaterialPbrMap::ThinFilmIor as u8, 0, T_IDENTITY, "thin_film_ior"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "bump"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "bump_map_amt"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "displacement"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "displacement_map_amt"),
    m(MaterialPbrMap::CoatNormal as u8, 0, T_IDENTITY, "coat_bump"),
    m(MaterialPbrMap::CoatNormal as u8, 0, T_IDENTITY, "coat_bump_map_amt"),
    m(MaterialPbrMap::TangentMap as u8, 0, T_IDENTITY, "geometry_tangent_map"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_IDENTITY, "geometry_opacity"),
];

const OPENPBR_MATERIAL_FEATURES: &[Mapping] = &[m(MaterialFeature::ThinWalled as u8, 0, T_IDENTITY, "geometry_thin_walled")];

const MAX3DS_PBR_METAL_ROUGH_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "base_color"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "baseColor"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "roughness"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "Roughness_Map"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "metalness"),
    m(MaterialPbrMap::AmbientOcclusion as u8, 0, T_IDENTITY, "ao"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "norm"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "emit_color"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "displacement"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "displacement_amt"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_IDENTITY, "opacity"),
];

const MAX3DS_PBR_SPEC_GLOSS_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "base_color"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "baseColor"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "Specular"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1, T_IDENTITY, "specular"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "glossiness"),
    m(MaterialPbrMap::AmbientOcclusion as u8, 0, T_IDENTITY, "ao"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "norm"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "emit_color"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "displacement"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "displacement_amt"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_IDENTITY, "opacity"),
];

const MAX3DS_PBR_FEATURES: &[Mapping] = &[m(MaterialFeature::RoughnessAsGlossiness as u8, F_IF_AROUND_1, T_IDENTITY, "useGlossiness")];

const GLTF_MATERIAL_FEATURES: &[Mapping] = &[
    m(MaterialFeature::DoubleSided as u8, 0, T_IDENTITY, "main|DoubleSided"),
    m(MaterialFeature::Sheen as u8, 0, T_IDENTITY, "extension|enableSheen"),
    m(MaterialFeature::Coat as u8, 0, T_IDENTITY, "extension|enableClearCoat"),
    m(MaterialFeature::Transmission as u8, 0, T_IDENTITY, "extension|enableTransmission"),
    m(MaterialFeature::Ior as u8, 0, T_IDENTITY, "extension|enableIndexOfRefraction"),
    m(MaterialFeature::Specular as u8, 0, T_IDENTITY, "extension|enableSpecular"),
    m(MaterialFeature::Unlit as u8, 0, T_IDENTITY, "extension|unlit"),
];

const SHADERFX_GRAPH_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "color"),
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "base_color"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "roughness"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "metallic"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "normal"),
    m(MaterialPbrMap::EmissionFactor as u8, 0, T_IDENTITY, "emissive_intensity"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "emissive"),
    m(MaterialPbrMap::AmbientOcclusion as u8, 0, T_IDENTITY, "ao"),
];

const BLENDER_PHONG_SHADER_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1, T_IDENTITY, "DiffuseColor"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_BLENDER_OPACITY, "TransparencyFactor"),
    m(MaterialPbrMap::EmissionFactor as u8, 0, T_IDENTITY, "EmissiveFactor"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1, T_IDENTITY, "EmissiveColor"),
    m(MaterialPbrMap::Roughness as u8, 0, T_BLENDER_SHININESS, "Shininess"),
    m(MaterialPbrMap::Roughness as u8, 0, T_BLENDER_SHININESS, "ShininessExponent"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "ReflectionFactor"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "NormalMap"),
];

const OBJ_PBR_MAPPING: &[Mapping] = &[
    m(MaterialPbrMap::BaseColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Kd"),
    m(MaterialPbrMap::SpecularColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Ks"),
    m(MaterialPbrMap::EmissionColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Ke"),
    m(MaterialPbrMap::Roughness as u8, 0, T_UNKNOWN_SHININESS, "Ns"),
    m(MaterialPbrMap::Roughness as u8, 0, T_IDENTITY, "Pr"),
    m(MaterialPbrMap::SpecularIor as u8, 0, T_IDENTITY, "Ni"),
    m(MaterialPbrMap::Metalness as u8, 0, T_IDENTITY, "Pm"),
    m(MaterialPbrMap::Opacity as u8, WIDEN_TO_RGB, T_IDENTITY, "d"),
    m(MaterialPbrMap::TransmissionColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Tf"),
    m(MaterialPbrMap::DisplacementMap as u8, 0, T_IDENTITY, "disp"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "bump"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "Bump"),
    m(MaterialPbrMap::NormalMap as u8, 0, T_IDENTITY, "norm"),
    m(MaterialPbrMap::SheenColor as u8, DEFAULT_W_1 | WIDEN_TO_RGB, T_IDENTITY, "Ps"),
    m(MaterialPbrMap::CoatFactor as u8, 0, T_IDENTITY, "Pc"),
    m(MaterialPbrMap::CoatRoughness as u8, 0, T_IDENTITY, "Pcr"),
    m(MaterialPbrMap::SpecularAnisotropy as u8, 0, T_IDENTITY, "aniso"),
    m(MaterialPbrMap::SpecularRotation as u8, 0, T_IDENTITY, "anisor"),
];

const OBJ_FEATURES: &[Mapping] = &[
    m(MaterialFeature::Pbr as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "Pr"),
    m(MaterialFeature::Pbr as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "Pm"),
    m(MaterialFeature::Sheen as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "Ps"),
    m(MaterialFeature::Coat as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "Pc"),
    m(MaterialFeature::Metalness as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "Pm"),
    m(MaterialFeature::Ior as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "Ni"),
    m(MaterialFeature::Opacity as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "d"),
    m(MaterialFeature::Transmission as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "Tf"),
    m(MaterialFeature::Emission as u8, F_IF_EXISTS_OR_TEXTURE, T_IDENTITY, "Ke"),
];

const fn mat(f: MaterialFeature) -> u32 {
    1 << f as u32
}

const PBR: u32 = mat(MaterialFeature::Pbr);
const METALNESS: u32 = mat(MaterialFeature::Metalness);
const DIFFUSE: u32 = mat(MaterialFeature::Diffuse);
const SPECULAR: u32 = mat(MaterialFeature::Specular);
const EMISSION: u32 = mat(MaterialFeature::Emission);
const COAT: u32 = mat(MaterialFeature::Coat);
const SHEEN: u32 = mat(MaterialFeature::Sheen);
const TRANSMISSION: u32 = mat(MaterialFeature::Transmission);
const OPACITY: u32 = mat(MaterialFeature::Opacity);
const AMBIENT_OCCLUSION: u32 = mat(MaterialFeature::AmbientOcclusion);
const IOR: u32 = mat(MaterialFeature::Ior);
const DIFFUSE_ROUGHNESS: u32 = mat(MaterialFeature::DiffuseRoughness);

struct MappingList {
    data: &'static [Mapping],
    features: &'static [Mapping],
    default_features: u32,
    texture_prefix: &'static str,
    texture_suffix: &'static str,
    texture_enabled_prefix: &'static str,
    texture_enabled_suffix: &'static str,
}

const fn list(data: &'static [Mapping], features: &'static [Mapping], default_features: u32) -> MappingList {
    MappingList { data, features, default_features, texture_prefix: "", texture_suffix: "", texture_enabled_prefix: "", texture_enabled_suffix: "" }
}

const fn list_tex(
    data: &'static [Mapping],
    features: &'static [Mapping],
    default_features: u32,
    texture: (&'static str, &'static str),
    texture_enabled: (&'static str, &'static str),
) -> MappingList {
    MappingList {
        data,
        features,
        default_features,
        texture_prefix: texture.0,
        texture_suffix: texture.1,
        texture_enabled_prefix: texture_enabled.0,
        texture_enabled_suffix: texture_enabled.1,
    }
}

/// In the order of `ShaderType`
const SHADER_PBR_MAPPINGS: [MappingList; 13] = [
    list(FBX_PHONG_SHADER_PBR_MAPPING, &[], DIFFUSE | SPECULAR | EMISSION | TRANSMISSION),
    list(FBX_LAMBERT_SHADER_PBR_MAPPING, &[], DIFFUSE | EMISSION | TRANSMISSION),
    list(FBX_PHONG_SHADER_PBR_MAPPING, &[], DIFFUSE | SPECULAR | EMISSION | TRANSMISSION),
    list(
        OSL_STANDARD_SHADER_PBR_MAPPING,
        OSL_STANDARD_SHADER_FEATURES,
        PBR | METALNESS | DIFFUSE | SPECULAR | COAT | SHEEN | TRANSMISSION | OPACITY | IOR | DIFFUSE_ROUGHNESS,
    ),
    list(
        ARNOLD_SHADER_PBR_MAPPING,
        ARNOLD_SHADER_FEATURES,
        PBR | METALNESS | DIFFUSE | SPECULAR | COAT | SHEEN | TRANSMISSION | OPACITY | IOR | DIFFUSE_ROUGHNESS,
    ),
    list_tex(
        MAX3DS_PHYSICAL_MATERIAL_PBR_MAPPING,
        MAX3DS_PHYSICAL_MATERIAL_FEATURES,
        PBR | METALNESS | DIFFUSE | COAT | SHEEN | TRANSMISSION | OPACITY | IOR,
        ("", "_map"),
        ("", "_map_on"),
    ),
    list_tex(MAX3DS_PBR_METAL_ROUGH_PBR_MAPPING, MAX3DS_PBR_FEATURES, PBR | METALNESS | DIFFUSE | OPACITY, ("", "_map"), ("", "")),
    list_tex(MAX3DS_PBR_SPEC_GLOSS_PBR_MAPPING, MAX3DS_PBR_FEATURES, PBR | SPECULAR | DIFFUSE | OPACITY, ("", "_map"), ("", "")),
    list_tex(GLTF_MATERIAL_PBR_MAPPING, GLTF_MATERIAL_FEATURES, PBR | METALNESS | DIFFUSE | EMISSION | OPACITY | AMBIENT_OCCLUSION, ("", "Map"), ("", "")),
    list_tex(
        OPENPBR_MATERIAL_PBR_MAPPING,
        OPENPBR_MATERIAL_FEATURES,
        PBR | METALNESS | DIFFUSE | SPECULAR | COAT | SHEEN | TRANSMISSION | OPACITY | IOR | DIFFUSE_ROUGHNESS,
        ("", "_map"),
        ("", "_map_on"),
    ),
    list_tex(SHADERFX_GRAPH_PBR_MAPPING, &[], PBR | METALNESS | DIFFUSE | EMISSION | AMBIENT_OCCLUSION, ("TEX_", "_map"), ("use_", "_map")),
    list(BLENDER_PHONG_SHADER_PBR_MAPPING, &[], PBR | METALNESS | DIFFUSE | EMISSION),
    list(OBJ_PBR_MAPPING, OBJ_FEATURES, DIFFUSE | SPECULAR),
];

const FETCH_VALUE: u32 = 0x1;
const FETCH_TEXTURE: u32 = 0x2;
const FETCH_TEXTURE_ENABLED: u32 = 0x4;
const FETCH_FEATURE: u32 = 0x8;

enum Dst<'a> {
    Maps(&'a mut [MaterialMap]),
    Features(&'a mut [MaterialFeatureInfo]),
}

#[allow(clippy::too_many_arguments)]
fn fetch_mapping_maps(
    scene: &Scene,
    material: &Material,
    dst: &mut Dst,
    shader: Option<&Shader>,
    mappings: &[Mapping],
    prefix: &str,
    prefix2: &str,
    suffix: &str,
    flags: u32,
) {
    for mapping in mappings {
        let mut prop_name = mapping.prop.to_string();
        if (!prefix.is_empty() || !prefix2.is_empty() || !suffix.is_empty()) && mapping.prop.len() + prefix.len() + prefix2.len() + suffix.len() <= 512 {
            prop_name = [prefix, prefix2, mapping.prop, suffix].concat();
        }

        let found = shader.map_or(&[][..], |s| crate::api::find_shader_prop_bindings(scene, s, &prop_name));
        let identity = [ShaderPropBinding { material_prop: prop_name.clone(), shader_prop: String::new() }];
        let bindings = if found.is_empty() { &identity[..] } else { found };

        for binding in bindings {
            let name = binding.material_prop.as_str();
            let prop = material.element.props.find(name);
            match dst {
                Dst::Features(features) => {
                    let feature = &mut features[mapping.index as usize];
                    if let Some(prop) = prop
                        && prop.prop_type != PropType::Reference
                    {
                        feature.enabled = prop.value_int != 0;
                        feature.is_explicit = true;
                        if mapping.flags & F_IF_AROUND_1 != 0 {
                            feature.enabled = prop.value_real() >= 0.5 && prop.value_real() <= 1.5;
                        }
                        if mapping.flags & F_INVERTED != 0 {
                            feature.enabled = !feature.enabled;
                        }
                        if mapping.flags & F_IF_EXISTS != 0 {
                            feature.enabled = true;
                        }
                    }
                    if mapping.flags & F_IF_TEXTURE != 0 && crate::api::find_prop_texture(material, name).is_some() {
                        feature.enabled = true;
                    }
                }
                Dst::Maps(maps) => {
                    let map = &mut maps[mapping.index as usize];
                    if flags & FETCH_VALUE != 0
                        && let Some(prop) = prop
                        && prop.prop_type != PropType::Reference
                    {
                        if mapping.flags & MULTIPLY_VALUE != 0 {
                            map.value_vec4.x *= prop.value_real_arr[0];
                            map.value_int = f64_to_i64(map.value_vec4.x);
                        } else {
                            map.value_vec4 = prop.value_vec4();
                            map.value_int = prop.value_int;
                        }
                        map.has_value = true;
                        if mapping.transform != T_IDENTITY {
                            mat_transform(mapping.transform, &mut map.value_vec4);
                        }
                        let prop_flags = prop.flags;
                        if mapping.flags & DEFAULT_W_1 != 0 && prop_flags & prop_flags::VALUE_VEC4 == 0 {
                            map.value_vec4.w = 1.0;
                        }
                        if mapping.flags & WIDEN_TO_RGB != 0 && prop_flags & prop_flags::VALUE_REAL != 0 {
                            map.value_vec4.y = map.value_vec4.x;
                            map.value_vec4.z = map.value_vec4.x;
                        }
                        map.value_components = if prop_flags & prop_flags::VALUE_REAL != 0 {
                            1
                        } else if prop_flags & prop_flags::VALUE_VEC2 != 0 {
                            2
                        } else if prop_flags & prop_flags::VALUE_VEC3 != 0 {
                            3
                        } else if prop_flags & prop_flags::VALUE_VEC4 != 0 {
                            4
                        } else {
                            0
                        };
                    }
                    if flags & FETCH_TEXTURE != 0
                        && let Some(texture) = crate::api::find_prop_texture(material, name)
                    {
                        map.texture = Some(texture);
                        map.texture_enabled = true;
                    }
                    if flags & FETCH_TEXTURE_ENABLED != 0
                        && let Some(prop) = prop
                    {
                        map.texture_enabled = prop.value_int != 0;
                    }
                }
            }
        }
    }
}

fn update_factor(maps: &mut [MaterialMap], factor: usize, color: usize) {
    if !maps[factor].has_value {
        if maps[color].has_value && !crate::math::is_vec4_zero(maps[color].value_vec4) {
            maps[factor].value_vec4.x = 1.0;
            maps[factor].value_int = 1;
        } else {
            maps[factor].value_vec4.x = 0.0;
            maps[factor].value_int = 0;
        }
    }
}

const GLOSSINESS_REMAPS: [(MaterialFeature, MaterialPbrMap, MaterialPbrMap); 3] = [
    (MaterialFeature::RoughnessAsGlossiness, MaterialPbrMap::Roughness, MaterialPbrMap::Glossiness),
    (MaterialFeature::CoatRoughnessAsGlossiness, MaterialPbrMap::CoatRoughness, MaterialPbrMap::CoatGlossiness),
    (MaterialFeature::TransmissionRoughnessAsGlossiness, MaterialPbrMap::TransmissionRoughness, MaterialPbrMap::TransmissionGlossiness),
];

pub(crate) fn fetch_maps(scene: &mut Scene, material: usize) {
    let (fbx, pbr, features) = {
        let scene_ref: &Scene = scene;
        let mat = &scene_ref.materials[material];
        let shader = mat.shader.map(|s| &scene_ref.shaders[s]);

        let mut fbx = MaterialFbxMaps::default();
        let mut pbr = MaterialPbrMaps::default();
        let mut features = MaterialFeatures::default();

        let base_mapping = if matches!(scene_ref.metadata.file_format, FileFormat::Obj | FileFormat::Mtl) { OBJ_FBX_MAPPING } else { BASE_FBX_MAPPING };
        fetch_mapping_maps(scene_ref, mat, &mut Dst::Maps(&mut fbx.maps), None, base_mapping, "", "", "", FETCH_VALUE | FETCH_TEXTURE);

        let list = &SHADER_PBR_MAPPINGS[mat.shader_type as usize];
        for i in 0..MaterialFeature::COUNT {
            if list.default_features & (1u32 << i) != 0 {
                features.features[i].enabled = true;
            }
        }

        let prefix = if shader.is_none() { mat.shader_prop_prefix.as_str() } else { "" };

        if !list.texture_prefix.is_empty() || !list.texture_suffix.is_empty() {
            fetch_mapping_maps(
                scene_ref,
                mat,
                &mut Dst::Maps(&mut pbr.maps),
                shader,
                list.data,
                prefix,
                list.texture_prefix,
                list.texture_suffix,
                FETCH_TEXTURE,
            );
        }
        fetch_mapping_maps(scene_ref, mat, &mut Dst::Maps(&mut pbr.maps), shader, list.data, prefix, "", "", FETCH_VALUE | FETCH_TEXTURE);
        if !list.texture_enabled_prefix.is_empty() || !list.texture_enabled_suffix.is_empty() {
            fetch_mapping_maps(
                scene_ref,
                mat,
                &mut Dst::Maps(&mut pbr.maps),
                shader,
                list.data,
                prefix,
                list.texture_enabled_prefix,
                list.texture_enabled_suffix,
                FETCH_TEXTURE_ENABLED,
            );
        }
        fetch_mapping_maps(scene_ref, mat, &mut Dst::Features(&mut features.features), shader, list.features, prefix, "", "", FETCH_FEATURE);

        use MaterialFbxMap as F;
        use MaterialPbrMap as P;
        for (factor, color) in [
            (F::DiffuseFactor, F::DiffuseColor),
            (F::SpecularFactor, F::SpecularColor),
            (F::ReflectionFactor, F::ReflectionColor),
            (F::TransparencyFactor, F::TransparencyColor),
            (F::EmissionFactor, F::EmissionColor),
            (F::AmbientFactor, F::AmbientColor),
        ] {
            update_factor(&mut fbx.maps, factor as usize, color as usize);
        }
        for (factor, color) in [
            (P::BaseFactor, P::BaseColor),
            (P::SpecularFactor, P::SpecularColor),
            (P::EmissionFactor, P::EmissionColor),
            (P::SheenFactor, P::SheenColor),
            (P::ThinFilmFactor, P::ThinFilmThickness),
            (P::TransmissionFactor, P::TransmissionColor),
        ] {
            update_factor(&mut pbr.maps, factor as usize, color as usize);
        }

        if !pbr[P::TransmissionRoughness].has_value && pbr[P::Roughness].has_value && pbr[P::TransmissionExtraRoughness].has_value {
            pbr[P::TransmissionRoughness].value_vec4.x = pbr[P::Roughness].value_vec4.x + pbr[P::TransmissionExtraRoughness].value_vec4.x;
        }

        for (feature, roughness_map, glossiness_map) in GLOSSINESS_REMAPS {
            if features[feature].enabled {
                pbr[glossiness_map] = pbr[roughness_map];
                pbr[roughness_map] = MaterialMap::default();
                if pbr[glossiness_map].has_value {
                    pbr[roughness_map].value_vec4.x = 1.0 - pbr[glossiness_map].value_vec4.x;
                }
            } else if pbr[roughness_map].has_value {
                pbr[glossiness_map].value_vec4.x = 1.0 - pbr[roughness_map].value_vec4.x;
            }
        }
        (fbx, pbr, features)
    };
    let mat = &mut scene.materials[material];
    mat.fbx = fbx;
    mat.pbr = pbr;
    mat.features = features;
}

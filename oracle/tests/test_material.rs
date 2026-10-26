mod ufbxt;
use ufbxt::*;

fn round100(v: Real) -> i32 {
    (100.0 * v).round() as i32
}

file_test!(maya_textured_cube, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "phong1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 6);

    check_material_texture(scene, material.fbx.diffuse_color().texture, "checkerboard_diffuse.png", true);
    check_material_texture(scene, material.fbx.specular_color().texture, "checkerboard_specular.png", true);
    check_material_texture(scene, material.fbx.reflection_color().texture, "checkerboard_reflection.png", true);
    check_material_texture(scene, material.fbx.transparency_color().texture, "checkerboard_transparency.png", true);
    check_material_texture(scene, material.fbx.emission_color().texture, "checkerboard_emissive.png", true);
    check_material_texture(scene, material.fbx.ambient_color().texture, "checkerboard_ambient.png", true);
});

file_test!(synthetic_texture_split, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "phong1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 6);

    check_material_texture(scene, material.fbx.diffuse_color().texture, "checkerboard_diffuse.png", true);
    check_material_texture(scene, material.fbx.specular_color().texture, "checkerboard_specular.png", true);
    check_material_texture(scene, material.fbx.reflection_color().texture, "checkerboard_reflection.png", false);
    check_material_texture(scene, material.fbx.transparency_color().texture, "checkerboard_transparency.png", false);
    check_material_texture(scene, material.fbx.emission_color().texture, "checkerboard_emissive.png", true);
    check_material_texture(scene, material.fbx.ambient_color().texture, "checkerboard_ambient.png", true);

    if scene.metadata.ascii {
        check_warning(scene, WarningType::BadBase64Content, ElementType::Video, Some("file2"), 1, None);
        check_warning(scene, WarningType::BadBase64Content, ElementType::Video, Some("file6"), 1, None);
    }
});

#[test]
fn ignore_embedded() {
    for path in FileIterator::new("maya_textured_cube") {
        let opts = LoadOpts { ignore_embedded: true, ..Default::default() };

        let scene = load_file(&path, &opts);
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();
        check_scene(&scene);

        for video in &scene.videos {
            ufbxt_assert!(video.content.is_empty());
        }

        for texture in &scene.textures {
            ufbxt_assert!(texture.content.is_empty());
        }
    }
}

file_test!(maya_shared_textures, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "Shared").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 6);

    ufbxt_assert!(material.shader_type == ShaderType::FbxLambert);
    check_material_texture(scene, material.fbx.diffuse_color().texture, "checkerboard_ambient.png", true); // sic: test has wrong texture
    check_material_texture(scene, material.fbx.diffuse_factor().texture, "checkerboard_diffuse.png", true);
    check_material_texture(scene, material.fbx.emission_color().texture, "checkerboard_emissive.png", true);
    check_material_texture(scene, material.fbx.ambient_color().texture, "checkerboard_ambient.png", true);
    check_material_texture(scene, material.fbx.transparency_color().texture, "checkerboard_transparency.png", true);
    check_material_texture(scene, material.fbx.bump().texture, "checkerboard_bump.png", true);

    let material = find_element(scene, ElementType::Material, "Special").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.unwrap().shader_type == ShaderType::FbxPhong);
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 1);

    ufbxt_assert!(scene.textures[material.fbx.diffuse_color().texture.unwrap()].relative_filename == "textures\\tiny_clouds.png");
});

file_test!(maya_arnold_textures, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "aiStandardSurface1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 5);

    ufbxt_assert!(material.shader_type == ShaderType::ArnoldStandardSurface);
    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", true);
    check_material_texture(scene, material.pbr.specular_color().texture, "checkerboard_specular.png", true);
    check_material_texture(scene, material.pbr.roughness().texture, "checkerboard_roughness.png", true);
    check_material_texture(scene, material.pbr.metalness().texture, "checkerboard_metallic.png", true);
    check_material_texture(scene, material.pbr.diffuse_roughness().texture, "checkerboard_roughness.png", true);
});

file_test!(max_physical_material_properties, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "PhysicalMaterial").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::Max3dsPhysicalMaterial);

    ufbxt_assert!(1 == round100(material.pbr.base_factor().value_real()));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec4.x));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec4.y));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec4.z));
    ufbxt_assert!(5 == round100(material.pbr.base_color().value_vec4.w));
    ufbxt_assert!(6 == round100(material.pbr.roughness().value_real()));
    ufbxt_assert!(7 == round100(material.pbr.metalness().value_real()));
    ufbxt_assert!(80 == round100(material.pbr.specular_ior().value_real())); // 3ds Max doesn't allow lower than 0.1 IOR
    ufbxt_assert!(9 == round100(material.pbr.transmission_factor().value_real()));
    ufbxt_assert!(10 == round100(material.pbr.transmission_color().value_vec4.x));
    ufbxt_assert!(11 == round100(material.pbr.transmission_color().value_vec4.y));
    ufbxt_assert!(12 == round100(material.pbr.transmission_color().value_vec4.z));
    ufbxt_assert!(13 == round100(material.pbr.transmission_color().value_vec4.w));
    ufbxt_assert!(14 == round100(material.pbr.transmission_roughness().value_real()));
    ufbxt_assert!(6 == round100(material.pbr.transmission_depth().value_real())); // ??? Unit conversion probably
    ufbxt_assert!(16 == round100(material.pbr.subsurface_factor().value_real()));
    ufbxt_assert!(17 == round100(material.pbr.subsurface_tint_color().value_vec4.x));
    ufbxt_assert!(18 == round100(material.pbr.subsurface_tint_color().value_vec4.y));
    ufbxt_assert!(19 == round100(material.pbr.subsurface_tint_color().value_vec4.z));
    ufbxt_assert!(20 == round100(material.pbr.subsurface_tint_color().value_vec4.w));
    ufbxt_assert!(21 == round100(material.pbr.subsurface_color().value_vec4.x));
    ufbxt_assert!(22 == round100(material.pbr.subsurface_color().value_vec4.y));
    ufbxt_assert!(23 == round100(material.pbr.subsurface_color().value_vec4.z));
    ufbxt_assert!(24 == round100(material.pbr.subsurface_color().value_vec4.w));
    ufbxt_assert!(10 == round100(material.pbr.subsurface_radius().value_real())); // ??? Unit conversion probably
    ufbxt_assert!(26 == round100(material.pbr.subsurface_scale().value_real()));
    ufbxt_assert!(27 == round100(material.pbr.emission_factor().value_real()));
    ufbxt_assert!(28 == round100(material.pbr.emission_color().value_vec4.x));
    ufbxt_assert!(29 == round100(material.pbr.emission_color().value_vec4.y));
    ufbxt_assert!(30 == round100(material.pbr.emission_color().value_vec4.z));
    ufbxt_assert!(31 == round100(material.pbr.emission_color().value_vec4.w));
    ufbxt_assert!(32 == round100(material.pbr.specular_anisotropy().value_real()));
    ufbxt_assert!(33 == round100(material.pbr.specular_rotation().value_real()));
    ufbxt_assert!(34 == round100(material.pbr.normal_map().value_real()));
    ufbxt_assert!(35 == round100(material.pbr.coat_normal().value_real()));
    ufbxt_assert!(36 == round100(material.pbr.displacement_map().value_real()));
    ufbxt_assert!(37 == round100(material.pbr.diffuse_roughness().value_real()));
    ufbxt_assert!(38 == round100(material.pbr.coat_factor().value_real()));
    ufbxt_assert!(39 == round100(material.pbr.coat_color().value_vec4.x));
    ufbxt_assert!(40 == round100(material.pbr.coat_color().value_vec4.y));
    ufbxt_assert!(41 == round100(material.pbr.coat_color().value_vec4.z));
    ufbxt_assert!(42 == round100(material.pbr.coat_color().value_vec4.w));
    ufbxt_assert!(43 == round100(material.pbr.coat_roughness().value_real()));
    ufbxt_assert!(44 == round100(material.pbr.coat_ior().value_real()));
    ufbxt_assert!(45 == round100(material.pbr.coat_affect_base_color().value_real()));
    ufbxt_assert!(46 == round100(material.pbr.coat_affect_base_roughness().value_real()));

    ufbxt_assert!(material.pbr.base_factor().texture_enabled);
    ufbxt_assert!(material.pbr.base_color().texture_enabled);
    ufbxt_assert!(material.pbr.specular_factor().texture_enabled);
    ufbxt_assert!(material.pbr.specular_color().texture_enabled);
    ufbxt_assert!(material.pbr.roughness().texture_enabled);
    ufbxt_assert!(material.pbr.metalness().texture_enabled);
    ufbxt_assert!(material.pbr.diffuse_roughness().texture_enabled);
    ufbxt_assert!(material.pbr.specular_anisotropy().texture_enabled);
    ufbxt_assert!(material.pbr.specular_rotation().texture_enabled);
    ufbxt_assert!(material.pbr.transmission_factor().texture_enabled);
    ufbxt_assert!(material.pbr.transmission_color().texture_enabled);
    ufbxt_assert!(material.pbr.transmission_roughness().texture_enabled);
    ufbxt_assert!(material.pbr.specular_ior().texture_enabled);
    ufbxt_assert!(material.pbr.subsurface_factor().texture_enabled);
    ufbxt_assert!(material.pbr.subsurface_tint_color().texture_enabled);
    ufbxt_assert!(material.pbr.subsurface_scale().texture_enabled);
    ufbxt_assert!(material.pbr.emission_factor().texture_enabled);
    ufbxt_assert!(material.pbr.emission_color().texture_enabled);
    ufbxt_assert!(material.pbr.coat_factor().texture_enabled);
    ufbxt_assert!(material.pbr.coat_color().texture_enabled);
    ufbxt_assert!(material.pbr.coat_roughness().texture_enabled);
    ufbxt_assert!(material.pbr.normal_map().texture_enabled);
    ufbxt_assert!(material.pbr.coat_normal().texture_enabled);
    ufbxt_assert!(material.pbr.displacement_map().texture_enabled);
    ufbxt_assert!(material.pbr.opacity().texture_enabled);

    ufbxt_assert!(material.pbr.base_factor().value_components == 1);
    ufbxt_assert!(material.pbr.base_color().value_components == 4);
    ufbxt_assert!(material.pbr.specular_factor().value_components == 1);
    ufbxt_assert!(material.pbr.specular_color().value_components == 4);
    ufbxt_assert!(material.pbr.roughness().value_components == 1);
    ufbxt_assert!(material.pbr.metalness().value_components == 1);
    ufbxt_assert!(material.pbr.diffuse_roughness().value_components == 1);
    ufbxt_assert!(material.pbr.specular_anisotropy().value_components == 1);
    ufbxt_assert!(material.pbr.specular_rotation().value_components == 1);
    ufbxt_assert!(material.pbr.transmission_factor().value_components == 1);
    ufbxt_assert!(material.pbr.transmission_color().value_components == 4);
    ufbxt_assert!(material.pbr.transmission_roughness().value_components == 1);
    ufbxt_assert!(material.pbr.transmission_extra_roughness().value_components == 0);
    ufbxt_assert!(material.pbr.specular_ior().value_components == 1);
    ufbxt_assert!(material.pbr.subsurface_factor().value_components == 1);
    ufbxt_assert!(material.pbr.subsurface_tint_color().value_components == 4);
    ufbxt_assert!(material.pbr.subsurface_color().value_components == 4);
    ufbxt_assert!(material.pbr.subsurface_radius().value_components == 1);
    ufbxt_assert!(material.pbr.subsurface_scale().value_components == 1);
    ufbxt_assert!(material.pbr.emission_factor().value_components == 1);
    ufbxt_assert!(material.pbr.emission_color().value_components == 4);
    ufbxt_assert!(material.pbr.coat_factor().value_components == 1);
    ufbxt_assert!(material.pbr.coat_color().value_components == 4);
    ufbxt_assert!(material.pbr.coat_roughness().value_components == 1);
    ufbxt_assert!(material.pbr.normal_map().value_components == 1);
    ufbxt_assert!(material.pbr.coat_normal().value_components == 1);
    ufbxt_assert!(material.pbr.displacement_map().value_components == 1);
    ufbxt_assert!(material.pbr.opacity().value_components == 0);
});

file_test!(max_physical_material_inverted, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "InvertedMaterial").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::Max3dsPhysicalMaterial);

    assert_close_real(err, material.pbr.roughness().value_real(), 0.9);
    assert_close_real(err, material.pbr.transmission_roughness().value_real(), 0.8);
    assert_close_real(err, material.pbr.coat_roughness().value_real(), 0.7);
});

file_test!(max_physical_material_textures, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "PhysicalMaterial").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::Max3dsPhysicalMaterial);
    check_material_texture(scene, material.pbr.base_factor().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", true);
    check_material_texture(scene, material.pbr.specular_factor().texture, "checkerboard_reflection.png", true);
    check_material_texture(scene, material.pbr.specular_color().texture, "checkerboard_reflection.png", true);
    check_material_texture(scene, material.pbr.roughness().texture, "checkerboard_roughness.png", true);
    check_material_texture(scene, material.pbr.metalness().texture, "checkerboard_metallic.png", true);
    check_material_texture(scene, material.pbr.diffuse_roughness().texture, "checkerboard_roughness.png", true);
    check_material_texture(scene, material.pbr.specular_anisotropy().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.specular_rotation().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.transmission_factor().texture, "checkerboard_transparency.png", true);
    check_material_texture(scene, material.pbr.transmission_color().texture, "checkerboard_transparency.png", true);
    check_material_texture(scene, material.pbr.transmission_roughness().texture, "checkerboard_roughness.png", true);
    check_material_texture(scene, material.pbr.specular_ior().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.subsurface_factor().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.subsurface_tint_color().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.subsurface_scale().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.emission_factor().texture, "checkerboard_emissive.png", true);
    check_material_texture(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png", true);
    check_material_texture(scene, material.pbr.coat_factor().texture, "checkerboard_specular.png", true);
    check_material_texture(scene, material.pbr.coat_color().texture, "checkerboard_specular.png", true);
    check_material_texture(scene, material.pbr.coat_roughness().texture, "checkerboard_specular.png", true);
    check_material_texture(scene, material.pbr.normal_map().texture, "checkerboard_bump.png", true);
    check_material_texture(scene, material.pbr.coat_normal().texture, "checkerboard_bump.png", true);
    check_material_texture(scene, material.pbr.displacement_map().texture, "checkerboard_displacement.png", true);
    check_material_texture(scene, material.pbr.opacity().texture, "checkerboard_transparency.png", true);
});

file_test!(max_pbr_metal_rough_material, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "01 - Default").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::Max3dsPbrMetalRough);

    ufbxt_assert!(1 == round100(material.pbr.base_color().value_vec4.x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec4.y));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec4.z));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec4.w));
    ufbxt_assert!(5 == round100(material.pbr.metalness().value_real()));
    ufbxt_assert!(6 == round100(material.pbr.roughness().value_real()));
    ufbxt_assert!(8 == round100(material.pbr.emission_color().value_vec4.x));
    ufbxt_assert!(9 == round100(material.pbr.emission_color().value_vec4.y));
    ufbxt_assert!(10 == round100(material.pbr.emission_color().value_vec4.z));
    ufbxt_assert!(11 == round100(material.pbr.emission_color().value_vec4.w));
    ufbxt_assert!(12 == round100(material.pbr.displacement_map().value_real()));

    assert_close_real(err, material.pbr.base_factor().value_real(), 1.0);
    assert_close_real(err, material.pbr.emission_factor().value_real(), 1.0);

    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", false);
    check_material_texture(scene, material.pbr.metalness().texture, "checkerboard_metallic.png", false);
    check_material_texture(scene, material.pbr.roughness().texture, "checkerboard_roughness.png", false);
    check_material_texture(scene, material.pbr.ambient_occlusion().texture, "checkerboard_ambient.png", false);
    check_material_texture(scene, material.pbr.normal_map().texture, "checkerboard_normal.png", false);
    check_material_texture(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png", false);
    check_material_texture(scene, material.pbr.displacement_map().texture, "checkerboard_displacement.png", false);
    check_material_texture(scene, material.pbr.opacity().texture, "checkerboard_transparency.png", false);

    ufbxt_assert!(material.features[MaterialFeature::Metalness].enabled);
    ufbxt_assert!(!material.features[MaterialFeature::Specular].enabled);
});

file_test!(max_pbr_metal_gloss_material, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "01 - Default").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::Max3dsPbrMetalRough);

    ufbxt_assert!(1 == round100(material.pbr.base_color().value_vec4.x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec4.y));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec4.z));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec4.w));
    ufbxt_assert!(5 == round100(material.pbr.metalness().value_real()));
    ufbxt_assert!(6 == round100(material.pbr.glossiness().value_real()));
    ufbxt_assert!(8 == round100(material.pbr.emission_color().value_vec4.x));
    ufbxt_assert!(9 == round100(material.pbr.emission_color().value_vec4.y));
    ufbxt_assert!(10 == round100(material.pbr.emission_color().value_vec4.z));
    ufbxt_assert!(11 == round100(material.pbr.emission_color().value_vec4.w));
    ufbxt_assert!(12 == round100(material.pbr.displacement_map().value_real()));

    ufbxt_assert!(100 - 6 == round100(material.pbr.roughness().value_real()));

    assert_close_real(err, material.pbr.base_factor().value_real(), 1.0);
    assert_close_real(err, material.pbr.emission_factor().value_real(), 1.0);

    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", false);
    check_material_texture(scene, material.pbr.metalness().texture, "checkerboard_metallic.png", false);
    check_material_texture(scene, material.pbr.glossiness().texture, "checkerboard_roughness.png", false);
    check_material_texture(scene, material.pbr.ambient_occlusion().texture, "checkerboard_ambient.png", false);
    check_material_texture(scene, material.pbr.normal_map().texture, "checkerboard_normal.png", false);
    check_material_texture(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png", false);
    check_material_texture(scene, material.pbr.displacement_map().texture, "checkerboard_displacement.png", false);
    check_material_texture(scene, material.pbr.opacity().texture, "checkerboard_transparency.png", false);

    ufbxt_assert!(material.pbr.roughness().texture.is_none());

    ufbxt_assert!(material.features[MaterialFeature::Metalness].enabled);
    ufbxt_assert!(!material.features[MaterialFeature::Specular].enabled);
});

file_test!(max_pbr_spec_rough_material, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "02 - Default").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::Max3dsPbrSpecGloss);

    ufbxt_assert!(1 == round100(material.pbr.base_color().value_vec4.x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec4.y));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec4.z));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec4.w));
    ufbxt_assert!(5 == round100(material.pbr.specular_color().value_vec4.x));
    ufbxt_assert!(6 == round100(material.pbr.specular_color().value_vec4.y));
    ufbxt_assert!(7 == round100(material.pbr.specular_color().value_vec4.z));
    ufbxt_assert!(8 == round100(material.pbr.specular_color().value_vec4.w));
    ufbxt_assert!(9 == round100(material.pbr.roughness().value_real()));
    ufbxt_assert!(12 == round100(material.pbr.emission_color().value_vec4.x));
    ufbxt_assert!(13 == round100(material.pbr.emission_color().value_vec4.y));
    ufbxt_assert!(14 == round100(material.pbr.emission_color().value_vec4.z));
    ufbxt_assert!(15 == round100(material.pbr.emission_color().value_vec4.w));
    ufbxt_assert!(16 == round100(material.pbr.displacement_map().value_real()));

    assert_close_real(err, material.pbr.base_factor().value_real(), 1.0);
    assert_close_real(err, material.pbr.emission_factor().value_real(), 1.0);

    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", false);
    check_material_texture(scene, material.pbr.specular_color().texture, "checkerboard_specular.png", false);
    check_material_texture(scene, material.pbr.roughness().texture, "checkerboard_weight.png", false);
    check_material_texture(scene, material.pbr.ambient_occlusion().texture, "checkerboard_ambient.png", false);
    check_material_texture(scene, material.pbr.normal_map().texture, "checkerboard_normal.png", false);
    check_material_texture(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png", false);
    check_material_texture(scene, material.pbr.displacement_map().texture, "checkerboard_displacement.png", false);
    check_material_texture(scene, material.pbr.opacity().texture, "checkerboard_transparency.png", false);

    ufbxt_assert!(!material.features[MaterialFeature::Metalness].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Specular].enabled);
});

file_test!(max_pbr_spec_gloss_material, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "02 - Default").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::Max3dsPbrSpecGloss);

    ufbxt_assert!(1 == round100(material.pbr.base_color().value_vec4.x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec4.y));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec4.z));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec4.w));
    ufbxt_assert!(5 == round100(material.pbr.specular_color().value_vec4.x));
    ufbxt_assert!(6 == round100(material.pbr.specular_color().value_vec4.y));
    ufbxt_assert!(7 == round100(material.pbr.specular_color().value_vec4.z));
    ufbxt_assert!(8 == round100(material.pbr.specular_color().value_vec4.w));
    ufbxt_assert!(9 == round100(material.pbr.glossiness().value_real()));
    ufbxt_assert!(12 == round100(material.pbr.emission_color().value_vec4.x));
    ufbxt_assert!(13 == round100(material.pbr.emission_color().value_vec4.y));
    ufbxt_assert!(14 == round100(material.pbr.emission_color().value_vec4.z));
    ufbxt_assert!(15 == round100(material.pbr.emission_color().value_vec4.w));
    ufbxt_assert!(16 == round100(material.pbr.displacement_map().value_real()));

    ufbxt_assert!(100 - 9 == round100(material.pbr.roughness().value_real()));

    assert_close_real(err, material.pbr.base_factor().value_real(), 1.0);
    assert_close_real(err, material.pbr.emission_factor().value_real(), 1.0);

    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", false);
    check_material_texture(scene, material.pbr.specular_color().texture, "checkerboard_specular.png", false);
    check_material_texture(scene, material.pbr.glossiness().texture, "checkerboard_weight.png", false);
    check_material_texture(scene, material.pbr.ambient_occlusion().texture, "checkerboard_ambient.png", false);
    check_material_texture(scene, material.pbr.normal_map().texture, "checkerboard_normal.png", false);
    check_material_texture(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png", false);
    check_material_texture(scene, material.pbr.displacement_map().texture, "checkerboard_displacement.png", false);
    check_material_texture(scene, material.pbr.opacity().texture, "checkerboard_transparency.png", false);

    ufbxt_assert!(material.pbr.roughness().texture.is_none());

    ufbxt_assert!(!material.features[MaterialFeature::Metalness].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Specular].enabled);
});

file_test!(max_gltf_material, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "01 - Default").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::GltfMaterial);

    ufbxt_assert!(1 == round100(material.pbr.base_color().value_vec4.x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec4.y));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec4.z));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec4.w));
    ufbxt_assert!(5 == round100(material.pbr.metalness().value_real()));
    ufbxt_assert!(6 == round100(material.pbr.roughness().value_real()));
    ufbxt_assert!(7 == round100(material.pbr.normal_map().value_real()));
    ufbxt_assert!(8 == round100(material.pbr.ambient_occlusion().value_real()));
    ufbxt_assert!(9 == round100(material.pbr.emission_color().value_vec4.x));
    ufbxt_assert!(10 == round100(material.pbr.emission_color().value_vec4.y));
    ufbxt_assert!(11 == round100(material.pbr.emission_color().value_vec4.z));
    ufbxt_assert!(12 == round100(material.pbr.emission_color().value_vec4.w));
    ufbxt_assert!(13 == round100(material.pbr.coat_factor().value_real()));
    ufbxt_assert!(14 == round100(material.pbr.coat_roughness().value_real()));
    ufbxt_assert!(15 == round100(material.pbr.coat_normal().value_real()));
    ufbxt_assert!(16 == round100(material.pbr.sheen_color().value_vec4.x));
    ufbxt_assert!(17 == round100(material.pbr.sheen_color().value_vec4.y));
    ufbxt_assert!(18 == round100(material.pbr.sheen_color().value_vec4.z));
    ufbxt_assert!(19 == round100(material.pbr.sheen_color().value_vec4.w));
    ufbxt_assert!(20 == round100(material.pbr.sheen_roughness().value_real()));
    ufbxt_assert!(21 == round100(material.pbr.specular_factor().value_real()));
    ufbxt_assert!(22 == round100(material.pbr.specular_color().value_vec4.x));
    ufbxt_assert!(23 == round100(material.pbr.specular_color().value_vec4.y));
    ufbxt_assert!(24 == round100(material.pbr.specular_color().value_vec4.z));
    ufbxt_assert!(25 == round100(material.pbr.specular_color().value_vec4.w));
    ufbxt_assert!(26 == round100(material.pbr.transmission_factor().value_real()));
    ufbxt_assert!(133 == round100(material.pbr.specular_ior().value_real()));

    assert_close_real(err, material.pbr.base_factor().value_real(), 1.0);
    assert_close_real(err, material.pbr.emission_factor().value_real(), 1.0);

    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", true);
    check_material_texture(scene, material.pbr.opacity().texture, "checkerboard_transparency.png", true);
    check_material_texture(scene, material.pbr.metalness().texture, "checkerboard_metallic.png", true);
    check_material_texture(scene, material.pbr.roughness().texture, "checkerboard_roughness.png", true);
    check_material_texture(scene, material.pbr.normal_map().texture, "checkerboard_normal.png", true);
    check_material_texture(scene, material.pbr.ambient_occlusion().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png", true);
    check_material_texture(scene, material.pbr.coat_factor().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.coat_roughness().texture, "checkerboard_roughness.png", true);
    check_material_texture(scene, material.pbr.coat_normal().texture, "checkerboard_normal.png", true);
    check_material_texture(scene, material.pbr.sheen_color().texture, "checkerboard_ambient.png", true);
    check_material_texture(scene, material.pbr.sheen_roughness().texture, "checkerboard_roughness.png", true);
    check_material_texture(scene, material.pbr.specular_factor().texture, "checkerboard_weight.png", true);
    check_material_texture(scene, material.pbr.specular_color().texture, "checkerboard_specular.png", true);
    check_material_texture(scene, material.pbr.transmission_factor().texture, "checkerboard_transparency.png", true);

    ufbxt_assert!(material.features[MaterialFeature::Metalness].enabled);
    ufbxt_assert!(!material.features[MaterialFeature::Metalness].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::DoubleSided].enabled);
    ufbxt_assert!(material.features[MaterialFeature::DoubleSided].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::Coat].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Coat].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::Sheen].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Sheen].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::Specular].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Specular].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::Transmission].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Transmission].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::Ior].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Ior].is_explicit);
});

file_test!(maya_shaderfx_pbs_material, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "StingrayPBS1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::ShaderfxGraph);

    ufbxt_assert!(1 == round100(material.pbr.base_color().value_vec3().x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec3().y));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec3().z));
    ufbxt_assert!(4 == round100(material.pbr.metalness().value_vec3().x));
    ufbxt_assert!(5 == round100(material.pbr.roughness().value_vec3().x));
    ufbxt_assert!(6 == round100(material.pbr.emission_color().value_vec3().x));
    ufbxt_assert!(7 == round100(material.pbr.emission_color().value_vec3().y));
    ufbxt_assert!(8 == round100(material.pbr.emission_color().value_vec3().z));
    ufbxt_assert!(9 == round100(material.pbr.emission_factor().value_real()));
    assert_close_real(err, material.pbr.base_factor().value_real(), 1.0);

    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", true);
    check_material_texture(scene, material.pbr.normal_map().texture, "checkerboard_normal.png", true);
    check_material_texture(scene, material.pbr.metalness().texture, "checkerboard_metallic.png", true);
    check_material_texture(scene, material.pbr.roughness().texture, "checkerboard_roughness.png", true);
    check_material_texture(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png", true);
    check_material_texture(scene, material.pbr.ambient_occlusion().texture, "checkerboard_weight.png", true);

    ufbxt_assert!(material.pbr.base_color().texture_enabled == true);
    ufbxt_assert!(material.pbr.normal_map().texture_enabled == false);
    ufbxt_assert!(material.pbr.metalness().texture_enabled == true);
    ufbxt_assert!(material.pbr.roughness().texture_enabled == false);
    ufbxt_assert!(material.pbr.emission_color().texture_enabled == true);
    ufbxt_assert!(material.pbr.ambient_occlusion().texture_enabled == false);

    let shader = material.shader.map(|s| &scene.shaders[s]);
    ufbxt_assert!(shader.is_some());
    let shader = shader.unwrap();

    if scene.metadata.version >= 7000 {
        let blob = shader.element.props.find_blob("ShaderGraph", &[]);
        ufbxt_assert!(blob.len() == 32918);
        let hash_ref = fnv1a(blob);
        ufbxt_assert!(hash_ref == 0x7be121c5);
    }
});

file_test!(max_openpbr_material, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "OpenPBR").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::OpenpbrMaterial);

    ufbxt_assert!(1 == round100(material.pbr.base_factor().value_real()));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec4.x));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec4.y));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec4.z));
    ufbxt_assert!(5 == round100(material.pbr.base_color().value_vec4.w));
    ufbxt_assert!(6 == round100(material.pbr.metalness().value_real()));
    ufbxt_assert!(7 == round100(material.pbr.diffuse_roughness().value_real()));
    ufbxt_assert!(8 == round100(material.pbr.specular_factor().value_real()));
    ufbxt_assert!(9 == round100(material.pbr.specular_color().value_vec4.x));
    ufbxt_assert!(10 == round100(material.pbr.specular_color().value_vec4.y));
    ufbxt_assert!(11 == round100(material.pbr.specular_color().value_vec4.z));
    ufbxt_assert!(12 == round100(material.pbr.specular_color().value_vec4.w));
    ufbxt_assert!(14 == round100(material.pbr.roughness().value_real()));
    ufbxt_assert!(15 == round100(material.pbr.specular_anisotropy().value_real()));
    ufbxt_assert!(16 == round100(material.pbr.transmission_factor().value_real()));
    ufbxt_assert!(17 == round100(material.pbr.transmission_color().value_vec4.x));
    ufbxt_assert!(18 == round100(material.pbr.transmission_color().value_vec4.y));
    ufbxt_assert!(19 == round100(material.pbr.transmission_color().value_vec4.z));
    ufbxt_assert!(20 == round100(material.pbr.transmission_color().value_vec4.w));
    ufbxt_assert!(21 == round100(material.pbr.transmission_depth().value_real()));
    ufbxt_assert!(22 == round100(material.pbr.transmission_scatter().value_vec4.x));
    ufbxt_assert!(23 == round100(material.pbr.transmission_scatter().value_vec4.y));
    ufbxt_assert!(24 == round100(material.pbr.transmission_scatter().value_vec4.z));
    ufbxt_assert!(25 == round100(material.pbr.transmission_scatter().value_vec4.w));
    ufbxt_assert!(26 == round100(material.pbr.transmission_scatter_anisotropy().value_real()));
    ufbxt_assert!(27 == round100(material.pbr.transmission_dispersion().value_real()));
    ufbxt_assert!(29 == round100(material.pbr.subsurface_factor().value_real()));
    ufbxt_assert!(30 == round100(material.pbr.subsurface_color().value_vec4.x));
    ufbxt_assert!(31 == round100(material.pbr.subsurface_color().value_vec4.y));
    ufbxt_assert!(32 == round100(material.pbr.subsurface_color().value_vec4.z));
    ufbxt_assert!(33 == round100(material.pbr.subsurface_color().value_vec4.w));
    ufbxt_assert!(34 == round100(material.pbr.subsurface_anisotropy().value_real()));
    ufbxt_assert!(35 == round100(material.pbr.subsurface_radius().value_vec4.x));
    ufbxt_assert!(36 == round100(material.pbr.subsurface_radius().value_vec4.y));
    ufbxt_assert!(37 == round100(material.pbr.subsurface_radius().value_vec4.z));
    ufbxt_assert!(38 == round100(material.pbr.subsurface_radius().value_vec4.w));
    ufbxt_assert!(39 == round100(material.pbr.subsurface_scale().value_real()));
    ufbxt_assert!(40 == round100(material.pbr.coat_factor().value_real()));
    ufbxt_assert!(41 == round100(material.pbr.coat_color().value_vec4.x));
    ufbxt_assert!(42 == round100(material.pbr.coat_color().value_vec4.y));
    ufbxt_assert!(43 == round100(material.pbr.coat_color().value_vec4.z));
    ufbxt_assert!(44 == round100(material.pbr.coat_color().value_vec4.w));
    ufbxt_assert!(46 == round100(material.pbr.coat_roughness().value_real()));
    ufbxt_assert!(47 == round100(material.pbr.coat_anisotropy().value_real()));
    ufbxt_assert!(49 == round100(material.pbr.sheen_factor().value_real()));
    ufbxt_assert!(50 == round100(material.pbr.sheen_color().value_vec4.x));
    ufbxt_assert!(51 == round100(material.pbr.sheen_color().value_vec4.y));
    ufbxt_assert!(52 == round100(material.pbr.sheen_color().value_vec4.z));
    ufbxt_assert!(53 == round100(material.pbr.sheen_color().value_vec4.w));
    ufbxt_assert!(54 == round100(material.pbr.sheen_roughness().value_real()));
    ufbxt_assert!(56 == round100(material.pbr.emission_color().value_vec4.x));
    ufbxt_assert!(57 == round100(material.pbr.emission_color().value_vec4.y));
    ufbxt_assert!(58 == round100(material.pbr.emission_color().value_vec4.z));
    ufbxt_assert!(59 == round100(material.pbr.emission_color().value_vec4.w));
    ufbxt_assert!(61 == round100(material.pbr.thin_film_factor().value_real()));
    ufbxt_assert!(62 == round100(material.pbr.thin_film_thickness().value_real()));
    ufbxt_assert!(64 == round100(material.pbr.normal_map().value_real()));
    ufbxt_assert!(65 == round100(material.pbr.coat_normal().value_real()));
    ufbxt_assert!(66 == round100(material.pbr.displacement_map().value_real()));

    ufbxt_assert!(130 == round100(material.pbr.specular_ior().value_real()));
    ufbxt_assert!(450 == round100(material.pbr.coat_ior().value_real()));
    ufbxt_assert!(630 == round100(material.pbr.thin_film_ior().value_real()));

    assert_close_real(err, material.pbr.emission_factor().value_real(), 0.55 * 6000.0);

    check_material_texture(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png", false);
    check_material_texture(scene, material.pbr.metalness().texture, "checkerboard_metallic.png", false);
    check_material_texture(scene, material.pbr.specular_color().texture, "checkerboard_specular.png", false);
    check_material_texture(scene, material.pbr.roughness().texture, "checkerboard_roughness.png", false);
    check_material_texture(scene, material.pbr.transmission_color().texture, "checkerboard_transparency.png", false);
    check_material_texture(scene, material.pbr.transmission_scatter().texture, "checkerboard_reflection.png", false);
    check_material_texture(scene, material.pbr.subsurface_color().texture, "checkerboard_ambient.png", false);
    check_material_texture(scene, material.pbr.coat_color().texture, "checkerboard_weight.png", false);
    check_material_texture(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png", false);
    check_material_texture(scene, material.pbr.emission_factor().texture, "tiny_clouds.png", false);
    check_material_texture(scene, material.pbr.normal_map().texture, "checkerboard_normal.png", false);
    check_material_texture(scene, material.pbr.displacement_map().texture, "checkerboard_vector_displacement.png", false);

    ufbxt_assert!(material.pbr.base_factor().texture_enabled);
    ufbxt_assert!(material.pbr.base_color().texture_enabled);
    ufbxt_assert!(material.pbr.specular_factor().texture_enabled);
    ufbxt_assert!(material.pbr.specular_color().texture_enabled);
    ufbxt_assert!(material.pbr.roughness().texture_enabled);
    ufbxt_assert!(material.pbr.metalness().texture_enabled);
    ufbxt_assert!(material.pbr.diffuse_roughness().texture_enabled);
    ufbxt_assert!(material.pbr.specular_anisotropy().texture_enabled);
    ufbxt_assert!(material.pbr.transmission_factor().texture_enabled);
    ufbxt_assert!(material.pbr.transmission_color().texture_enabled);
    ufbxt_assert!(material.pbr.specular_ior().texture_enabled);
    ufbxt_assert!(material.pbr.subsurface_factor().texture_enabled);
    ufbxt_assert!(material.pbr.subsurface_scale().texture_enabled);
    ufbxt_assert!(material.pbr.emission_factor().texture_enabled);
    ufbxt_assert!(material.pbr.emission_color().texture_enabled);
    ufbxt_assert!(material.pbr.coat_factor().texture_enabled);
    ufbxt_assert!(material.pbr.coat_color().texture_enabled);
    ufbxt_assert!(material.pbr.coat_roughness().texture_enabled);
    ufbxt_assert!(material.pbr.normal_map().texture_enabled);
    ufbxt_assert!(material.pbr.coat_normal().texture_enabled);
    ufbxt_assert!(material.pbr.displacement_map().texture_enabled);
    ufbxt_assert!(material.pbr.opacity().texture_enabled);

    ufbxt_assert!(material.pbr.base_factor().value_components == 1);
    ufbxt_assert!(material.pbr.base_color().value_components == 4);
    ufbxt_assert!(material.pbr.specular_factor().value_components == 1);
    ufbxt_assert!(material.pbr.specular_color().value_components == 4);
    ufbxt_assert!(material.pbr.roughness().value_components == 1);
    ufbxt_assert!(material.pbr.metalness().value_components == 1);
    ufbxt_assert!(material.pbr.diffuse_roughness().value_components == 1);
    ufbxt_assert!(material.pbr.specular_anisotropy().value_components == 1);
    ufbxt_assert!(material.pbr.transmission_factor().value_components == 1);
    ufbxt_assert!(material.pbr.transmission_color().value_components == 4);
    ufbxt_assert!(material.pbr.transmission_extra_roughness().value_components == 0);
    ufbxt_assert!(material.pbr.specular_ior().value_components == 1);
    ufbxt_assert!(material.pbr.subsurface_factor().value_components == 1);
    ufbxt_assert!(material.pbr.subsurface_color().value_components == 4);
    ufbxt_assert!(material.pbr.subsurface_radius().value_components == 4);
    ufbxt_assert!(material.pbr.subsurface_scale().value_components == 1);
    ufbxt_assert!(material.pbr.emission_factor().value_components == 1);
    ufbxt_assert!(material.pbr.emission_color().value_components == 4);
    ufbxt_assert!(material.pbr.coat_factor().value_components == 1);
    ufbxt_assert!(material.pbr.coat_color().value_components == 4);
    ufbxt_assert!(material.pbr.coat_roughness().value_components == 1);
    ufbxt_assert!(material.pbr.normal_map().value_components == 1);
    ufbxt_assert!(material.pbr.coat_normal().value_components == 1);
    ufbxt_assert!(material.pbr.displacement_map().value_components == 1);
    ufbxt_assert!(material.pbr.opacity().value_components == 0);

    ufbxt_assert!(material.features[MaterialFeature::Diffuse].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Metalness].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Specular].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Coat].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Sheen].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Transmission].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Opacity].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Ior].enabled);
    ufbxt_assert!(material.features[MaterialFeature::DiffuseRoughness].enabled);
    ufbxt_assert!(material.features[MaterialFeature::ThinWalled].enabled);
    ufbxt_assert!(material.features[MaterialFeature::ThinWalled].is_explicit);
});

file_test!(blender_279_internal_textures, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "Material.001").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 5);

    ufbxt_assert!(material.shader_type == ShaderType::FbxPhong);
    ufbxt_assert!(scene.textures[material.fbx.diffuse_color().texture.unwrap()].relative_filename == "textures\\checkerboard_diffuse.png");
    ufbxt_assert!(scene.textures[material.fbx.specular_color().texture.unwrap()].relative_filename == "textures\\checkerboard_specular.png");
    ufbxt_assert!(scene.textures[material.fbx.specular_factor().texture.unwrap()].relative_filename == "textures\\checkerboard_weight.png");
    ufbxt_assert!(scene.textures[material.fbx.ambient_factor().texture.unwrap()].relative_filename == "textures\\checkerboard_ambient.png");
    ufbxt_assert!(scene.textures[material.fbx.emission_factor().texture.unwrap()].relative_filename == "textures\\checkerboard_emissive.png");
});

fn blender_pbr_load_opts() -> LoadOpts {
    LoadOpts { use_blender_pbr_material: true, ..Default::default() }
}

file_test!(blender_293_textures, opts = blender_pbr_load_opts(), flags = FUZZ_OPTS, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "Material.001").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 5);

    ufbxt_assert!(material.shader_type == ShaderType::BlenderPhong);
    ufbxt_assert!(scene.textures[material.pbr.base_color().texture.unwrap()].relative_filename == "textures\\checkerboard_diffuse.png");
    ufbxt_assert!(scene.textures[material.pbr.roughness().texture.unwrap()].relative_filename == "textures\\checkerboard_roughness.png");
    ufbxt_assert!(scene.textures[material.pbr.metalness().texture.unwrap()].relative_filename == "textures\\checkerboard_metallic.png");
    ufbxt_assert!(scene.textures[material.pbr.emission_color().texture.unwrap()].relative_filename == "textures\\checkerboard_emissive.png");
    ufbxt_assert!(scene.textures[material.pbr.opacity().texture.unwrap()].relative_filename == "textures\\checkerboard_weight.png");
});

file_test!(blender_293_textures_default, file = "blender_293_textures", flags = FUZZ_ALWAYS, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "Material.001").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 5);

    ufbxt_assert!(material.shader_type == ShaderType::FbxPhong);
    ufbxt_assert!(scene.textures[material.pbr.base_color().texture.unwrap()].relative_filename == "textures\\checkerboard_diffuse.png");
    ufbxt_assert!(scene.textures[material.pbr.roughness().texture.unwrap()].relative_filename == "textures\\checkerboard_roughness.png");
    ufbxt_assert!(material.pbr.metalness().texture.is_none());
    ufbxt_assert!(scene.textures[material.pbr.emission_color().texture.unwrap()].relative_filename == "textures\\checkerboard_emissive.png");
    ufbxt_assert!(material.pbr.opacity().texture.is_none());
});

file_test!(blender_293_embedded_textures, opts = blender_pbr_load_opts(), flags = FUZZ_OPTS, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "Material.001").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 5);

    ufbxt_assert!(material.shader_type == ShaderType::BlenderPhong);
    check_texture_content(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png");
    check_texture_content(scene, material.pbr.roughness().texture, "checkerboard_roughness.png");
    check_texture_content(scene, material.pbr.metalness().texture, "checkerboard_metallic.png");
    check_texture_content(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png");
    check_texture_content(scene, material.pbr.opacity().texture, "checkerboard_weight.png");
});

file_test!(blender_293_embedded_textures_default, file = "blender_293_embedded_textures", flags = FUZZ_ALWAYS, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "Material.001").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.textures.len() == 5);

    ufbxt_assert!(material.shader_type == ShaderType::FbxPhong);
    check_texture_content(scene, material.pbr.base_color().texture, "checkerboard_diffuse.png");
    check_texture_content(scene, material.pbr.roughness().texture, "checkerboard_roughness.png");
    ufbxt_assert!(material.pbr.metalness().texture.is_none());
    check_texture_content(scene, find_prop_texture(material, "ReflectionFactor"), "checkerboard_metallic.png");
    check_texture_content(scene, material.pbr.emission_color().texture, "checkerboard_emissive.png");
    ufbxt_assert!(material.pbr.opacity().texture.is_none());
    check_texture_content(scene, find_prop_texture(material, "TransparencyFactor"), "checkerboard_weight.png");
});

file_test!(blender_293_material_mapping, opts = blender_pbr_load_opts(), flags = FUZZ_OPTS, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "Material.001").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::BlenderPhong);
    assert_close_real(err, material.fbx.specular_exponent().value_vec3().x, 76.913);
    assert_close_real(err, material.fbx.transparency_factor().value_vec3().x, 0.544);
    assert_close_real(err, material.pbr.opacity().value_vec3().x, 0.456);
    assert_close_real(err, material.pbr.roughness().value_vec3().x, 0.123);
});

file_test!(blender_293_material_mapping_default, file = "blender_293_material_mapping", flags = FUZZ_ALWAYS, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "Material.001").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::FbxPhong);
    assert_close_real(err, material.fbx.specular_exponent().value_vec3().x, 76.913);
    assert_close_real(err, material.fbx.transparency_factor().value_vec3().x, 0.544);
    ufbxt_assert!(!material.pbr.opacity().has_value);
    assert_close_real(err, material.pbr.roughness().value_vec3().x, 0.123);
});

file_test!(maya_different_shaders, |scene, err, load_error| {
    let lambert1 = find_element(scene, ElementType::Material, "lambert1").and_then(|id| scene.as_material(id));
    let phong1 = find_element(scene, ElementType::Material, "phong1").and_then(|id| scene.as_material(id));
    let arnold = find_element(scene, ElementType::Material, "aiStandardSurface1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(lambert1.is_some() && phong1.is_some() && arnold.is_some());
    let (lambert1, phong1, arnold) = (lambert1.unwrap(), phong1.unwrap(), arnold.unwrap());

    let r = Vec3::new(1.0, 0.0, 0.0);
    let g = Vec3::new(0.0, 1.0, 0.0);
    let b = Vec3::new(0.0, 0.0, 1.0);

    ufbxt_assert!(lambert1.shader_type == ShaderType::FbxLambert);
    ufbxt_assert!(lambert1.shading_model_name == "lambert");
    assert_close_vec3(err, lambert1.fbx.diffuse_color().value_vec3(), g);
    assert_close_vec3(err, lambert1.pbr.base_color().value_vec3(), g);
    assert_close_real(err, lambert1.pbr.specular_factor().value_vec3().x, 0.0);

    ufbxt_assert!(phong1.shader_type == ShaderType::FbxPhong);
    ufbxt_assert!(phong1.shading_model_name == "phong");
    assert_close_vec3(err, phong1.fbx.diffuse_color().value_vec3(), b);
    assert_close_vec3(err, phong1.pbr.base_color().value_vec3(), b);
    assert_close_real(err, phong1.pbr.specular_factor().value_vec3().x, 1.0);

    ufbxt_assert!(arnold.shader_type == ShaderType::ArnoldStandardSurface);
    ufbxt_assert!(arnold.shading_model_name == "unknown");
    assert_close_vec3(err, arnold.pbr.base_color().value_vec3(), r);
});

#[test]
fn blender_phong_quirks() {
    for quirks in 0..=1 {
        let opts = LoadOpts { disable_quirks: quirks == 0, use_blender_pbr_material: true, ..Default::default() };

        let buf = data_path("blender_293_textures_7400_binary.fbx");

        let scene = load_file(&buf, &opts);
        ufbxt_assert!(scene.is_ok());
        let scene = &scene.unwrap();

        // Exporter should be detected even with quirks off
        ufbxt_assert!(scene.metadata.exporter == Exporter::BlenderBinary);
        ufbxt_assert!(scene.metadata.exporter_version == pack_version(4, 22, 0));

        let material = find_element(scene, ElementType::Material, "Material.001").and_then(|id| scene.as_material(id));
        ufbxt_assert!(material.is_some());
        let material = material.unwrap();
        if quirks != 0 {
            ufbxt_assert!(material.shader_type == ShaderType::BlenderPhong);
        } else {
            ufbxt_assert!(material.shader_type == ShaderType::FbxPhong);
        }
    }
}

file_test!(maya_phong_properties, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "phong1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.shader_type == ShaderType::FbxPhong);

    ufbxt_assert!(1 == round100(material.fbx.diffuse_color().value_vec3().x));
    ufbxt_assert!(2 == round100(material.fbx.diffuse_color().value_vec3().y));
    ufbxt_assert!(3 == round100(material.fbx.diffuse_color().value_vec3().z));
    ufbxt_assert!(4 == round100(material.fbx.transparency_color().value_vec3().x));
    ufbxt_assert!(5 == round100(material.fbx.transparency_color().value_vec3().y));
    ufbxt_assert!(6 == round100(material.fbx.transparency_color().value_vec3().z));
    ufbxt_assert!(7 == round100(material.fbx.ambient_color().value_vec3().x));
    ufbxt_assert!(8 == round100(material.fbx.ambient_color().value_vec3().y));
    ufbxt_assert!(9 == round100(material.fbx.ambient_color().value_vec3().z));
    ufbxt_assert!(10 == round100(material.fbx.emission_color().value_vec3().x));
    ufbxt_assert!(11 == round100(material.fbx.emission_color().value_vec3().y));
    ufbxt_assert!(12 == round100(material.fbx.emission_color().value_vec3().z));
    ufbxt_assert!(13 == round100(material.fbx.diffuse_factor().value_vec3().x));
    ufbxt_assert!(17 == material.fbx.specular_exponent().value_vec3().x.round() as i32);
    ufbxt_assert!(18 == round100(material.fbx.specular_color().value_vec3().x));
    ufbxt_assert!(19 == round100(material.fbx.specular_color().value_vec3().y));
    ufbxt_assert!(20 == round100(material.fbx.specular_color().value_vec3().z));
    ufbxt_assert!(21 == round100(material.fbx.reflection_factor().value_vec3().x));
    ufbxt_assert!(22 == round100(material.fbx.reflection_color().value_vec3().x));
    ufbxt_assert!(23 == round100(material.fbx.reflection_color().value_vec3().y));
    ufbxt_assert!(24 == round100(material.fbx.reflection_color().value_vec3().z));

    ufbxt_assert!(1 == round100(material.pbr.base_color().value_vec3().x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec3().y));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec3().z));
    ufbxt_assert!(13 == round100(material.pbr.base_factor().value_vec3().x));
    assert_close_real(err, material.pbr.metalness().value_real(), 0.0);
    assert_close_real(err, material.pbr.roughness().value_real(), 0.587689);
});

file_test!(maya_arnold_properties, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "aiStandardSurface1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.shader_type == ShaderType::ArnoldStandardSurface);

    ufbxt_assert!(1 == round100(material.pbr.base_factor().value_vec3().x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec3().x));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec3().y));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec3().z));
    ufbxt_assert!(5 == round100(material.pbr.diffuse_roughness().value_vec3().x));
    ufbxt_assert!(6 == round100(material.pbr.metalness().value_vec3().x));
    ufbxt_assert!(7 == round100(material.pbr.specular_factor().value_vec3().x));
    ufbxt_assert!(8 == round100(material.pbr.specular_color().value_vec3().x));
    ufbxt_assert!(9 == round100(material.pbr.specular_color().value_vec3().y));
    ufbxt_assert!(10 == round100(material.pbr.specular_color().value_vec3().z));
    ufbxt_assert!(11 == round100(material.pbr.roughness().value_vec3().x));
    ufbxt_assert!(12 == round100(material.pbr.specular_ior().value_vec3().x));
    ufbxt_assert!(13 == round100(material.pbr.specular_anisotropy().value_vec3().x));
    ufbxt_assert!(14 == round100(material.pbr.specular_rotation().value_vec3().x));
    ufbxt_assert!(15 == round100(material.pbr.transmission_factor().value_vec3().x));
    ufbxt_assert!(16 == round100(material.pbr.transmission_color().value_vec3().x));
    ufbxt_assert!(17 == round100(material.pbr.transmission_color().value_vec3().y));
    ufbxt_assert!(18 == round100(material.pbr.transmission_color().value_vec3().z));
    ufbxt_assert!(19 == round100(material.pbr.transmission_depth().value_vec3().x));
    ufbxt_assert!(20 == round100(material.pbr.transmission_scatter().value_vec3().x));
    ufbxt_assert!(21 == round100(material.pbr.transmission_scatter().value_vec3().y));
    ufbxt_assert!(22 == round100(material.pbr.transmission_scatter().value_vec3().z));
    ufbxt_assert!(23 == round100(material.pbr.transmission_scatter_anisotropy().value_vec3().x));
    ufbxt_assert!(24 == round100(material.pbr.transmission_dispersion().value_vec3().x));
    ufbxt_assert!(25 == round100(material.pbr.transmission_extra_roughness().value_vec3().x));
    ufbxt_assert!(26 == round100(material.pbr.subsurface_factor().value_vec3().x));
    ufbxt_assert!(27 == round100(material.pbr.subsurface_color().value_vec3().x));
    ufbxt_assert!(28 == round100(material.pbr.subsurface_color().value_vec3().y));
    ufbxt_assert!(29 == round100(material.pbr.subsurface_color().value_vec3().z));
    ufbxt_assert!(30 == round100(material.pbr.subsurface_radius().value_vec3().x));
    ufbxt_assert!(31 == round100(material.pbr.subsurface_radius().value_vec3().y));
    ufbxt_assert!(32 == round100(material.pbr.subsurface_radius().value_vec3().z));
    ufbxt_assert!(33 == round100(material.pbr.subsurface_scale().value_vec3().x));
    ufbxt_assert!(34 == round100(material.pbr.subsurface_anisotropy().value_vec3().x));
    ufbxt_assert!(35 == round100(material.pbr.coat_factor().value_vec3().x));
    ufbxt_assert!(36 == round100(material.pbr.coat_color().value_vec3().x));
    ufbxt_assert!(37 == round100(material.pbr.coat_color().value_vec3().y));
    ufbxt_assert!(38 == round100(material.pbr.coat_color().value_vec3().z));
    ufbxt_assert!(39 == round100(material.pbr.coat_roughness().value_vec3().x));
    ufbxt_assert!(40 == round100(material.pbr.coat_ior().value_vec3().x));
    ufbxt_assert!(41 == round100(material.pbr.coat_anisotropy().value_vec3().x));
    ufbxt_assert!(42 == round100(material.pbr.coat_rotation().value_vec3().x));
    ufbxt_assert!(43 == round100(material.pbr.coat_normal().value_vec3().x));
    ufbxt_assert!(44 == round100(material.pbr.coat_normal().value_vec3().y));
    ufbxt_assert!(45 == round100(material.pbr.coat_normal().value_vec3().z));
    ufbxt_assert!(46 == round100(material.pbr.sheen_factor().value_vec3().x));
    ufbxt_assert!(47 == round100(material.pbr.sheen_color().value_vec3().x));
    ufbxt_assert!(48 == round100(material.pbr.sheen_color().value_vec3().y));
    ufbxt_assert!(49 == round100(material.pbr.sheen_color().value_vec3().z));
    ufbxt_assert!(50 == round100(material.pbr.sheen_roughness().value_vec3().x));
    ufbxt_assert!(51 == round100(material.pbr.emission_factor().value_vec3().x));
    ufbxt_assert!(52 == round100(material.pbr.emission_color().value_vec3().x));
    ufbxt_assert!(53 == round100(material.pbr.emission_color().value_vec3().y));
    ufbxt_assert!(54 == round100(material.pbr.emission_color().value_vec3().z));
    ufbxt_assert!(55 == round100(material.pbr.thin_film_thickness().value_vec3().x));
    ufbxt_assert!(56 == round100(material.pbr.thin_film_ior().value_vec3().x));
    ufbxt_assert!(57 == round100(material.pbr.opacity().value_vec3().x));
    ufbxt_assert!(58 == round100(material.pbr.opacity().value_vec3().y));
    ufbxt_assert!(59 == round100(material.pbr.opacity().value_vec3().z));
    ufbxt_assert!(60 == round100(material.pbr.tangent_map().value_vec3().x));
    ufbxt_assert!(61 == round100(material.pbr.tangent_map().value_vec3().y));
    ufbxt_assert!(62 == round100(material.pbr.tangent_map().value_vec3().z));
    ufbxt_assert!(63 == round100(material.pbr.indirect_diffuse().value_vec3().x));
    ufbxt_assert!(64 == round100(material.pbr.indirect_specular().value_vec3().x));
    ufbxt_assert!(65 == round100(material.pbr.matte_color().value_vec3().x));
    ufbxt_assert!(66 == round100(material.pbr.matte_color().value_vec3().y));
    ufbxt_assert!(67 == round100(material.pbr.matte_color().value_vec3().z));
    ufbxt_assert!(68 == round100(material.pbr.matte_factor().value_vec3().x));
    ufbxt_assert!(69 == material.pbr.transmission_priority().value_int);

    // Computed
    assert_close_real(err, material.pbr.transmission_roughness().value_real(), 0.36); // 11+25
    ufbxt_assert!(material.pbr.thin_film_factor().value_real() == 1.0);

    ufbxt_assert!(material.pbr.base_factor().value_components == 1);
    ufbxt_assert!(material.pbr.base_color().value_components == 3);
    ufbxt_assert!(material.pbr.specular_factor().value_components == 1);
    ufbxt_assert!(material.pbr.specular_color().value_components == 3);
    ufbxt_assert!(material.pbr.roughness().value_components == 1);
    ufbxt_assert!(material.pbr.metalness().value_components == 1);
    ufbxt_assert!(material.pbr.diffuse_roughness().value_components == 1);
    ufbxt_assert!(material.pbr.specular_anisotropy().value_components == 1);
    ufbxt_assert!(material.pbr.specular_rotation().value_components == 1);
    ufbxt_assert!(material.pbr.transmission_factor().value_components == 1);
    ufbxt_assert!(material.pbr.transmission_color().value_components == 3);
    ufbxt_assert!(material.pbr.transmission_roughness().value_components == 0);
    ufbxt_assert!(material.pbr.specular_ior().value_components == 1);
    ufbxt_assert!(material.pbr.subsurface_factor().value_components == 1);
    ufbxt_assert!(material.pbr.subsurface_tint_color().value_components == 0);
    ufbxt_assert!(material.pbr.subsurface_color().value_components == 3);
    ufbxt_assert!(material.pbr.subsurface_radius().value_components == 3);
    ufbxt_assert!(material.pbr.subsurface_scale().value_components == 1);
    ufbxt_assert!(material.pbr.emission_factor().value_components == 1);
    ufbxt_assert!(material.pbr.emission_color().value_components == 3);
    ufbxt_assert!(material.pbr.coat_factor().value_components == 1);
    ufbxt_assert!(material.pbr.coat_color().value_components == 3);
    ufbxt_assert!(material.pbr.coat_roughness().value_components == 1);
    ufbxt_assert!(material.pbr.normal_map().value_components == 3);
    ufbxt_assert!(material.pbr.coat_normal().value_components == 3);
    ufbxt_assert!(material.pbr.displacement_map().value_components == 0);
    ufbxt_assert!(material.pbr.opacity().value_components == 3);

    ufbxt_assert!(material.pbr.subsurface_type().value_int == 1);
    ufbxt_assert!(material.pbr.transmission_enable_in_aov().value_int != 0);
    ufbxt_assert!(material.features[MaterialFeature::ThinWalled].enabled);
    ufbxt_assert!(material.features[MaterialFeature::ThinWalled].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::Matte].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Matte].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::Caustics].enabled);
    ufbxt_assert!(material.features[MaterialFeature::Caustics].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::InternalReflections].enabled);
    ufbxt_assert!(material.features[MaterialFeature::InternalReflections].is_explicit);
    ufbxt_assert!(material.features[MaterialFeature::ExitToBackground].enabled);
    ufbxt_assert!(material.features[MaterialFeature::ExitToBackground].is_explicit);
});

file_test!(maya_arnold_properties_shader, file = "maya_arnold_properties", |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "aiStandardSurface1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.shader_type == ShaderType::ArnoldStandardSurface);

    let shader = material.shader.map(|s| &scene.shaders[s]);
    ufbxt_assert!(shader.is_some());
    let shader = shader.unwrap();
    ufbxt_assert!(shader.shader_type == ShaderType::ArnoldStandardSurface);

    let prop = find_shader_prop(scene, shader, "baseColor");
    ufbxt_assert!(prop == "Maya|baseColor");

    let base_color = material.element.props.find_vec3(prop, Vec3::ZERO);
    ufbxt_assert!(2 == round100(base_color.x));
    ufbxt_assert!(3 == round100(base_color.y));
    ufbxt_assert!(4 == round100(base_color.z));

    let prop_none = find_shader_prop(scene, shader, "nonexistent");
    ufbxt_assert!(prop_none.is_empty());
});

file_test!(maya_osl_properties, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "standardSurface2").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();
    ufbxt_assert!(material.shader_type == ShaderType::OslStandardSurface);

    ufbxt_assert!(1 == round100(material.pbr.base_factor().value_vec3().x));
    ufbxt_assert!(2 == round100(material.pbr.base_color().value_vec3().x));
    ufbxt_assert!(3 == round100(material.pbr.base_color().value_vec3().y));
    ufbxt_assert!(4 == round100(material.pbr.base_color().value_vec3().z));
    ufbxt_assert!(5 == round100(material.pbr.diffuse_roughness().value_vec3().x));
    ufbxt_assert!(6 == round100(material.pbr.metalness().value_vec3().x));
    ufbxt_assert!(7 == round100(material.pbr.specular_factor().value_vec3().x));
    ufbxt_assert!(8 == round100(material.pbr.specular_color().value_vec3().x));
    ufbxt_assert!(9 == round100(material.pbr.specular_color().value_vec3().y));
    ufbxt_assert!(10 == round100(material.pbr.specular_color().value_vec3().z));
    ufbxt_assert!(11 == round100(material.pbr.roughness().value_vec3().x));
    ufbxt_assert!(12 == round100(material.pbr.specular_ior().value_vec3().x));
    ufbxt_assert!(13 == round100(material.pbr.specular_anisotropy().value_vec3().x));
    ufbxt_assert!(14 == round100(material.pbr.specular_rotation().value_vec3().x));
    ufbxt_assert!(15 == round100(material.pbr.transmission_factor().value_vec3().x));
    ufbxt_assert!(16 == round100(material.pbr.transmission_color().value_vec3().x));
    ufbxt_assert!(17 == round100(material.pbr.transmission_color().value_vec3().y));
    ufbxt_assert!(18 == round100(material.pbr.transmission_color().value_vec3().z));
    ufbxt_assert!(19 == round100(material.pbr.transmission_depth().value_vec3().x));
    ufbxt_assert!(20 == round100(material.pbr.transmission_scatter().value_vec3().x));
    ufbxt_assert!(21 == round100(material.pbr.transmission_scatter().value_vec3().y));
    ufbxt_assert!(22 == round100(material.pbr.transmission_scatter().value_vec3().z));
    ufbxt_assert!(23 == round100(material.pbr.transmission_scatter_anisotropy().value_vec3().x));
    ufbxt_assert!(24 == round100(material.pbr.transmission_dispersion().value_vec3().x));
    ufbxt_assert!(25 == round100(material.pbr.transmission_extra_roughness().value_vec3().x));
    ufbxt_assert!(26 == round100(material.pbr.subsurface_factor().value_vec3().x));
    ufbxt_assert!(27 == round100(material.pbr.subsurface_color().value_vec3().x));
    ufbxt_assert!(28 == round100(material.pbr.subsurface_color().value_vec3().y));
    ufbxt_assert!(29 == round100(material.pbr.subsurface_color().value_vec3().z));
    ufbxt_assert!(30 == round100(material.pbr.subsurface_radius().value_vec3().x));
    ufbxt_assert!(31 == round100(material.pbr.subsurface_radius().value_vec3().y));
    ufbxt_assert!(32 == round100(material.pbr.subsurface_radius().value_vec3().z));
    ufbxt_assert!(33 == round100(material.pbr.subsurface_scale().value_vec3().x));
    ufbxt_assert!(34 == round100(material.pbr.subsurface_anisotropy().value_vec3().x));
    ufbxt_assert!(35 == round100(material.pbr.coat_factor().value_vec3().x));
    ufbxt_assert!(36 == round100(material.pbr.coat_color().value_vec3().x));
    ufbxt_assert!(37 == round100(material.pbr.coat_color().value_vec3().y));
    ufbxt_assert!(38 == round100(material.pbr.coat_color().value_vec3().z));
    ufbxt_assert!(39 == round100(material.pbr.coat_roughness().value_vec3().x));
    ufbxt_assert!(40 == round100(material.pbr.coat_ior().value_vec3().x));
    ufbxt_assert!(41 == round100(material.pbr.coat_anisotropy().value_vec3().x));
    ufbxt_assert!(42 == round100(material.pbr.coat_rotation().value_vec3().x));
    ufbxt_assert!(46 == round100(material.pbr.sheen_factor().value_vec3().x));
    ufbxt_assert!(47 == round100(material.pbr.sheen_color().value_vec3().x));
    ufbxt_assert!(48 == round100(material.pbr.sheen_color().value_vec3().y));
    ufbxt_assert!(49 == round100(material.pbr.sheen_color().value_vec3().z));
    ufbxt_assert!(50 == round100(material.pbr.sheen_roughness().value_vec3().x));
    ufbxt_assert!(51 == round100(material.pbr.emission_factor().value_vec3().x));
    ufbxt_assert!(52 == round100(material.pbr.emission_color().value_vec3().x));
    ufbxt_assert!(53 == round100(material.pbr.emission_color().value_vec3().y));
    ufbxt_assert!(54 == round100(material.pbr.emission_color().value_vec3().z));
    ufbxt_assert!(55 == round100(material.pbr.thin_film_thickness().value_vec3().x));
    ufbxt_assert!(56 == round100(material.pbr.thin_film_ior().value_vec3().x));
    ufbxt_assert!(57 == round100(material.pbr.opacity().value_vec3().x));
    ufbxt_assert!(58 == round100(material.pbr.opacity().value_vec3().y));
    ufbxt_assert!(59 == round100(material.pbr.opacity().value_vec3().z));

    // Computed
    assert_close_real(err, material.pbr.transmission_roughness().value_real(), 0.36); // 11+25
    ufbxt_assert!(material.pbr.thin_film_factor().value_real() == 1.0);

    ufbxt_assert!(material.features[MaterialFeature::ThinWalled].enabled);
    ufbxt_assert!(material.features[MaterialFeature::ThinWalled].is_explicit);
});

file_test!(maya_texture_layers, |scene, err, load_error| {
    // TODO: Recover layered textures from <7000.....
    if scene.metadata.version < 7000 {
        return;
    }

    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some_and(|m| scene.meshes[m].materials.len() == 1)));
    let material = &scene.materials[scene.meshes[node.unwrap().mesh.unwrap()].materials[0].unwrap()];

    let layered = material.fbx.diffuse_color().texture.map(|t| &scene.textures[t]);
    ufbxt_assert!(layered.is_some());
    let layered = layered.unwrap();
    ufbxt_assert!(layered.texture_type == TextureType::Layered);
    ufbxt_assert!(layered.layers.len() == 3);
    ufbxt_assert!(layered.layers[0].blend_mode == BlendMode::Multiply);
    assert_close_real(err, layered.layers[0].alpha, 0.75);
    ufbxt_assert!(scene.textures[layered.layers[0].texture].relative_filename == "textures\\checkerboard_weight.png");
    ufbxt_assert!(layered.layers[1].blend_mode == BlendMode::Over);
    assert_close_real(err, layered.layers[1].alpha, 0.5);
    ufbxt_assert!(scene.textures[layered.layers[1].texture].relative_filename == "textures\\checkerboard_diffuse.png");
    ufbxt_assert!(layered.layers[2].blend_mode == BlendMode::Additive);
    assert_close_real(err, layered.layers[2].alpha, 1.0);
    ufbxt_assert!(scene.textures[layered.layers[2].texture].relative_filename == "textures\\checkerboard_ambient.png");

    ufbxt_assert!(layered.file_textures.len() == 3);
    for i in 0..3 {
        ufbxt_assert!(layered.file_textures[i] == layered.layers[i].texture);
    }

    {
        let texture = material.fbx.emission_color().texture.map(|t| &scene.textures[t]);
        ufbxt_assert!(texture.is_some());
        let texture = texture.unwrap();
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_emissive.png");
        assert_close_real(err, texture.uv_transform.translation.x, 1.0);
        assert_close_real(err, texture.uv_transform.translation.y, 2.0);
        assert_close_real(err, texture.uv_transform.translation.z, 0.0);

        let mut uv = Vec3::new(0.5, 0.5, 0.0);
        uv = transform_position(&texture.uv_to_texture, uv);
        assert_close_real(err, uv.x, -0.5);
        assert_close_real(err, uv.y, -1.5);
        assert_close_real(err, uv.z, 0.0);
    }

    {
        let texture = material.fbx.transparency_color().texture.map(|t| &scene.textures[t]);
        ufbxt_assert!(texture.is_some());
        let texture = texture.unwrap();
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_transparency.png");
        assert_close_real(err, texture.uv_transform.translation.x, 0.5);
        assert_close_real(err, texture.uv_transform.translation.y, -0.20710678);
        assert_close_real(err, texture.uv_transform.translation.z, 0.0);
        let euler = quat_to_euler(texture.uv_transform.rotation, RotationOrder::Xyz);
        assert_close_real(err, euler.x, 0.0);
        assert_close_real(err, euler.y, 0.0);
        assert_close_real(err, euler.z, 45.0);

        {
            let mut uv = Vec3::new(0.5, 0.5, 0.0);
            uv = transform_position(&texture.uv_to_texture, uv);
            assert_close_real(err, uv.x, 0.5);
            assert_close_real(err, uv.y, 0.5);
            assert_close_real(err, uv.z, 0.0);
        }

        {
            let mut uv = Vec3::new(1.0, 0.5, 0.0);
            uv = transform_position(&texture.uv_to_texture, uv);
            assert_close_real(err, uv.x, 0.853553);
            assert_close_real(err, uv.y, 0.146447);
            assert_close_real(err, uv.z, 0.0);
        }
    }
});

file_test!(maya_texture_blend_modes, |scene, err, load_error| {
    // TODO: Recover layered textures from <7000.....
    if scene.metadata.version < 7000 {
        return;
    }

    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some_and(|m| scene.meshes[m].materials.len() == 1)));
    let material = &scene.materials[scene.meshes[node.unwrap().mesh.unwrap()].materials[0].unwrap()];

    let layered = material.fbx.diffuse_color().texture.map(|t| &scene.textures[t]);
    ufbxt_assert!(layered.is_some());
    let layered = layered.unwrap();
    ufbxt_assert!(layered.texture_type == TextureType::Layered);
    ufbxt_assert!(layered.layers.len() == 14);

    for i in 0..layered.layers.len() {
        let alpha: Real = if i < 10 { (i + 1) as Real * 0.1 } else { 1.0 };
        let ix = layered.layers.len() - i - 1;
        assert_close_real(err, layered.layers[ix].alpha, alpha);
    }

    ufbxt_assert!(layered.file_textures.len() == 1);
    for i in 0..14 {
        ufbxt_assert!(layered.layers[i].texture == layered.file_textures[0]);
    }

    ufbxt_assert!(layered.layers[0].blend_mode == BlendMode::Replace); // "CPV Modulate" (unsupported)
    ufbxt_assert!(layered.layers[1].blend_mode == BlendMode::Luminosity); // "Illuminate"
    ufbxt_assert!(layered.layers[2].blend_mode == BlendMode::Replace); // "Desaturate" (unsupported)
    ufbxt_assert!(layered.layers[3].blend_mode == BlendMode::Saturation);
    ufbxt_assert!(layered.layers[4].blend_mode == BlendMode::Darken);
    ufbxt_assert!(layered.layers[5].blend_mode == BlendMode::Lighten);
    ufbxt_assert!(layered.layers[6].blend_mode == BlendMode::Difference);
    ufbxt_assert!(layered.layers[7].blend_mode == BlendMode::Multiply);
    ufbxt_assert!(layered.layers[8].blend_mode == BlendMode::Subtract);
    ufbxt_assert!(layered.layers[9].blend_mode == BlendMode::Additive);
    ufbxt_assert!(layered.layers[10].blend_mode == BlendMode::Replace); // "Out" (unsupported)
    ufbxt_assert!(layered.layers[11].blend_mode == BlendMode::Replace); // "In" (unsupported)
    ufbxt_assert!(layered.layers[12].blend_mode == BlendMode::Over);
    ufbxt_assert!(layered.layers[13].blend_mode == BlendMode::Replace); // "None"
});

file_test!(max_texture_mapping, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "01 - Default").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    {
        let texture_ix = material.pbr.base_factor().texture;
        ufbxt_assert!(texture_ix.is_some_and(|t| scene.textures[t].shader.is_some()));
        let texture_ix = texture_ix.unwrap();
        let texture = &scene.textures[texture_ix];
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_weight.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "OSLBitmap2");

        let source_hash = fnv1a(shader.shader_source.as_bytes());
        ufbxt_assert!(source_hash == 0x599994a9);

        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(texture.file_textures[0] == texture_ix);
    }

    {
        let texture_ix = material.pbr.base_color().texture;
        ufbxt_assert!(texture_ix.is_some_and(|t| scene.textures[t].shader.is_some()));
        let texture = &scene.textures[texture_ix.unwrap()];
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "RandomTilingBitmap");
        let source_hash = fnv1a(shader.shader_source.as_bytes());
        ufbxt_assert!(source_hash == 0x60186ba7);

        {
            let input = find_shader_texture_input(shader, "Filename");
            ufbxt_assert!(input.is_some());
            ufbxt_assert!(input.unwrap().value_str == "D:\\Dev\\clean\\ufbx\\data\\textures\\checkerboard_diffuse.png");
        }

        {
            let input = find_shader_texture_input(shader, "Scale");
            ufbxt_assert!(input.is_some());
            assert_close_real(err, input.unwrap().value_vec4.x, 0.25);
        }

        ufbxt_assert!(texture.file_textures.is_empty());
    }

    {
        let texture_ix = material.pbr.roughness().texture;
        ufbxt_assert!(texture_ix.is_some_and(|t| scene.textures[t].shader.is_some()));
        let texture_ix = texture_ix.unwrap();
        let texture = &scene.textures[texture_ix];
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_roughness.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "UberBitmap2");

        let source_hash = fnv1a(shader.shader_source.as_bytes());
        ufbxt_assert!(source_hash == 0x19833d47);

        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(texture.file_textures[0] == texture_ix);
    }

    {
        let texture_ix = material.pbr.metalness().texture;
        ufbxt_assert!(texture_ix.is_some_and(|t| scene.textures[t].shader.is_none()));
        let texture_ix = texture_ix.unwrap();
        let texture = &scene.textures[texture_ix];
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_metallic.png");

        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(texture.file_textures[0] == texture_ix);
    }

    {
        let texture_ix = material.pbr.transmission_color().texture;
        ufbxt_assert!(texture_ix.is_some_and(|t| scene.textures[t].shader.is_some()));
        let texture_ix = texture_ix.unwrap();
        let texture = &scene.textures[texture_ix];
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_transparency.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_image");
        ufbxt_assert!(shader.shader_source.is_empty());

        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(texture.file_textures[0] == texture_ix);
    }

    {
        let texture_ix = material.pbr.transmission_color().texture;
        ufbxt_assert!(texture_ix.is_some_and(|t| scene.textures[t].shader.is_some()));
        let texture_ix = texture_ix.unwrap();
        let texture = &scene.textures[texture_ix];
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_transparency.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_source.is_empty());

        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(texture.file_textures[0] == texture_ix);
    }

    {
        let texture_ix = material.pbr.emission_color().texture;
        ufbxt_assert!(texture_ix.is_some_and(|t| scene.textures[t].shader.is_some()));
        let texture = &scene.textures[texture_ix.unwrap()];
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "ColorTweak");

        let source_hash = fnv1a(shader.shader_source.as_bytes());
        ufbxt_assert!(source_hash == 0xd2f4f86f);

        let input = find_shader_texture_input(shader, "Input");
        ufbxt_assert!(input.is_some());
        let input_texture = input.unwrap().texture;
        ufbxt_assert!(input_texture.is_some());
        let input_texture = input_texture.unwrap();
        ufbxt_assert!(scene.textures[input_texture].texture_type == TextureType::File);

        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(texture.file_textures[0] == input_texture);
    }

    {
        let texture_ix = material.pbr.normal_map().texture;
        ufbxt_assert!(texture_ix.is_some_and(|t| scene.textures[t].shader.is_some()));
        let texture = &scene.textures[texture_ix.unwrap()];
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_bump2d");
        ufbxt_assert!(shader.shader_source.is_empty());

        let bump_map = find_shader_texture_input(shader, "bump_map");
        ufbxt_assert!(bump_map.is_some());
        let bump_texture = bump_map.unwrap().texture;
        ufbxt_assert!(bump_texture.is_some());
        let bump_texture = bump_texture.unwrap();
        ufbxt_assert!(scene.textures[bump_texture].texture_type == TextureType::File);

        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(texture.file_textures[0] == bump_texture);
    }
});

file_test!(synthetic_missing_material_factor, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "phong1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    ufbxt_assert!(material.shader_type == ShaderType::FbxPhong);

    assert_close_real(err, material.fbx.diffuse_factor().value_vec3().x, 1.0);
    assert_close_real(err, material.fbx.specular_factor().value_vec3().x, 1.0);
    assert_close_real(err, material.fbx.reflection_factor().value_vec3().x, 1.0);
    assert_close_real(err, material.fbx.transparency_factor().value_vec3().x, 1.0);
    assert_close_real(err, material.fbx.emission_factor().value_vec3().x, 1.0);
    assert_close_real(err, material.fbx.ambient_factor().value_vec3().x, 1.0);

    assert_close_real(err, material.pbr.base_factor().value_vec3().x, 1.0);
    assert_close_real(err, material.pbr.specular_factor().value_vec3().x, 1.0);
    assert_close_real(err, material.pbr.emission_factor().value_vec3().x, 1.0);
});

file_test!(max_instanced_material, |scene, err, load_error| {
    let red_node = find_node(scene, "Red").map(|n| &scene.nodes[n]);
    let green_node = find_node(scene, "Green").map(|n| &scene.nodes[n]);
    let blue_node = find_node(scene, "Blue").map(|n| &scene.nodes[n]);
    ufbxt_assert!(red_node.is_some_and(|n| n.mesh.is_some()));
    ufbxt_assert!(green_node.is_some_and(|n| n.mesh.is_some()));
    ufbxt_assert!(blue_node.is_some_and(|n| n.mesh.is_some()));
    let (red_node, green_node, blue_node) = (red_node.unwrap(), green_node.unwrap(), blue_node.unwrap());
    let red_mesh = red_node.mesh.unwrap();
    let green_mesh = green_node.mesh.unwrap();
    let blue_mesh = blue_node.mesh.unwrap();

    let red_mat = find_element(scene, ElementType::Material, "RedMat");
    let green_mat = find_element(scene, ElementType::Material, "GreenMat");
    let blue_mat = find_element(scene, ElementType::Material, "BlueMat");
    let mat_id = |m: Option<usize>| m.map(|m| scene.materials[m].element.element_id);

    if scene.metadata.version < 7000 {
        ufbxt_assert!(red_mesh != green_mesh);
        ufbxt_assert!(red_mesh != blue_mesh);
        ufbxt_assert!(green_mesh != blue_mesh);

        // 6100 is a bit broken with materials?

        ufbxt_assert!(green_node.materials.len() == 3);
        ufbxt_assert!(red_node.materials.len() == 2);
        ufbxt_assert!(blue_node.materials.len() == 1);

        ufbxt_assert!(scene.meshes[green_mesh].materials.len() == 3);
        ufbxt_assert!(scene.meshes[red_mesh].materials.len() == 2);
        ufbxt_assert!(scene.meshes[blue_mesh].materials.len() == 1);
    } else {
        ufbxt_assert!(red_mesh == green_mesh);
        ufbxt_assert!(red_mesh == blue_mesh);
        ufbxt_assert!(green_mesh == blue_mesh);

        ufbxt_assert!(green_node.materials.len() == 1);
        ufbxt_assert!(red_node.materials.len() == 1);
        ufbxt_assert!(blue_node.materials.len() == 1);

        ufbxt_assert!(mat_id(green_node.materials[0]) == green_mat);
        ufbxt_assert!(mat_id(red_node.materials[0]) == red_mat);
        ufbxt_assert!(mat_id(blue_node.materials[0]) == blue_mat);

        ufbxt_assert!(mat_id(scene.meshes[red_mesh].materials[0]) == green_mat);
    }
});

fn has_texture(scene: &Scene, list: &[usize], name: &str) -> bool {
    for &t in list {
        if scene.textures[t].relative_filename == name {
            return true;
        }
    }
    false
}

fn check_shader_input_map(scene: &Scene, texture: &Texture, name: &str, map: &str, output_index: i64) {
    ufbxt_assert!(texture.shader.is_some());
    let input = find_shader_texture_input(texture.shader.as_ref().unwrap(), name);
    ufbxt_assert!(input.is_some());
    let input = input.unwrap();
    ufbxt_assert!(input.texture.is_some());
    ufbxt_assert!(scene.textures[input.texture.unwrap()].element.name == map);
    ufbxt_assert!(input.texture_output_index == output_index);
}

file_test!(max_shadergraph, |scene, err, load_error| {
    {
        let texture = find_element(scene, ElementType::Texture, "Map #1").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_checkerboard");
        ufbxt_assert!(texture.file_textures.len() == 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #2").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_add");
        ufbxt_assert!(texture.file_textures.len() == 0);
        check_shader_input_map(scene, texture, "input1", "Map #1", 0);
        check_shader_input_map(scene, texture, "input2", "Map #3", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #3").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "Checker");
        ufbxt_assert!(texture.file_textures.len() == 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #6").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "ColorMul");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_diffuse.png"));
        check_shader_input_map(scene, texture, "A", "Map #2", 0);
        check_shader_input_map(scene, texture, "B", "Map #7", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #7").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_diffuse.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "OSLBitmap2");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_diffuse.png"));
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #11").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_metallic.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_image");
        ufbxt_assert!(texture.file_textures.len() == 2);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_diffuse.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_metallic.png"));
        check_shader_input_map(scene, texture, "offset", "Map #7", 2);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #13").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "ColorMax");
        ufbxt_assert!(texture.file_textures.len() == 2);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_diffuse.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_weight.png"));
        check_shader_input_map(scene, texture, "A", "Map #15", 0);
        check_shader_input_map(scene, texture, "B", "Map #6", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #14").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_weight.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "UberBitmap2");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_weight.png"));
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #15").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "TriTone");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_weight.png"));
        check_shader_input_map(scene, texture, "Input_map", "Map #14", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #16").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_mix_rgba");
        ufbxt_assert!(texture.file_textures.len() == 3);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_diffuse.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_metallic.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_weight.png"));
        check_shader_input_map(scene, texture, "input1", "Map #13", 0);
        check_shader_input_map(scene, texture, "input2", "Map #11", 0);
        check_shader_input_map(scene, texture, "mix", "Map #7", 1);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #17").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_dot");
        ufbxt_assert!(texture.file_textures.len() == 3);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_diffuse.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_metallic.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_weight.png"));
        check_shader_input_map(scene, texture, "input1", "Map #16", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #19").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "ColorAdd");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_emissive.png"));
        check_shader_input_map(scene, texture, "A", "Map #20", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #20").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_none()));
        let texture = texture.unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_emissive.png");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_emissive.png"));
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #21").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_add");
        ufbxt_assert!(texture.file_textures.len() == 2);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_emissive.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_specular.png"));
        check_shader_input_map(scene, texture, "input1", "Map #22", 0);
        check_shader_input_map(scene, texture, "input2", "Map #19", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #22").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_specular.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "ai_image");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_specular.png"));
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #26").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "VectorAdd");
        ufbxt_assert!(texture.file_textures.len() == 3);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_emissive.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_specular.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_transparency.png"));
        check_shader_input_map(scene, texture, "A", "Map #28", 0);
        check_shader_input_map(scene, texture, "B", "Map #21", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #28").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_transparency.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "OSLBitmap2");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_transparency.png"));
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #29").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "ColorAdd");
        ufbxt_assert!(texture.file_textures.len() == 4);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_emissive.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_specular.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_transparency.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_ambient.png"));
        check_shader_input_map(scene, texture, "A", "Map #30", 0);
        check_shader_input_map(scene, texture, "B", "Map #26", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #30").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_ambient.png");
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Osl);
        ufbxt_assert!(shader.shader_name == "UberBitmap2");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_ambient.png"));
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #31").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_some()));
        let texture = texture.unwrap();
        let shader = texture.shader.as_ref().unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::Shader);
        ufbxt_assert!(shader.shader_type == ShaderTextureType::Unknown);
        ufbxt_assert!(shader.shader_name == "");
        ufbxt_assert!(shader.shader_type_id == 0x0000023000000000);
        ufbxt_assert!(texture.file_textures.len() == 5);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_emissive.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_specular.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_transparency.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_ambient.png"));
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_displacement.png"));
        check_shader_input_map(scene, texture, "map1", "Map #29", 0);
        check_shader_input_map(scene, texture, "map2", "Map #32", 0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "Map #32").and_then(|id| scene.as_texture(id));
        ufbxt_assert!(texture.is_some_and(|t| t.shader.is_none()));
        let texture = texture.unwrap();
        ufbxt_assert!(texture.texture_type == TextureType::File);
        ufbxt_assert!(texture.relative_filename == "textures\\checkerboard_displacement.png");
        ufbxt_assert!(texture.file_textures.len() == 1);
        ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_displacement.png"));
    }

    {
        let material = find_element(scene, ElementType::Material, "Material #25").and_then(|id| scene.as_material(id));
        ufbxt_assert!(material.is_some());
        let material = material.unwrap();
        ufbxt_assert!(material.shader_type == ShaderType::OslStandardSurface);

        ufbxt_assert!(material.pbr.base_factor().texture.is_some());
        ufbxt_assert!(scene.textures[material.pbr.base_factor().texture.unwrap()].element.name == "Map #17");

        ufbxt_assert!(material.pbr.base_color().texture.is_some());
        ufbxt_assert!(scene.textures[material.pbr.base_color().texture.unwrap()].element.name == "Map #13");

        ufbxt_assert!(material.pbr.emission_factor().texture.is_some());
        ufbxt_assert!(scene.textures[material.pbr.emission_factor().texture.unwrap()].element.name == "Map #31");

        {
            let texture = material.pbr.roughness().texture.map(|t| &scene.textures[t]);
            ufbxt_assert!(texture.is_some());
            let texture = texture.unwrap();
            ufbxt_assert!(texture.shader.is_some());
            let shader = texture.shader.as_ref().unwrap();
            ufbxt_assert!(texture.file_textures.len() == 1);
            ufbxt_assert!(has_texture(scene, &texture.file_textures, "textures\\checkerboard_ambient.png"));
            ufbxt_assert!(shader.main_texture.is_some());
            ufbxt_assert!(shader.main_texture_output_index == 6);
            ufbxt_assert!(scene.textures[shader.main_texture.unwrap()].element.name == "Map #30");
            check_shader_input_map(scene, texture, "sourceMap", "Map #30", 6);
        }
    }
});

file_test!(maya_duplicated_texture, |scene, err, load_error| {
    ufbxt_assert!(scene.texture_files.len() == 1);
    ufbxt_assert!(scene.textures.len() == 4);

    {
        let texture = find_element(scene, ElementType::Texture, "file1").and_then(|id| scene.as_texture(id)).unwrap();
        ufbxt_assert!(texture.has_file);
        ufbxt_assert!(texture.file_index == 0);
        ufbxt_assert!(texture.uv_set == "map1");
        ufbxt_assert!(!texture.has_uv_transform);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "file2").and_then(|id| scene.as_texture(id)).unwrap();
        ufbxt_assert!(texture.has_file);
        ufbxt_assert!(texture.file_index == 0);
        ufbxt_assert!(texture.uv_set == "map1");
        ufbxt_assert!(!texture.has_uv_transform);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "file3").and_then(|id| scene.as_texture(id)).unwrap();
        ufbxt_assert!(texture.has_file);
        ufbxt_assert!(texture.file_index == 0);
        ufbxt_assert!(texture.uv_set == "map1");
        ufbxt_assert!(texture.has_uv_transform);
        assert_close_real(err, texture.uv_transform.scale.x, 2.0);
        assert_close_real(err, texture.uv_transform.scale.y, 2.0);
    }

    {
        let texture = find_element(scene, ElementType::Texture, "file4").and_then(|id| scene.as_texture(id)).unwrap();
        ufbxt_assert!(texture.has_file);
        ufbxt_assert!(texture.file_index == 0);
        ufbxt_assert!(texture.uv_set == "second");
        ufbxt_assert!(!texture.has_uv_transform);
    }
});

fn slash_separator_opts() -> LoadOpts {
    LoadOpts { path_separator: b'/', ..Default::default() }
}

fn backslash_separator_opts() -> LoadOpts {
    LoadOpts { path_separator: b'\\', ..Default::default() }
}

file_test!(maya_absolute_texture, opts = slash_separator_opts(), |scene, err, load_error| {
    let material = find_material(scene, "lambert1").map(|m| &scene.materials[m]);
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    let texture = material.fbx.diffuse_color().texture.map(|t| &scene.textures[t]);
    ufbxt_assert!(texture.is_some());
    let texture = texture.unwrap();
    ufbxt_assert!(texture.relative_filename == "W:\\checkerboard_diffuse.png");
    ufbxt_assert!(texture.absolute_filename == "W:/checkerboard_diffuse.png");
    ufbxt_assert!(texture.filename == "W:/checkerboard_diffuse.png");
});

file_test!(maya_absolute_texture_backslash, file = "maya_absolute_texture", opts = backslash_separator_opts(), |scene, err, load_error| {
    let material = find_material(scene, "lambert1").map(|m| &scene.materials[m]);
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    let texture = material.fbx.diffuse_color().texture.map(|t| &scene.textures[t]);
    ufbxt_assert!(texture.is_some());
    let texture = texture.unwrap();
    ufbxt_assert!(texture.relative_filename == "W:\\checkerboard_diffuse.png");
    ufbxt_assert!(texture.absolute_filename == "W:/checkerboard_diffuse.png");
    ufbxt_assert!(texture.filename == "W:\\checkerboard_diffuse.png");
});

fn load_pixel(texture: &Texture) -> Vec3 {
    let mut error = None;
    let image = read_png(&texture.content, Some(&mut error));
    if let Some(error) = error {
        ufbxt_logf!("failed to read embedded png {}: {}", texture.relative_filename, error);
    }
    ufbxt_assert!(image.width >= 1 && image.height >= 1);
    let pixel = image.pixels[0];
    Vec3::new(srgb_to_linear(pixel.r as Real / 65535.0), srgb_to_linear(pixel.g as Real / 65535.0), srgb_to_linear(pixel.b as Real / 65535.0))
}

fn check_chart_material(scene: &Scene, err: &mut DiffError, mat: &Material, reference: Vec3) {
    ufbxt_hintf!("material = {}", mat.element.name);

    let base_texture = mat.pbr.base_color().texture;
    let mut base_color = mat.pbr.base_color().value_vec3();
    let base_factor = mat.pbr.base_factor().value_real();
    if let Some(t) = base_texture {
        base_color = load_pixel(&scene.textures[t]);
    }

    let emit_texture = mat.pbr.emission_color().texture;
    let mut emit_color = mat.pbr.emission_color().value_vec3();
    let emit_factor = mat.pbr.emission_factor().value_real();
    if let Some(t) = emit_texture {
        emit_color = load_pixel(&scene.textures[t]);
    }

    let sum = add3(mul3(base_color, base_factor), mul3(emit_color, emit_factor));
    if base_texture.is_some() || emit_texture.is_some() {
        assert_close_vec3_threshold(err, sum, reference, 0.01);
    } else {
        assert_close_vec3(err, sum, reference);
    }
}

file_test!(maya_material_chart, |scene, err, load_error| {
    let layer = find_element(scene, ElementType::DisplayLayer, "Sheets").and_then(|id| scene.as_display_layer(id));
    ufbxt_assert!(layer.is_some());
    let layer = layer.unwrap();
    ufbxt_assert!(layer.nodes.len() == 12);

    let reference = Vec3::new(0.125, 0.25, 0.5);
    for &n in &layer.nodes {
        let node = &scene.nodes[n];
        ufbxt_assert!(node.mesh.is_some());
        ufbxt_assert!(node.materials.len() == 1);
        let material = &scene.materials[node.materials[0].unwrap()];
        check_chart_material(scene, err, material, reference);
    }
});

file_test!(blender_402_material_chart, |scene, err, load_error| {
    let reference = Vec3::new(0.25, 0.125, 0.5);
    ufbxt_assert!(scene.materials.len() == 6);
    for material in &scene.materials {
        check_chart_material(scene, err, material, reference);
    }
});

file_test!(blender_suzanne_multimaterial, |scene, err, load_error| {
    let node = find_node(scene, "Suzanne").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    ufbxt_assert!(node.unwrap().mesh.is_some());
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    let order_ref: [u32; 7] = [0, 4, 6, 1, 5, 2, 3];
    ufbxt_assert!(mesh.material_part_usage_order.len() == order_ref.len());
    for i in 0..mesh.material_part_usage_order.len() {
        ufbxt_assert!(mesh.material_part_usage_order[i] == order_ref[i]);
    }
});

file_test!(blender_suzanne_multimaterial_reorder, |scene, err, load_error| {
    let node = find_node(scene, "Suzanne").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    ufbxt_assert!(node.unwrap().mesh.is_some());
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    let order_ref: [u32; 7] = [4, 0, 6, 1, 5, 2, 3];
    ufbxt_assert!(mesh.material_part_usage_order.len() == order_ref.len());
    for i in 0..mesh.material_part_usage_order.len() {
        ufbxt_assert!(mesh.material_part_usage_order[i] == order_ref[i]);
    }
});

file_test!(synthetic_embedded_base64, |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "lambert1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    check_texture_content(scene, material.fbx.diffuse_color().texture, "tiny_clouds.png");
});

file_test!(synthetic_embedded_base64_dom, file = "synthetic_embedded_base64", opts = retain_dom_opts(), |scene, err, load_error| {
    let material = find_element(scene, ElementType::Material, "lambert1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    let texture = material.fbx.diffuse_color().texture.map(|t| &scene.textures[t]);
    ufbxt_assert!(texture.is_some());
    let texture = texture.unwrap();

    let video = texture.video.map(|v| &scene.videos[v]);
    ufbxt_assert!(video.is_some());
    let video = video.unwrap();

    let dom_video = video.element.dom_node.as_ref().and_then(|path| scene.dom_node(path));
    ufbxt_assert!(dom_video.is_some());
    let dom_video = dom_video.unwrap();

    let dom_content = dom_video.find("Content");
    ufbxt_assert!(dom_content.is_some());
    let dom_content = dom_content.unwrap();

    let blobs = dom_content.as_blob_list();
    ufbxt_assert!(blobs.len() == 71);

    let mut total_size = 0;
    for blob in blobs {
        total_size += blob.len();
    }

    let mut offset = 0;
    let mut data = Vec::with_capacity(total_size);
    for blob in blobs {
        data.extend_from_slice(blob);
        offset += blob.len();
    }
    ufbxt_assert!(offset == total_size);

    check_blob_content(&data, "textures/tiny_clouds.png");
});

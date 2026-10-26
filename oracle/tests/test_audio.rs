mod ufbxt;
use ufbxt::*;

file_test!(maya_audio, |scene, err, load_error| {
    let layer = find_element(scene, ElementType::AudioLayer, "audio_track1").and_then(|id| scene.as_audio_layer(id));
    ufbxt_assert!(layer.is_some());
    let layer = layer.unwrap();

    let clip = find_element(scene, ElementType::AudioClip, "plonk").and_then(|id| scene.as_audio_clip(id));
    ufbxt_assert!(clip.is_some());
    let clip = clip.unwrap();

    ufbxt_assert!(layer.clips.len() == 1);
    ufbxt_assert!(scene.audio_clips[layer.clips[0]].element.element_id == clip.element.element_id);

    ufbxt_assert!(clip.relative_filename == "audio\\plonk.wav");
    ufbxt_assert!(clip.absolute_filename == "D:/Dev/clean/ufbx/data/audio/plonk.wav");

    check_blob_content(&clip.content, "audio/plonk.wav");
});

file_test!(maya_audio_clips, |scene, err, load_error| {
    let layer1 = find_element(scene, ElementType::AudioLayer, "audio_track1").and_then(|id| scene.as_audio_layer(id));
    ufbxt_assert!(layer1.is_some());
    let layer2 = find_element(scene, ElementType::AudioLayer, "audio_track2").and_then(|id| scene.as_audio_layer(id));
    ufbxt_assert!(layer2.is_some());
    let (layer1, layer2) = (layer1.unwrap(), layer2.unwrap());

    let plonk = find_element(scene, ElementType::AudioClip, "plonk").and_then(|id| scene.as_audio_clip(id));
    ufbxt_assert!(plonk.is_some());
    let plonk1 = find_element(scene, ElementType::AudioClip, "plonk1").and_then(|id| scene.as_audio_clip(id));
    ufbxt_assert!(plonk1.is_some());
    let plonk_bitcrush = find_element(scene, ElementType::AudioClip, "plonk_bitcrush").and_then(|id| scene.as_audio_clip(id));
    ufbxt_assert!(plonk_bitcrush.is_some());
    let (plonk, plonk1, plonk_bitcrush) = (plonk.unwrap(), plonk1.unwrap(), plonk_bitcrush.unwrap());

    let clip_id = |layer: &AudioLayer, i: usize| scene.audio_clips[layer.clips[i]].element.element_id;

    ufbxt_assert!(layer1.clips.len() == 2);
    ufbxt_assert!(clip_id(layer1, 0) == plonk1.element.element_id);
    ufbxt_assert!(clip_id(layer1, 1) == plonk.element.element_id);

    ufbxt_assert!(layer2.clips.len() == 1);
    ufbxt_assert!(clip_id(layer2, 0) == plonk_bitcrush.element.element_id);

    ufbxt_assert!(plonk.relative_filename == "audio\\plonk.wav");
    ufbxt_assert!(plonk.absolute_filename == "D:/Dev/clean/ufbx/data/audio/plonk.wav");
    ufbxt_assert!(plonk1.relative_filename == "audio\\plonk.wav");
    ufbxt_assert!(plonk1.absolute_filename == "D:/Dev/clean/ufbx/data/audio/plonk.wav");
    ufbxt_assert!(plonk_bitcrush.relative_filename == "audio\\plonk_bitcrush.wav");
    ufbxt_assert!(plonk_bitcrush.absolute_filename == "D:/Dev/clean/ufbx/data/audio/plonk_bitcrush.wav");

    check_blob_content(&plonk.content, "audio/plonk.wav");
    check_blob_content(&plonk1.content, "audio/plonk.wav");
    check_blob_content(&plonk_bitcrush.content, "audio/plonk_bitcrush.wav");
});

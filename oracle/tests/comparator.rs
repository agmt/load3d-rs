use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use fbx_oracle::scene::rt::Cx;
use fbx_oracle::{compare_scenes, convert_scene};
use load3d as f;

fn corpus() -> Option<PathBuf> {
    match std::env::var_os("FBX_ORACLE_CORPUS").map(PathBuf::from) {
        Some(d) if d.is_dir() => Some(d),
        _ => {
            // straight to stderr: the test harness captures only the print macros
            let _ = writeln!(std::io::stderr(), "skipped: set FBX_ORACLE_CORPUS to the data folder of the ufbx repository to run this test");
            None
        }
    }
}

fn scenes(dir: &std::path::Path) -> Vec<(String, ufbx::SceneRoot)> {
    let mut files: Vec<PathBuf> =
        std::fs::read_dir(dir).expect("corpus").filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e == "fbx")).collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| ufbx::load_file(p.to_str()?, ufbx::LoadOpts { retain_dom: true, ..Default::default() }).ok().map(|s| (p.display().to_string(), s)))
        .collect()
}

type Mutation = (&'static str, fn(&mut f::Scene) -> Option<String>);

fn first_prop(s: &mut f::Scene) -> Option<(usize, usize, &mut f::Prop)> {
    let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| !n.element.props.props.is_empty())?;
    Some((i, 0, &mut n.element.props.props[0]))
}

fn dom_number(n: &mut f::DomNode, path: &mut String) -> Option<()> {
    if let Some(j) = n.values.iter().position(|v| v.value_type == f::DomValueType::Number) {
        n.values[j].value_int ^= 1;
        path.push_str(&format!(".values[{j}].value_int"));
        return Some(());
    }
    for (i, c) in n.children.iter_mut().enumerate() {
        let len = path.len();
        path.push_str(&format!(".children[{i}]"));
        if dom_number(c, path).is_some() {
            return Some(());
        }
        path.truncate(len);
    }
    None
}

const MUTATIONS: &[Mutation] = &[
    ("one ulp of a float", |s| {
        let (i, m) = s.meshes.iter_mut().enumerate().find(|(_, m)| m.vertex_position.values.len() > 1)?;
        let v = &mut m.vertex_position.values[1].y;
        *v = f64::from_bits(v.to_bits() ^ 1);
        Some(format!("meshes[{i}].vertex_position.values[1].y"))
    }),
    ("zero to negative zero", |s| {
        let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| n.node_to_world.m10.to_bits() == 0)?;
        n.node_to_world.m10 = -0.0;
        Some(format!("nodes[{i}].node_to_world.m10"))
    }),
    ("a node's parent", |s| {
        let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| n.parent.is_some())?;
        n.parent = n.parent.map(|p| p + 1);
        Some(format!("nodes[{i}].parent"))
    }),
    ("a parent removed", |s| {
        let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| n.parent.is_some())?;
        n.parent = None;
        Some(format!("nodes[{i}].parent"))
    }),
    ("an attribute's element id", |s| {
        let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| n.attrib.is_some())?;
        n.attrib = n.attrib.map(|e| e + 1);
        Some(format!("nodes[{i}].attrib"))
    }),
    ("one index more in a list", |s| {
        s.nodes[0].children.push(0);
        Some("nodes[0].children.len".into())
    }),
    ("an index of a list", |s| {
        let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| n.children.len() > 1)?;
        n.children[1] += 7;
        Some(format!("nodes[{i}].children[1]"))
    }),
    ("a string", |s| {
        s.nodes[0].element.name.push('x');
        Some("nodes[0].element.name".into())
    }),
    ("a blob", |s| {
        s.metadata.raw_filename.push(b'x');
        Some("metadata.raw_filename".into())
    }),
    ("an enum", |s| {
        let n = &mut s.nodes[0];
        n.rotation_order = if n.rotation_order == f::RotationOrder::Zyx { f::RotationOrder::Xyz } else { f::RotationOrder::Zyx };
        Some("nodes[0].rotation_order".into())
    }),
    ("a renamed enum field", |s| {
        let (i, l) = s.lights.iter_mut().enumerate().next()?;
        l.light_type = if l.light_type == f::LightType::Area { f::LightType::Point } else { f::LightType::Area };
        Some(format!("lights[{i}].type_"))
    }),
    ("a prop flag", |s| {
        let (i, j, p) = first_prop(s)?;
        p.flags ^= f::prop_flags::HIDDEN;
        Some(format!("nodes[{i}].element.props.props[{j}].flags"))
    }),
    ("a private field (internal key)", |s| {
        let (i, j, p) = first_prop(s)?;
        p.internal_key ^= 1;
        Some(format!("nodes[{i}].element.props.props[{j}]._internal_key"))
    }),
    ("the value union", |s| {
        let (i, j, p) = first_prop(s)?;
        p.value_real_arr[2] += 1.0;
        Some(format!("nodes[{i}].element.props.props[{j}].value_vec4.z"))
    }),
    ("a material map", |s| {
        let (i, m) = s.materials.iter_mut().enumerate().next()?;
        m.fbx[f::MaterialFbxMap::DiffuseColor].value_vec4.y += 0.5;
        Some(format!("materials[{i}].fbx.diffuse_color.value_vec4.y"))
    }),
    ("a material feature", |s| {
        let (i, m) = s.materials.iter_mut().enumerate().next()?;
        m.features[f::MaterialFeature::Pbr].enabled ^= true;
        Some(format!("materials[{i}].features.pbr.enabled"))
    }),
    ("the shared anim", |s| {
        Arc::make_mut(&mut s.anim).time_end += 1.0;
        Some("anim.time_end".into())
    }),
    ("an anim stack's layers", |s| {
        let (i, st) = s.anim_stacks.iter_mut().enumerate().find(|(_, st)| !st.layers.is_empty())?;
        st.layers[0] += 1;
        Some(format!("anim_stacks[{i}].layers[0]"))
    }),
    ("a keyframe's f32 tangent", |s| {
        let (i, c) = s.anim_curves.iter_mut().enumerate().find(|(_, c)| !c.keyframes.is_empty())?;
        let t = &mut c.keyframes[0].left.dx;
        *t = f32::from_bits(t.to_bits() ^ 1);
        Some(format!("anim_curves[{i}].keyframes[0].left.dx"))
    }),
    ("an anim value's curve", |s| {
        let (i, v) = s.anim_values.iter_mut().enumerate().find(|(_, v)| v.curves[0].is_some())?;
        v.curves[0] = None;
        Some(format!("anim_values[{i}].curves[0]"))
    }),
    ("the template defaults", |s| {
        let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| n.element.props.defaults.is_some())?;
        n.element.props.defaults = None;
        Some(format!("nodes[{i}].element.props.defaults"))
    }),
    ("a template default's value", |s| {
        let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| n.element.props.defaults.as_ref().is_some_and(|d| !d.props.is_empty()))?;
        Arc::make_mut(n.element.props.defaults.as_mut().unwrap()).props[0].value_int += 1;
        Some(format!("nodes[{i}].element.props.defaults.props[0].value_int"))
    }),
    ("an element's DOM node", |s| {
        let (i, n) = s.nodes.iter_mut().enumerate().find(|(_, n)| n.element.dom_node.as_ref().is_some_and(|p| !p.is_empty()))?;
        *n.element.dom_node.as_mut().unwrap().last_mut().unwrap() += 1;
        Some(format!("nodes[{i}].element.dom_node"))
    }),
    ("a DOM value", |s| {
        let mut path = "dom_root".to_string();
        dom_number(s.dom_root.as_mut()?, &mut path)?;
        Some(path)
    }),
    ("the element list", |s| {
        s.elements[1].index += 1;
        Some("elements[1].typed_id".into())
    }),
    ("a connection", |s| {
        let c = s.connections_src.first_mut()?;
        c.src += 1;
        Some("connections_src[0].src".into())
    }),
    ("a name element", |s| {
        let e = s.elements_by_name.first_mut()?;
        e.element += 1;
        Some("elements_by_name[0].element".into())
    }),
    ("a warning", |s| {
        s.metadata.warnings.push(f::Warning { kind: f::WarningType::BadUnicode, description: "x".into(), element_id: u32::MAX, count: 1 });
        Some("metadata.warnings.len".into())
    }),
    ("a flag of has_warning", |s| {
        s.metadata.has_warning[3] ^= true;
        Some("metadata.has_warning[3]".into())
    }),
    ("a skin weight", |s| {
        let (i, d) = s.skin_deformers.iter_mut().enumerate().find(|(_, d)| !d.weights.is_empty())?;
        d.weights[0].weight += 0.25;
        Some(format!("skin_deformers[{i}].weights[0].weight"))
    }),
    ("a blend keyframe's shape", |s| {
        let (i, c) = s.blend_channels.iter_mut().enumerate().find(|(_, c)| !c.keyframes.is_empty())?;
        c.keyframes[0].shape += 1;
        Some(format!("blend_channels[{i}].keyframes[0].shape"))
    }),
    ("a subdivision result that ufbx has not", |s| {
        let (i, m) = s.meshes.iter_mut().enumerate().find(|(_, m)| m.subdivision_result.is_none())?;
        m.subdivision_result = Some(Box::default());
        Some(format!("meshes[{i}].subdivision_result"))
    }),
    ("a texture's file content", |s| {
        let (i, t) = s.textures.iter_mut().enumerate().next()?;
        t.content.push(0);
        Some(format!("textures[{i}].content"))
    }),
    ("a material's texture list", |s| {
        let (i, m) = s.materials.iter_mut().enumerate().find(|(_, m)| !m.textures.is_empty())?;
        m.textures[0].texture += 1;
        Some(format!("materials[{i}].textures[0].texture"))
    }),
    ("a bone pose", |s| {
        let (i, p) = s.poses.iter_mut().enumerate().find(|(_, p)| !p.bone_poses.is_empty())?;
        p.bone_poses[0].bone_to_world.m03 += 1.0;
        Some(format!("poses[{i}].bone_poses[0].bone_to_world.m03"))
    }),
    ("an extra element at the end of a typed list", |s| {
        let n = s.nodes[0].clone();
        s.nodes.push(n);
        Some("nodes.len".into())
    }),
];

#[test]
fn converted_scenes_compare_equal() {
    let Some(dir) = corpus() else { return };
    let all = scenes(&dir);
    assert!(all.len() > 500, "{} scenes loaded", all.len());
    for (name, c) in &all {
        let p = convert_scene(c);
        let o = compare_scenes(c, &p, 5);
        assert_eq!(o.count, 0, "{name}: {:?}", o.kept);
    }
}

#[test]
fn every_change_is_found_at_its_path() {
    let Some(dir) = corpus() else { return };
    let all = scenes(&dir);
    let mut missed = Vec::new();
    for (what, mutate) in MUTATIONS {
        let mut applied = false;
        for (name, c) in &all {
            let mut p = convert_scene(c);
            let Some(expected) = mutate(&mut p) else { continue };
            applied = true;
            let o = compare_scenes(c, &p, 5);
            let paths: Vec<&str> = o.kept.iter().map(|d| d.path.as_str()).collect();
            if o.count != 1 || paths != [expected.as_str()] {
                missed.push(format!("{what} in {name}: expected one difference at {expected}, found {} at {paths:?}", o.count));
            }
            break;
        }
        if !applied {
            missed.push(format!("{what}: no corpus scene to apply it to"));
        }
    }
    assert!(missed.is_empty(), "{}", missed.join("\n"));
}

#[test]
fn values_compare_bit_for_bit_and_nans_alike() {
    let mut cx = Cx::detached(10);
    cx.push("v");
    cx.f64(f64::NAN, f64::NAN);
    cx.f64(f64::from_bits(0x7ff8_0000_0000_0001), f64::NAN);
    cx.f64(0.0, -0.0);
    cx.f64(1.0, 1.0);
    cx.f32(f32::NAN, f32::from_bits(f32::NAN.to_bits() | 1));
    cx.int(-1, -1);
    cx.int(3, 4);
    cx.str(b"abc", b"abc");
    cx.str(b"abc", b"abd");
    cx.bytes(b"", b"\0");
    cx.id(None, None);
    cx.id(Some(2), None);
    cx.len(3, 3);
    cx.len(3, 2);
    assert_eq!(cx.count, 6, "{:?}", cx.kept);
    assert_eq!(cx.checks, 14);
    let shown: Vec<String> = cx.kept.iter().map(|d| format!("{}: {} vs {}", d.path, d.c, d.p)).collect();
    assert_eq!(shown[0], "v: 0.0 vs -0.0");
    assert_eq!(shown[5], "v.len: 3 vs 2");
    assert_eq!(cx.by_field.get("v"), Some(&5));
}

#[test]
fn private_fields_are_read_at_their_offsets() {
    let Some(dir) = corpus() else { return };
    let mut props = 0;
    let mut anim_props = 0;
    for (name, c) in scenes(&dir) {
        let p = convert_scene(&c);
        for n in &p.nodes {
            for q in &n.element.props.props {
                assert_eq!(q.internal_key, f::props::name_key(q.name.as_bytes()), "{name}: prop {:?}", q.name);
                props += 1;
            }
        }
        for e in &p.elements_by_name {
            assert_eq!(e.internal_key, f::props::name_key(e.name.as_bytes()), "{name}: name element {:?}", e.name);
        }
        for l in &p.anim_layers {
            for a in &l.anim_props {
                assert_eq!(a.internal_key, f::props::name_key(a.prop_name.as_bytes()), "{name}: anim prop {:?}", a.prop_name);
                let id = a.element;
                assert!(l.min_element_id <= id && id <= l.max_element_id, "{name}: {id} outside {}..={}", l.min_element_id, l.max_element_id);
                assert!(l.element_id_bitmask[(id as usize >> 5) & 3] & (1 << (id & 31)) != 0, "{name}: {id} not in the bitmask");
                anim_props += 1;
            }
        }
    }
    assert!(props > 10000 && anim_props > 100, "{props} props, {anim_props} anim props");
}

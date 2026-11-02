//! Kept out of `types.rs`, which the oracle pairs with ufbx's.

use crate::matcher::is_format;
use crate::{Face, FileFormat, Material, Mesh, Real, Scene, SceneSettings, Vec3, matrix_determinant, transform_position, triangulate_face};

/// One mesh where one node puts it, in triangles
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlatMesh {
    /// Index into `scene.nodes`
    pub node: usize,
    /// Index into `scene.meshes`
    pub mesh: usize,
    /// `mesh.vertices` through the node's `geometry_to_world`
    pub positions: Vec<Vec3>,
    /// Indices into `positions`, `NO_INDEX` at a corner `IndexErrorHandling::NoIndex` left empty
    pub triangles: Vec<[u32; 3]>,
    /// Each triangle's face in `mesh.faces`
    pub faces: Vec<u32>,
    /// Each triangle's material in `scene.materials`
    pub materials: Vec<Option<usize>>,
}

impl FlatMesh {
    /// The triangles of each material, the materials in the order the triangles first use them
    pub fn material_groups(&self) -> Vec<(usize, Vec<u32>)> {
        let mut groups: Vec<(usize, Vec<u32>)> = Vec::new();
        for (t, material) in self.materials.iter().enumerate() {
            let Some(material) = *material else { continue };
            match groups.iter_mut().find(|(m, _)| *m == material) {
                Some((_, triangles)) => triangles.push(t as u32),
                None => groups.push((material, vec![t as u32])),
            }
        }
        groups
    }
}

/// Every mesh once for each node that shows it, its faces cut by `triangulate_face`. A point or a line has no triangles.
pub fn flatten(scene: &Scene) -> Vec<FlatMesh> {
    flatten_with(scene, |mesh, face| {
        let mut corners = vec![0; 3 * (face.num_indices as usize).saturating_sub(2)];
        let n = triangulate_face(&mut corners, mesh, face) as usize;
        corners[..3 * n].as_chunks::<3>().0.iter().map(|c| [c[0], c[1], c[2]]).collect()
    })
}

/// `flatten` with a program's own cut of a face into triangles, each three mesh indices as `triangulate_face` writes them
pub fn flatten_with(scene: &Scene, mut cut: impl FnMut(&Mesh, Face) -> Vec<[u32; 3]>) -> Vec<FlatMesh> {
    let mut flat = Vec::new();
    for (index, mesh) in scene.meshes.iter().enumerate() {
        let (mut triangles, mut faces, mut slots) = (Vec::new(), Vec::new(), Vec::new());
        for (f, &face) in mesh.faces.iter().enumerate() {
            for corners in cut(mesh, face) {
                triangles.push(corners.map(|c| mesh.vertex_indices[c as usize]));
                faces.push(f as u32);
                slots.push(mesh.face_material.get(f).copied());
            }
        }
        for &instance in &mesh.element.instances {
            let node = &scene.nodes[instance];
            let to_world = &node.geometry_to_world;
            flat.push(FlatMesh {
                node: instance,
                mesh: index,
                positions: mesh.vertices.iter().map(|&p| transform_position(to_world, p)).collect(),
                // a mirror turns a triangle inside out, which swapping two corners puts right
                triangles: if matrix_determinant(to_world) < 0.0 { triangles.iter().map(|&[a, b, c]| [a, c, b]).collect() } else { triangles.clone() },
                faces: faces.clone(),
                materials: slots.iter().map(|slot: &Option<u32>| slot.and_then(|s| node.materials.get(s as usize).copied().flatten())).collect(),
            });
        }
    }
    flat
}

/// The format these bytes are taken for by their content alone, in the order a load tries them
pub fn detect_format(data: &[u8]) -> FileFormat {
    const LOOKAHEAD: usize = 0x4000;
    let mut lookahead = 32;
    loop {
        let head = &data[..lookahead.min(data.len())];
        for format in [FileFormat::Fbx, FileFormat::Obj, FileFormat::Mtl] {
            if is_format(head, format) {
                return format;
            }
        }
        if lookahead >= data.len() || lookahead >= LOOKAHEAD {
            break;
        }
        lookahead = (lookahead * 2).min(LOOKAHEAD);
    }
    if cfg!(feature = "format-3ds") && is_format(data, FileFormat::Tds) { FileFormat::Tds } else { FileFormat::Unknown }
}

impl Material {
    pub fn diffuse(&self) -> Option<Vec3> {
        let map = self.fbx.diffuse_color();
        map.has_value.then_some(Vec3::new(map.value_vec4.x, map.value_vec4.y, map.value_vec4.z))
    }
}

impl SceneSettings {
    /// Metres in one of the file's units where it states one; `unit_meters` is otherwise ufbx's default or nothing
    pub fn stated_unit(&self) -> Option<Real> {
        self.props.find_own("UnitScaleFactor").map(|_| self.unit_meters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Element, NO_INDEX, Node};

    #[cfg(feature = "format-fbx")]
    fn fbx(settings: &str, objects: &str, connections: &str) -> crate::Scene {
        let text = format!(
            "; FBX 7.4.0 project file\nFBXHeaderExtension:  {{\n    FBXVersion: 7400\n}}\nGlobalSettings:  {{\n    Properties70:  {{\n{settings}\n    }}\n}}\nObjects:  {{\n{objects}\n}}\nConnections:  {{\n{connections}\n}}\n"
        );
        crate::load_memory(text.as_bytes(), &crate::LoadOpts::default()).unwrap()
    }

    #[cfg(feature = "format-fbx")]
    const TRIANGLE: &str = "    Geometry: 1, \"Geometry::shape\", \"Mesh\" {\n        Vertices: *9 {\n            a: 1,0,0,0,1,0,0,0,1\n        }\n        PolygonVertexIndex: *3 {\n            a: 0,1,-3\n        }\n    }";

    #[cfg(feature = "format-fbx")]
    fn model(id: i64, name: &str, property: &str) -> String {
        format!("    Model: {id}, \"Model::{name}\", \"Mesh\" {{\n        Properties70:  {{\n            {property}\n        }}\n    }}")
    }

    fn mesh(faces: &[Face], indices: &[u32], slots: &[u32], instances: &[usize]) -> Mesh {
        Mesh {
            element: Element { instances: instances.to_vec(), ..Element::default() },
            vertices: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)],
            num_vertices: 4,
            faces: faces.to_vec(),
            num_indices: indices.len(),
            vertex_indices: indices.to_vec(),
            face_material: slots.to_vec(),
            ..Mesh::default()
        }
    }

    fn placed_as_it_is() -> Node {
        Node { geometry_to_world: crate::Matrix::IDENTITY, ..Node::default() }
    }

    #[test]
    #[cfg(feature = "format-obj")]
    fn a_cube_of_quads_comes_as_twelve_triangles_of_six_faces() {
        let cube = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\nv 1 1 1\nv 0 1 1\nf 1 4 3 2\nf 5 6 7 8\nf 1 2 6 5\nf 2 3 7 6\nf 3 4 8 7\nf 4 1 5 8\n";
        let scene = crate::load_memory(cube.as_bytes(), &crate::LoadOpts::default()).unwrap();
        let flat = flatten(&scene);
        assert_eq!(flat.len(), 1);
        assert_eq!((flat[0].positions.len(), flat[0].triangles.len()), (8, 12));
        assert_eq!(flat[0].faces, [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
        assert_eq!(flat[0].materials.len(), 12);
    }

    #[test]
    #[cfg(feature = "format-fbx")]
    fn a_mesh_comes_once_for_each_node_where_that_node_puts_it() {
        let objects = [
            TRIANGLE.to_string(),
            model(2, "left", "P: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\", -10,0,0"),
            model(3, "right", "P: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\", 10,0,0"),
        ];
        let scene = fbx("", &objects.join("\n"), "    C: \"OO\",1,2\n    C: \"OO\",1,3\n    C: \"OO\",2,0\n    C: \"OO\",3,0");
        let flat = flatten(&scene);
        let at: Vec<(&str, f64)> = flat.iter().map(|f| (scene.nodes[f.node].element.name.as_str(), f.positions[0].x)).collect();
        assert_eq!(at.len(), 2);
        assert!(at.contains(&("left", -9.0)) && at.contains(&("right", 11.0)), "{at:?}");
        assert!(flat.iter().all(|f| f.mesh == flat[0].mesh && f.triangles == [[0, 1, 2]]));
    }

    #[test]
    #[cfg(feature = "format-fbx")]
    fn a_mirroring_node_leaves_its_triangles_facing_the_same_way_out() {
        let objects = [TRIANGLE.to_string(), model(2, "mirrored", "P: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\", -1,1,1")];
        let scene = fbx("", &objects.join("\n"), "    C: \"OO\",1,2\n    C: \"OO\",2,0");
        assert_eq!(flatten(&scene)[0].triangles, [[0, 2, 1]]);
    }

    #[test]
    fn each_instance_wears_its_own_nodes_materials() {
        let (one, other) = (Node { materials: vec![Some(7), None], ..placed_as_it_is() }, Node { materials: vec![Some(1), Some(2)], ..placed_as_it_is() });
        let shared = mesh(&[Face { index_begin: 0, num_indices: 3 }, Face { index_begin: 3, num_indices: 3 }], &[0, 1, 2, 0, 2, 3], &[0, 1], &[0, 1]);
        let scene = Scene { nodes: vec![one, other], meshes: vec![shared], ..Scene::default() };
        let flat = flatten(&scene);
        assert_eq!(flat[0].materials, [Some(7), None]);
        assert_eq!(flat[1].materials, [Some(1), Some(2)]);
    }

    #[test]
    fn a_materials_triangles_come_in_the_order_first_used() {
        let part = FlatMesh { materials: vec![Some(5), None, Some(2), Some(5)], ..FlatMesh::default() };
        assert_eq!(part.material_groups(), [(5, vec![0, 3]), (2, vec![2])]);
    }

    #[test]
    fn a_corner_left_empty_stays_empty() {
        let scene = Scene {
            nodes: vec![placed_as_it_is()],
            meshes: vec![mesh(&[Face { index_begin: 0, num_indices: 3 }], &[0, NO_INDEX, 2], &[], &[0])],
            ..Scene::default()
        };
        assert_eq!(flatten(&scene)[0].triangles, [[0, NO_INDEX, 2]]);
    }

    #[test]
    fn a_program_may_cut_a_face_its_own_way() {
        let scene = Scene {
            nodes: vec![placed_as_it_is()],
            meshes: vec![mesh(&[Face { index_begin: 0, num_indices: 4 }], &[0, 1, 2, 3], &[], &[0])],
            ..Scene::default()
        };
        let flat = flatten_with(&scene, |_, face| {
            vec![[face.index_begin + 1, face.index_begin + 2, face.index_begin + 3], [face.index_begin + 1, face.index_begin + 3, face.index_begin]]
        });
        assert_eq!(flat[0].triangles, [[1, 2, 3], [1, 3, 0]]);
        assert_eq!(flat[0].faces, [0, 0]);
    }

    #[test]
    fn a_format_is_told_by_its_content() {
        assert_eq!(detect_format(b"Kaydara FBX Binary  \x00\x1a\x00"), FileFormat::Fbx);
        assert_eq!(detect_format(b"; FBX 7.5.0 project file\n"), FileFormat::Fbx);
        assert_eq!(detect_format(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"), FileFormat::Obj);
        assert_eq!(detect_format(b"newmtl paint\nKd 1 0 0\n"), FileFormat::Mtl);
        assert_eq!(detect_format(b"solid hull\nfacet"), FileFormat::Unknown);
        assert_eq!(detect_format(b""), FileFormat::Unknown);
        assert_eq!(detect_format(b"MM"), FileFormat::Unknown);
        let tds = if cfg!(feature = "format-3ds") { FileFormat::Tds } else { FileFormat::Unknown };
        assert_eq!(detect_format(b"MM\x06\0\0\0"), tds);
    }

    #[test]
    fn a_line_past_the_first_look_counts_and_one_past_the_last_does_not() {
        let at = |n: usize| [vec![b'#'; n], b"\nf 1 2 3\n".to_vec()].concat();
        assert_eq!(detect_format(&at(100)), FileFormat::Obj);
        assert_eq!(detect_format(&at(0x4000)), FileFormat::Unknown);
    }

    #[test]
    #[cfg(feature = "format-obj")]
    fn a_colour_is_read_alike_in_any_format() {
        let opts = crate::LoadOpts { obj_mtl_data: b"newmtl paint\nKd 1 0.5 0\nnewmtl bare\n".to_vec(), ..crate::LoadOpts::default() };
        let scene = crate::load_memory(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl paint\nf 1 2 3\nusemtl bare\nf 1 3 2\n", &opts).unwrap();
        let of = |name: &str| scene.materials.iter().find(|m| m.element.name == name).unwrap().diffuse();
        assert_eq!(of("paint"), Some(Vec3::new(1.0, 0.5, 0.0)));
        assert_eq!(of("bare"), None);
    }

    #[test]
    #[cfg(feature = "format-fbx")]
    fn the_stated_unit_is_the_files_whatever_the_load_turned_it_into() {
        let stated = "        P: \"UnitScaleFactor\", \"double\", \"Number\", \"\", 100";
        let scene = fbx(stated, &model(2, "empty", ""), "    C: \"OO\",2,0");
        assert_eq!(scene.settings.stated_unit(), Some(1.0));
        let unstated = fbx("", &model(2, "empty", ""), "    C: \"OO\",2,0");
        assert_eq!(unstated.settings.stated_unit(), None);
        let text = format!(
            "; FBX 7.4.0 project file\nFBXHeaderExtension:  {{\n    FBXVersion: 7400\n}}\nGlobalSettings:  {{\n    Properties70:  {{\n{stated}\n    }}\n}}\n"
        );
        let turned = crate::load_memory(text.as_bytes(), &crate::LoadOpts { target_unit_meters: 0.001, ..crate::LoadOpts::default() }).unwrap();
        assert_eq!(turned.settings.stated_unit(), Some(1.0));
    }

    #[test]
    #[cfg(feature = "format-obj")]
    fn an_obj_states_no_unit() {
        let scene = crate::load_memory(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", &crate::LoadOpts::default()).unwrap();
        assert_eq!(scene.settings.stated_unit(), None);
    }
}

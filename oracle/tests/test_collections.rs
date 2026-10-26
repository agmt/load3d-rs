mod ufbxt;
use ufbxt::*;

fn find_selection_set<'a>(scene: &'a Scene, name: &str) -> Option<&'a SelectionSet> {
    find_element(scene, ElementType::SelectionSet, name).map(|id| &scene.selection_sets[scene.elements[id as usize].index])
}

fn find_display_layer<'a>(scene: &'a Scene, name: &str) -> Option<&'a DisplayLayer> {
    find_element(scene, ElementType::DisplayLayer, name).map(|id| &scene.display_layers[scene.elements[id as usize].index])
}

file_test!(max_selection_sets, |scene, err, load_error| {
    let node_ix = find_node(scene, "Box001");
    ufbxt_assert!(node_ix.is_some_and(|n| scene.nodes[n].mesh.is_some()));
    let node_ix = node_ix.unwrap();
    let node = &scene.nodes[node_ix];
    let mesh_ix = node.mesh.unwrap();
    let mesh = &scene.meshes[mesh_ix];

    {
        let set = find_selection_set(scene, "ObjectCube");
        ufbxt_assert!(set.is_some_and(|s| s.nodes.len() == 1));
        let sel = &scene.selection_nodes[set.unwrap().nodes[0]];
        ufbxt_assert!(sel.target_node == Some(node_ix) && sel.target_mesh == Some(mesh_ix));
        ufbxt_assert!(sel.include_node);
        ufbxt_assert!(sel.vertices.is_empty());
        ufbxt_assert!(sel.edges.is_empty());
        ufbxt_assert!(sel.faces.is_empty());
    }

    {
        let set = find_selection_set(scene, "Box001_TopFace_Face");
        ufbxt_assert!(set.is_some_and(|s| s.nodes.len() == 1));
        let sel = &scene.selection_nodes[set.unwrap().nodes[0]];
        ufbxt_assert!(sel.target_node == Some(node_ix) && sel.target_mesh == Some(mesh_ix));
        ufbxt_assert!(!sel.include_node);
        ufbxt_assert!(sel.vertices.is_empty());
        ufbxt_assert!(sel.edges.is_empty());
        ufbxt_assert!(sel.faces.len() == 2);

        for &face_ix in &sel.faces {
            let face = mesh.faces[face_ix as usize];
            for j in 0..face.num_indices {
                let pos = mesh.vertex_position.get((face.index_begin + j) as usize);
                let pos = transform_position(&node.geometry_to_world, pos);
                assert_close_real(err, pos.y / 10.0, 2.0);
            }
        }
    }

    {
        let set = find_selection_set(scene, "Box001_BottomFace_Face");
        ufbxt_assert!(set.is_some_and(|s| s.nodes.len() == 1));
        let sel = &scene.selection_nodes[set.unwrap().nodes[0]];
        ufbxt_assert!(sel.target_node == Some(node_ix) && sel.target_mesh == Some(mesh_ix));
        ufbxt_assert!(!sel.include_node);
        ufbxt_assert!(sel.vertices.is_empty());
        ufbxt_assert!(sel.edges.is_empty());
        ufbxt_assert!(sel.faces.len() == 2);

        for &face_ix in &sel.faces {
            let face = mesh.faces[face_ix as usize];
            for j in 0..face.num_indices {
                let pos = mesh.vertex_position.get((face.index_begin + j) as usize);
                let pos = transform_position(&node.geometry_to_world, pos);
                assert_close_real(err, pos.y / 10.0, 0.0);
            }
        }
    }

    {
        let set = find_selection_set(scene, "Box001_SideFaces_Face");
        ufbxt_assert!(set.is_some_and(|s| s.nodes.len() == 1));
        let sel = &scene.selection_nodes[set.unwrap().nodes[0]];
        ufbxt_assert!(sel.target_node == Some(node_ix) && sel.target_mesh == Some(mesh_ix));
        ufbxt_assert!(!sel.include_node);
        ufbxt_assert!(sel.vertices.is_empty());
        ufbxt_assert!(sel.edges.is_empty());
        ufbxt_assert!(sel.faces.len() == 8);

        for &face_ix in &sel.faces {
            let face = mesh.faces[face_ix as usize];
            let mut avg: Real = 0.0;
            for j in 0..face.num_indices {
                let pos = mesh.vertex_position.get((face.index_begin + j) as usize);
                let pos = transform_position(&node.geometry_to_world, pos);
                avg += pos.y;
            }
            avg /= face.num_indices as Real * 10.0;
            ufbxt_assert!(avg > 0.5 && avg < 1.5);
        }
    }

    {
        let set = find_selection_set(scene, "Box001_TopEdges_Edge");
        ufbxt_assert!(set.is_some_and(|s| s.nodes.len() == 1));
        let sel = &scene.selection_nodes[set.unwrap().nodes[0]];
        ufbxt_assert!(sel.target_node == Some(node_ix) && sel.target_mesh == Some(mesh_ix));
        ufbxt_assert!(!sel.include_node);
        ufbxt_assert!(sel.vertices.is_empty());
        ufbxt_assert!(sel.edges.len() == 4);
        ufbxt_assert!(sel.faces.is_empty());

        for &edge_ix in &sel.edges {
            let edge = mesh.edges[edge_ix as usize];
            for index in [edge.a, edge.b] {
                let pos = mesh.vertex_position.get(index as usize);
                let pos = transform_position(&node.geometry_to_world, pos);
                assert_close_real(err, pos.y / 10.0, 2.0);
            }
        }
    }

    {
        let set = find_selection_set(scene, "Box001_BottomEdges_Edge");
        ufbxt_assert!(set.is_some_and(|s| s.nodes.len() == 1));
        let sel = &scene.selection_nodes[set.unwrap().nodes[0]];
        ufbxt_assert!(sel.target_node == Some(node_ix) && sel.target_mesh == Some(mesh_ix));
        ufbxt_assert!(!sel.include_node);
        ufbxt_assert!(sel.vertices.is_empty());
        ufbxt_assert!(sel.edges.len() == 4);
        ufbxt_assert!(sel.faces.is_empty());

        let mesh = &scene.meshes[sel.target_mesh.unwrap()];
        for &edge_ix in &sel.edges {
            let edge = mesh.edges[edge_ix as usize];
            for index in [edge.a, edge.b] {
                let pos = mesh.vertex_position.get(index as usize);
                let pos = transform_position(&node.geometry_to_world, pos);
                assert_close_real(err, pos.y / 10.0, 0.0);
            }
        }
    }

    {
        let set = find_selection_set(scene, "Box001_TopVerts_Vertex");
        ufbxt_assert!(set.is_some_and(|s| s.nodes.len() == 1));
        let sel = &scene.selection_nodes[set.unwrap().nodes[0]];
        ufbxt_assert!(sel.target_node == Some(node_ix) && sel.target_mesh == Some(mesh_ix));
        ufbxt_assert!(!sel.include_node);
        ufbxt_assert!(sel.vertices.len() == 4);
        ufbxt_assert!(sel.edges.is_empty());
        ufbxt_assert!(sel.faces.is_empty());

        for &vertex_ix in &sel.vertices {
            let pos = mesh.vertices[vertex_ix as usize];
            let pos = transform_position(&scene.nodes[sel.target_node.unwrap()].geometry_to_world, pos);
            assert_close_real(err, pos.y / 10.0, 2.0);
        }
    }

    {
        let set = find_selection_set(scene, "Box001_BottomVerts_Vertex");
        ufbxt_assert!(set.is_some_and(|s| s.nodes.len() == 1));
        let sel = &scene.selection_nodes[set.unwrap().nodes[0]];
        ufbxt_assert!(sel.target_node == Some(node_ix) && sel.target_mesh == Some(mesh_ix));
        ufbxt_assert!(!sel.include_node);
        ufbxt_assert!(sel.vertices.len() == 4);
        ufbxt_assert!(sel.edges.is_empty());
        ufbxt_assert!(sel.faces.is_empty());

        for &vertex_ix in &sel.vertices {
            let pos = mesh.vertices[vertex_ix as usize];
            let pos = transform_position(&scene.nodes[sel.target_node.unwrap()].geometry_to_world, pos);
            assert_close_real(err, pos.y / 10.0, 0.0);
        }
    }
});

file_test!(maya_display_layers, |scene, err, load_error| {
    let colors = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)];

    ufbxt_assert!(scene.display_layers.len() == 3);
    ufbxt_assert!(scene.nodes.len() == 5);

    {
        let layer = find_display_layer(scene, "LayerA");
        ufbxt_assert!(layer.is_some_and(|l| l.nodes.len() == 1 && scene.nodes[l.nodes[0]].element.name == "NodeA"));
        let layer = layer.unwrap();
        ufbxt_assert!(layer.visible && !layer.frozen);
        assert_close_vec3(err, layer.ui_color, colors[0]);
    }

    {
        let layer = find_display_layer(scene, "LayerB");
        ufbxt_assert!(layer.is_some_and(|l| l.nodes.len() == 1 && scene.nodes[l.nodes[0]].element.name == "NodeB"));
        let layer = layer.unwrap();
        ufbxt_assert!(layer.visible && layer.frozen);
        assert_close_vec3(err, layer.ui_color, colors[1]);
    }

    {
        let layer = find_display_layer(scene, "LayerC");
        ufbxt_assert!(layer.is_some_and(|l| l.nodes.len() == 1 && scene.nodes[l.nodes[0]].element.name == "NodeC"));
        let layer = layer.unwrap();
        ufbxt_assert!(!layer.visible && !layer.frozen);
        assert_close_vec3(err, layer.ui_color, colors[2]);
    }
});

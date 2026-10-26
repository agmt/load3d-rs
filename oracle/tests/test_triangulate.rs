mod ufbxt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use ufbxt::*;

fn do_triangulate_test(scene: &Scene) -> usize {
    let mut num_fail = 0;

    for mesh in &scene.meshes {
        ufbxt_assert!(mesh.element.instances.len() == 1);
        let instance = &scene.nodes[mesh.element.instances[0]];
        let should_be_top_left = instance.element.name.starts_with('A');
        ufbxt_assert!(mesh.num_faces == 1);
        let face = mesh.faces[0];
        ufbxt_assert!(face.index_begin == 0);
        ufbxt_assert!(face.num_indices == 4);
        let mut tris = [0u32; 6];
        let ok = triangulate_face(&mut tris, mesh, face);
        ufbxt_assert!(ok != 0);

        let mut top_left_ix = 0;
        let mut best_dot = Real::INFINITY;
        for ix in 0..4 {
            let v = mesh.vertex_position.get(ix);
            let dot = v.x + v.z;
            if dot < best_dot {
                top_left_ix = ix;
                best_dot = dot;
            }
        }

        let top_left_count = tris.iter().filter(|&&t| t as usize == top_left_ix).count();

        if should_be_top_left != (top_left_count == 2) {
            ufbxt_logf!("Fail: {}", instance.element.name);
            num_fail += 1;
        }
    }

    ufbxt_logf!("Triangulations OK: {}/{}", scene.meshes.len() - num_fail, scene.meshes.len());
    num_fail
}

file_test!(maya_triangulate, |scene, err, load_error| {
    let num_fail = do_triangulate_test(scene);
    ufbxt_assert!(num_fail <= 4);
});

file_test!(maya_triangulate_down, |scene, err, load_error| {
    let num_fail = do_triangulate_test(scene);
    ufbxt_assert!(num_fail <= 1);
});

file_test!(maya_tri_cone, |scene, err, load_error| {
    let node = find_node(scene, "pCone1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    for i in 0..mesh.num_faces {
        let face = mesh.faces[i];
        ufbxt_assert!(face.num_indices >= 3 && face.num_indices <= 32);

        let mut tris = [0u32; 32];

        let num_tris = face.num_indices - 2;
        for i in 0..32 {
            // `ufbx_catch_triangulate_face` in ufbx.c
            let result = catch_unwind(AssertUnwindSafe(|| triangulate_face(&mut tris[..i], mesh, face)));
            let did_panic = result.is_err();
            let num_tris_returned = result.unwrap_or(0);
            if i >= num_tris as usize * 3 {
                ufbxt_assert!(!did_panic);
                ufbxt_assert!(num_tris_returned == num_tris);
            } else {
                ufbxt_assert!(did_panic);
                ufbxt_assert!(num_tris_returned == 0);
            }
        }

        ufbxt_assert!(triangulate_face(&mut tris, mesh, face) != 0);
    }
});

#[allow(dead_code)]
fn ngon_write_obj(path: &str, mesh: &Mesh, indices: &[u32], num_triangles: usize) {
    use std::fmt::Write;
    let mut f = String::new();

    for v in &mesh.vertices {
        writeln!(f, "v {:.6} {:.6} {:.6}", v.x, v.y, v.z).unwrap();
    }

    writeln!(f).unwrap();
    for tri in indices[..num_triangles * 3].chunks(3) {
        writeln!(f, "f {} {} {}", mesh.vertex_indices[tri[0] as usize] + 1, mesh.vertex_indices[tri[1] as usize] + 1, mesh.vertex_indices[tri[2] as usize] + 1)
            .unwrap();
    }

    std::fs::write(path, f).unwrap();
}

fn ngon_3d_to_2d(basis: &[Vec3; 2], pos: Vec3) -> Vec2 {
    Vec2::new(dot3(basis[0], pos), dot3(basis[1], pos))
}

fn check_ngon_triangulation(err: &mut DiffError, mesh: &Mesh, face: Face, indices: &[u32], num_triangles: usize, basis: &[Vec3; 2]) {
    let begin = face.index_begin as usize;
    let n = face.num_indices as usize;
    let mut poly_area = 0.0;
    for i in 0..n {
        let a = ngon_3d_to_2d(basis, mesh.vertex_position.get(begin + i));
        let b = ngon_3d_to_2d(basis, mesh.vertex_position.get(begin + (i + 1) % n));
        poly_area += 0.5 * (a.x * b.y - a.y * b.x);
    }

    let mut tri_area = 0.0;
    for i in 0..num_triangles {
        let a = ngon_3d_to_2d(basis, mesh.vertex_position.get(indices[i * 3] as usize));
        let b = ngon_3d_to_2d(basis, mesh.vertex_position.get(indices[i * 3 + 1] as usize));
        let c = ngon_3d_to_2d(basis, mesh.vertex_position.get(indices[i * 3 + 2] as usize));
        let area = 0.5 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
        ufbxt_assert!(area >= -0.05);
        tri_area += area;
    }

    // ufbx's relative threshold is for a float `ufbx_real`; the port's `Real` is a double
    assert_close_real(err, poly_area, tri_area);
}

const BASIS_XY: [Vec3; 2] = [Vec3 { x: 1.0, y: 0.0, z: 0.0 }, Vec3 { x: 0.0, y: 1.0, z: 0.0 }];

fn check_single_ngon(err: &mut DiffError, scene: &Scene, name: &str, max_tris: usize, expected_tris: u32) {
    let node = find_node(scene, name).map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    ufbxt_assert!(mesh.num_faces == 1);
    let face = mesh.faces[0];

    let mut indices = vec![0u32; max_tris * 3];
    let num_tris = triangulate_face(&mut indices, mesh, face);
    ufbxt_assert!(num_tris == expected_tris);

    check_ngon_triangulation(err, mesh, face, &indices, num_tris as usize, &BASIS_XY);
}

file_test!(blender_300_ngon_intersection, |scene, err, load_error| {
    check_single_ngon(err, scene, "Plane", 3, 3);
});

file_test!(blender_300_ngon_e, |scene, err, load_error| {
    check_single_ngon(err, scene, "Plane", 10, 10);
});

file_test!(blender_300_ngon_abstract, |scene, err, load_error| {
    check_single_ngon(err, scene, "Plane", 144, 144);
});

file_test!(blender_300_ngon_big, |scene, err, load_error| {
    check_single_ngon(err, scene, "Plane", 8028, 8028);
});

file_test!(blender_300_ngon_irregular, |scene, err, load_error| {
    let node = find_node(scene, "Plane").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    let mut indices = [0u32; 256];

    for &face in &mesh.faces {
        let num_tris = triangulate_face(&mut indices, mesh, face);
        ufbxt_assert!(num_tris == face.num_indices - 2);
    }
});

file_test!(triangulate_empty, file = "maya_cube_7500_binary", |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    let empty_face = Face { index_begin: 0, num_indices: 0 };
    let mut indices = [0u32; 16];
    let result = triangulate_face(&mut indices, mesh, empty_face);
    ufbxt_assert!(result == 0);
});

file_test!(maya_ngon_gs, |scene, err, load_error| {
    check_single_ngon(err, scene, "G", 128, 126);
    check_single_ngon(err, scene, "S", 256, 198);
});

file_test!(maya_ngon_maze_segment, |scene, err, load_error| {
    check_single_ngon(err, scene, "Grid", 64, 60);
});

file_test!(maya_ngon_l, |scene, err, load_error| {
    check_single_ngon(err, scene, "Grid", 8, 8);
});

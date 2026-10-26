mod ufbxt;
use ufbxt::*;

fn check_generated_normals(mesh: &Mesh, err: &mut DiffError, expected_normals: usize) {
    let mut topo = vec![TopoEdge::default(); mesh.num_indices];

    compute_topology(mesh, &mut topo);

    let mut normal_indices = vec![0u32; mesh.num_indices];
    let num_normals = generate_normal_mapping(mesh, &topo, &mut normal_indices, false);

    if expected_normals > 0 {
        ufbxt_assert!(num_normals == expected_normals);
    }

    let mut normals = vec![Vec3::ZERO; num_normals];
    compute_normals(mesh, &mesh.vertex_position, &normal_indices, &mut normals);

    let new_normals = VertexVec3 { exists: true, values: normals, indices: normal_indices, ..Default::default() };

    for i in 0..mesh.num_indices {
        let fn_ = mesh.vertex_normal.get(i);
        let rn = new_normals.get(i);

        assert_close_vec3(err, fn_, rn);
    }
}

fn node_mesh<'a>(scene: &'a Scene, name: &str) -> &'a Mesh {
    let node = find_node(scene, name).map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    &scene.meshes[node.unwrap().mesh.unwrap()]
}

file_test!(maya_edge_smoothing, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");
    check_generated_normals(mesh, err, 16);
});

file_test!(maya_no_smoothing, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");
    check_generated_normals(mesh, err, 16);
});

file_test!(maya_planar_ngon, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pDisc1");
    check_generated_normals(mesh, err, 0);
});

file_test!(maya_subsurf_cube, |scene, err, load_error| {});

file_test!(maya_subsurf_plane, |scene, err, load_error| {});

file_test!(maya_subsurf_cube_crease, |scene, err, load_error| {});

file_test!(blender_293_suzanne_subsurf, |scene, err, load_error| {});

file_test!(subsurf_alloc_fail, file = "maya_subsurf_cube", |scene, err, load_error| {
    let _mesh = node_mesh(scene, "pCube1");
});

struct SubsurfBoundaryRef {
    node_name: &'static str,
    boundary: SubdivisionBoundary,
}

file_test!(blender_293_suzanne_subsurf_uv, |scene, err, load_error| {
    let refs = [
        SubsurfBoundaryRef { node_name: "SharpBoundary", boundary: SubdivisionBoundary::SharpBoundary },
        SubsurfBoundaryRef { node_name: "SharpCorners", boundary: SubdivisionBoundary::SharpCorners },
        SubsurfBoundaryRef { node_name: "SharpInterior", boundary: SubdivisionBoundary::SharpInterior },
        SubsurfBoundaryRef { node_name: "SharpNone", boundary: SubdivisionBoundary::SharpNone },
    ];

    for r in &refs {
        let node = find_node(scene, r.node_name).map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let prop = node.unwrap().element.props.find("ufbx:UVBoundary");
        ufbxt_assert!(prop.is_some());
        let prop = prop.unwrap();
        ufbxt_assert!((prop.flags & prop_flags::USER_DEFINED) != 0);
        ufbxt_assert!(prop.prop_type == PropType::Integer);
        ufbxt_assert!(prop.value_int == r.boundary as i64);
    }
});

file_test!(blender_293x_nonmanifold_subsurf, |scene, err, load_error| {});

file_test!(blender_293_ngon_subsurf, |scene, err, load_error| {});

file_test!(blender_293x_subsurf_boundary, |scene, err, load_error| {});

file_test!(blender_293x_subsurf_max_crease, |scene, err, load_error| {
    let mesh = scene.as_mesh(find_element(scene, ElementType::Mesh, "Plane").unwrap()).unwrap();
    let subdivided = ufbxt_subdivide_mesh(scene, mesh, 1, None).unwrap();

    for i in 0..mesh.num_edges {
        assert_close_real(err, mesh.edge_crease[i], 1.0);
    }

    let mut num_edge = 0;
    let mut num_center = 0;
    for i in 0..subdivided.num_edges {
        let edge = subdivided.edges[i];
        let a = subdivided.vertex_position.get(edge.a as usize);
        let b = subdivided.vertex_position.get(edge.b as usize);
        let a_len = a.x * a.x + a.y * a.y + a.z * a.z;
        let b_len = b.x * b.x + b.y * b.y + b.z * b.z;

        if a_len < 0.01 || b_len < 0.01 {
            assert_close_real(err, subdivided.edge_crease[i], 0.0);
            num_center += 1;
        } else {
            assert_close_real(err, subdivided.edge_crease[i], 1.0);
            num_edge += 1;
        }
    }

    ufbxt_assert!(num_edge == 8);
    ufbxt_assert!(num_center == 4);
});

file_test!(maya_subsurf_max_crease, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let subdivided = ufbxt_subdivide_mesh(scene, &scene.meshes[node.unwrap().mesh.unwrap()], 1, None);
    ufbxt_assert!(subdivided.is_ok());
    let subdivided = subdivided.unwrap();

    let mut num_top = 0;
    let mut num_bottom = 0;
    for i in 0..subdivided.num_edges {
        let edge = subdivided.edges[i];
        let a = subdivided.vertex_position.get(edge.a as usize);
        let b = subdivided.vertex_position.get(edge.b as usize);
        let a_len = a.x * a.x + a.z * a.z;
        let b_len = b.x * b.x + b.z * b.z;

        if a.y < -0.49 && b.y < -0.49 && a_len > 0.01 && b_len > 0.01 {
            assert_close_real(err, subdivided.edge_crease[i], 0.8);
            num_bottom += 1;
        } else if a.y > 0.49 && b.y > 0.49 && a_len > 0.01 && b_len > 0.01 {
            assert_close_real(err, subdivided.edge_crease[i], 1.0);
            num_top += 1;
        } else {
            assert_close_real(err, subdivided.edge_crease[i], 0.0);
        }
    }

    ufbxt_assert!(num_top == 8);
    ufbxt_assert!(num_bottom == 8);
});

file_test!(maya_subsurf_3x_cube, |scene, err, load_error| {});

file_test!(maya_subsurf_3x_cube_crease, |scene, err, load_error| {});

/// `ufbxt_vertex_pn` in test/test_topology.h, as the bytes of its two `ufbx_vec3`
const VERTEX_PN_SIZE: usize = 2 * VEC3_SIZE;
const VEC3_SIZE: usize = 3 * size_of::<Real>();

fn vec3_bytes(v: Vec3) -> Vec<u8> {
    [v.x, v.y, v.z].iter().flat_map(|c| c.to_ne_bytes()).collect()
}

fn triangulated_corners(mesh: &Mesh) -> Vec<(Vec3, Vec3)> {
    let mut corners = Vec::new();
    let mut tri = [0u32; 64];
    for fi in 0..mesh.num_faces {
        let num_tris = triangulate_face(&mut tri, mesh, mesh.faces[fi]) as usize;
        for &ix in &tri[..num_tris * 3] {
            corners.push((mesh.vertex_position.get(ix as usize), mesh.vertex_normal.get(ix as usize)));
        }
    }
    corners
}

fn vertices_pn(corners: &[(Vec3, Vec3)]) -> Vec<u8> {
    corners.iter().flat_map(|&(position, normal)| [vec3_bytes(position), vec3_bytes(normal)].concat()).collect()
}

file_test!(blender_293_half_smooth_cube, |scene, err, load_error| {
    let mesh = node_mesh(scene, "Cube");
    ufbxt_assert!(mesh.num_indices == 6 * 4);
    ufbxt_assert!(mesh.num_triangles == 6 * 2);

    let corners = triangulated_corners(mesh);
    let num_indices = corners.len();
    let mut vertices = vertices_pn(&corners);
    let mut indices = [0u32; 64];

    let mut stream = [VertexStream { data: &mut vertices, vertex_count: num_indices, vertex_size: VERTEX_PN_SIZE }];
    let num_vertices = generate_indices(&mut stream, &mut indices[..num_indices]).unwrap();
    ufbxt_assert!(num_vertices == 12);
});

file_test!(generate_indices_multi_stream, file = "blender_293_half_smooth_cube_7400_binary", |scene, err, load_error| {
    let mesh = node_mesh(scene, "Cube");
    ufbxt_assert!(mesh.num_indices == 6 * 4);
    ufbxt_assert!(mesh.num_triangles == 6 * 2);

    let corners = triangulated_corners(mesh);
    let num_indices = corners.len();
    let mut positions: Vec<u8> = corners.iter().flat_map(|c| vec3_bytes(c.0)).collect();
    let mut normals: Vec<u8> = corners.iter().flat_map(|c| vec3_bytes(c.1)).collect();
    let mut indices = [0u32; 64];

    let mut streams = [
        VertexStream { data: &mut positions, vertex_count: num_indices, vertex_size: VEC3_SIZE },
        VertexStream { data: &mut normals, vertex_count: num_indices, vertex_size: VEC3_SIZE },
    ];
    let num_vertices = generate_indices(&mut streams, &mut indices[..num_indices]).unwrap();
    ufbxt_assert!(num_vertices == 12);
});

file_test!(maya_vertex_crease_single, |scene, err, load_error| {});

const MIRROR_AXES: [MirrorAxis; 4] = [MirrorAxis::None, MirrorAxis::X, MirrorAxis::Y, MirrorAxis::Z];

fn vertex_crease_lefthanded(file: &str, obj_name: &str) {
    let mut err = DiffError::default();

    let obj_file = load_obj_file(obj_name, None);

    for path in FileIterator::new(file) {
        for (i, &axis) in MIRROR_AXES.iter().enumerate() {
            ufbxt_hintf!("mirror_axis={i}");

            let opts =
                LoadOpts { target_axes: CoordinateAxes::LEFT_HANDED_Y_UP, handedness_conversion_axis: axis, reverse_winding: i == 0, ..Default::default() };

            let scene = load_file(&path, &opts);
            if let Err(e) = &scene {
                log_error(e);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_scene(&scene);
            diff_to_obj(&scene, &obj_file, &mut err, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn maya_vertex_crease_single_lefthanded() {
    vertex_crease_lefthanded("maya_vertex_crease_single", "maya_vertex_crease_single_lefthanded");
}

file_test!(maya_vertex_crease, |scene, err, load_error| {});

#[test]
fn maya_vertex_crease_lefthanded() {
    vertex_crease_lefthanded("maya_vertex_crease", "maya_vertex_crease_lefthanded");
}

file_test!(blender_312x_vertex_crease, |scene, err, load_error| {});

#[test]
fn generate_indices_empty_vertex() {
    let mut indices = [0u32; 9];
    let result = generate_indices(&mut [], &mut indices);
    let num_vertices = *result.as_ref().unwrap_or(&0);
    ufbxt_assert!(num_vertices == 0);
    ufbxt_assert!(result.is_err_and(|e| e.kind == ErrorType::ZeroVertexSize));
}

#[test]
fn generate_indices_no_indices() {
    let mut vertices = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)].iter().flat_map(|&v| vec3_bytes(v)).collect::<Vec<u8>>();
    let mut streams = [VertexStream { data: &mut vertices, vertex_count: 3, vertex_size: VEC3_SIZE }];
    let mut indices = [0u32; 9];
    let result = generate_indices(&mut streams, &mut indices[..0]);
    let num_vertices = *result.as_ref().unwrap_or(&0);
    ufbxt_assert!(num_vertices == 0);
    ufbxt_assert!(result.is_ok());
}

#[test]
fn generate_indices_truncated_stream() {
    let mut vertices = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)].iter().flat_map(|&v| vec3_bytes(v)).collect::<Vec<u8>>();
    let mut streams = [VertexStream { data: &mut vertices, vertex_count: 2, vertex_size: VEC3_SIZE }];
    let mut indices = [0u32; 9];
    let result = generate_indices(&mut streams, &mut indices[..3]);
    let num_vertices = *result.as_ref().unwrap_or(&0);
    ufbxt_assert!(num_vertices == 0);
    ufbxt_assert!(result.is_err());
    let error = result.unwrap_err();
    ufbxt_assert!(error.kind == ErrorType::TruncatedVertexStream);
    ufbxt_assert!(error.info == "0");
    ufbxt_assert!(error.info.len() == 1);
}

file_test!(generate_indices_streams, file = "blender_293_half_smooth_cube", |scene, err, load_error| {
    let mesh = node_mesh(scene, "Cube");
    ufbxt_assert!(mesh.num_indices == 6 * 4);
    ufbxt_assert!(mesh.num_triangles == 6 * 2);

    let corners = triangulated_corners(mesh);
    let num_indices = corners.len();
    let mut positions: Vec<u8> = corners.iter().flat_map(|c| vec3_bytes(c.0)).collect();
    let mut normals: Vec<u8> = corners.iter().flat_map(|c| vec3_bytes(c.1)).collect();
    let mut indices = [0u32; 64];

    let mut streams = [
        VertexStream { data: &mut positions, vertex_count: num_indices, vertex_size: VEC3_SIZE },
        VertexStream { data: &mut normals, vertex_count: num_indices, vertex_size: VEC3_SIZE },
    ];
    let num_vertices = generate_indices(&mut streams, &mut indices[..num_indices]).unwrap();
    ufbxt_assert!(num_vertices == 12);
});

/// `ufbxt_vertex_huge` as C lays it out: two `ufbx_vec3`, 4096 bytes of padding and a `uint32_t` index, padded to
/// the doubles' alignment
const VERTEX_HUGE_INDEX_OFFSET: usize = VERTEX_PN_SIZE + 4096;
const VERTEX_HUGE_SIZE: usize = (VERTEX_HUGE_INDEX_OFFSET + 4).next_multiple_of(align_of::<Real>());

fn vertices_huge(corners: &[(Vec3, Vec3)], with_index: bool) -> Vec<u8> {
    let mut vertices = vec![0u8; corners.len() * VERTEX_HUGE_SIZE];
    for (i, (&(position, normal), vertex)) in corners.iter().zip(vertices.chunks_mut(VERTEX_HUGE_SIZE)).enumerate() {
        vertex[..VEC3_SIZE].copy_from_slice(&vec3_bytes(position));
        vertex[VEC3_SIZE..VERTEX_PN_SIZE].copy_from_slice(&vec3_bytes(normal));
        if with_index {
            vertex[VERTEX_HUGE_INDEX_OFFSET..VERTEX_HUGE_INDEX_OFFSET + 4].copy_from_slice(&(i as u32).to_ne_bytes());
        }
    }
    vertices
}

file_test!(generate_indices_huge_vertices, file = "blender_293_half_smooth_cube", |scene, err, load_error| {
    let mesh = node_mesh(scene, "Cube");
    ufbxt_assert!(mesh.num_indices == 6 * 4);
    ufbxt_assert!(mesh.num_triangles == 6 * 2);

    let corners = triangulated_corners(mesh);
    let num_indices = corners.len();

    {
        let mut vertices = vertices_huge(&corners, false);
        let mut indices = [0u32; 64];

        let mut stream = [VertexStream { data: &mut vertices, vertex_count: num_indices, vertex_size: VERTEX_HUGE_SIZE }];
        let num_vertices = generate_indices(&mut stream, &mut indices[..num_indices]).unwrap();
        ufbxt_assert!(num_vertices == 12);
    }

    {
        let mut vertices = vertices_huge(&corners, true);
        let mut indices = [0u32; 64];

        let mut stream = [VertexStream { data: &mut vertices, vertex_count: num_indices, vertex_size: VERTEX_HUGE_SIZE }];
        let num_vertices = generate_indices(&mut stream, &mut indices[..num_indices]).unwrap();
        ufbxt_assert!(num_vertices == mesh.num_triangles * 3);
    }
});

file_test!(generate_indices_huge_streams, file = "blender_293_half_smooth_cube", |scene, err, load_error| {
    let mesh = node_mesh(scene, "Cube");
    ufbxt_assert!(mesh.num_indices == 6 * 4);
    ufbxt_assert!(mesh.num_triangles == 6 * 2);

    let corners = triangulated_corners(mesh);
    let num_indices = corners.len();
    let num_streams = 4096;

    // ufbx's streams share one buffer of zeros; the port writes through each stream, so each has its own
    {
        let mut vertices = vertices_pn(&corners);
        let mut indices = [0u32; 64];
        let mut zeros = vec![[0u8; 64]; num_streams - 1];

        let mut streams = vec![VertexStream { data: &mut vertices, vertex_count: num_indices, vertex_size: VERTEX_PN_SIZE }];
        for zero in &mut zeros {
            streams.push(VertexStream { data: zero, vertex_count: num_indices, vertex_size: 1 });
        }

        let num_vertices = generate_indices(&mut streams, &mut indices[..num_indices]).unwrap();
        ufbxt_assert!(num_vertices == 12);
    }

    {
        let mut vertices = vertices_pn(&corners);
        let mut indices = [0u32; 64];
        let mut zeros = vec![[0u8; 64]; num_streams - 2];
        let mut vertex_indices = [0u32; 64];
        for (i, ix) in vertex_indices[..num_indices].iter_mut().enumerate() {
            *ix = i as u32;
        }
        let mut vertex_indices: Vec<u8> = vertex_indices.iter().flat_map(|ix| ix.to_ne_bytes()).collect();

        let mut streams = vec![VertexStream { data: &mut vertices, vertex_count: num_indices, vertex_size: VERTEX_PN_SIZE }];
        for zero in &mut zeros {
            streams.push(VertexStream { data: zero, vertex_count: num_indices, vertex_size: 1 });
        }
        streams.push(VertexStream { data: &mut vertex_indices, vertex_count: num_indices, vertex_size: size_of::<u32>() });

        let num_vertices = generate_indices(&mut streams, &mut indices[..num_indices]).unwrap();
        ufbxt_assert!(num_vertices == num_indices);
    }
});

fn subsurf_interpolate_opts_test(err: &mut DiffError, scene: &Scene, opts: &SubdivideOpts, check_normals: bool) {
    let mesh = node_mesh(scene, "pCube1");
    ufbxt_assert!(mesh.vertex_tangent.exists);

    let sub_mesh = ufbxt_subdivide_mesh(scene, mesh, 1, Some(opts));
    ufbxt_assert!(sub_mesh.is_ok());
    let sub_mesh = sub_mesh.unwrap();
    if !check_normals {
        ufbxt_assert!(!sub_mesh.vertex_normal.exists);
    }

    ufbxt_assert!(sub_mesh.faces.len() == mesh.faces.len() * 4);
    for (src_face_ix, &src_face) in mesh.faces.iter().enumerate() {
        let begin = src_face.index_begin as usize;
        let normal = mesh.vertex_normal.get(begin);
        let tangent = mesh.vertex_tangent.get(begin);
        let bitangent = mesh.vertex_bitangent.get(begin);

        for i in 0..src_face.num_indices as usize {
            assert_close_vec3(err, normal, mesh.vertex_normal.get(begin + i));
            assert_close_vec3(err, tangent, mesh.vertex_tangent.get(begin + i));
            assert_close_vec3(err, bitangent, mesh.vertex_bitangent.get(begin + i));
        }

        for dst_subface in 0..4 {
            let dst_face_ix = src_face_ix * 4 + dst_subface;
            let dst_face = sub_mesh.faces[dst_face_ix];

            for i in 0..dst_face.num_indices as usize {
                let ix = dst_face.index_begin as usize + i;
                if check_normals {
                    assert_close_vec3(err, normal, sub_mesh.vertex_normal.get(ix));
                }
                assert_close_vec3(err, tangent, sub_mesh.vertex_tangent.get(ix));
                assert_close_vec3(err, bitangent, sub_mesh.vertex_bitangent.get(ix));
            }
        }
    }
}

file_test!(subsurf_interpolate, file = "maya_cube", |scene, err, load_error| {
    let opts = SubdivideOpts { interpolate_normals: true, interpolate_tangents: true, ..Default::default() };
    subsurf_interpolate_opts_test(err, scene, &opts, true);
});

file_test!(subsurf_interpolate_ignore, file = "maya_cube", |scene, err, load_error| {
    let opts = SubdivideOpts { ignore_normals: true, interpolate_tangents: true, ..Default::default() };
    subsurf_interpolate_opts_test(err, scene, &opts, false);
});

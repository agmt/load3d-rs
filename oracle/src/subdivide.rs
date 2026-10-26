use std::alloc::Layout;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::Debug;

use load3d as fbx;

use crate::common::{both, canon, v2, v3, v4};
use crate::runner::{Args, Child, Job, Section, Tally, corpus_jobs};
use crate::scene::rt::flag_bits;

const KNOWN_C_FAILURE: &str = "ufbx 0.23.1 frees memory it still writes on valence 16+ vertices in ufbxi_subdivide_layer";
const C_DIED: &str = "ufbx_subdivide_mesh died";

const GUARD: usize = 64;
const POISON: u8 = 0xa5;

struct Block {
    ptr: *mut u8,
    cap: usize,
    size: usize,
}

/// C subdivision's temp blocks. ufbx 0.23.1 writes through a stale pointer one element past an inputs buffer it has just
/// grown: small blocks grow in place here, so that pointer stays right, and one that moves keeps a guard the write hits
#[derive(Default)]
struct Guarded {
    blocks: Vec<Block>,
    at: HashMap<usize, usize>,
    overrun: bool,
}

fn block_layout(cap: usize) -> Layout {
    Layout::from_size_align(cap, 16).unwrap()
}

impl Block {
    fn guard(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.add(self.size), GUARD) }
    }

    fn poison(&mut self) {
        unsafe { std::ptr::write_bytes(self.ptr.add(self.size), POISON, GUARD) };
    }
}

impl Guarded {
    fn alloc(&mut self, size: usize) -> *mut u8 {
        let cap = if size < 1 << 20 { ((size + GUARD).next_power_of_two() * 8).max(4096) } else { size + GUARD };
        let ptr = unsafe { std::alloc::alloc(block_layout(cap)) };
        if !ptr.is_null() {
            let mut b = Block { ptr, cap, size };
            b.poison();
            self.at.insert(ptr as usize, self.blocks.len());
            self.blocks.push(b);
        }
        ptr
    }

    fn realloc(&mut self, old: *mut u8, new_size: usize) -> *mut u8 {
        let Some(&i) = self.at.get(&(old as usize)) else { return std::ptr::null_mut() };
        let b = &mut self.blocks[i];
        self.overrun |= b.guard().iter().any(|&x| x != POISON);
        if new_size + GUARD <= b.cap {
            b.size = new_size;
            b.poison();
            return old;
        }
        let old_size = b.size;
        let p = self.alloc(new_size);
        if !p.is_null() {
            unsafe { std::ptr::copy_nonoverlapping(old, p, old_size.min(new_size)) };
        }
        p
    }

    /// ufbx's temp allocator opts pointing at `self`, which must stay put until the call returns
    fn opts(&mut self) -> ufbx::AllocatorOpts {
        unsafe extern "C" fn alloc(user: *mut c_void, size: usize) -> *mut c_void {
            unsafe { &mut *(user as *mut Guarded) }.alloc(size) as *mut c_void
        }
        unsafe extern "C" fn realloc(user: *mut c_void, old: *mut c_void, _: usize, new_size: usize) -> *mut c_void {
            unsafe { &mut *(user as *mut Guarded) }.realloc(old as *mut u8, new_size) as *mut c_void
        }
        unsafe extern "C" fn free(_: *mut c_void, _: *mut c_void, _: usize) {}
        let raw = ufbx::RawAllocator {
            alloc_fn: Some(alloc),
            realloc_fn: Some(realloc),
            free_fn: Some(free),
            free_allocator_fn: None,
            user: self as *mut Guarded as *mut c_void,
        };
        ufbx::AllocatorOpts { allocator: ufbx::Allocator::Raw(unsafe { ufbx::Unsafe::new(raw) }), ..Default::default() }
    }

    fn overrun(&self) -> bool {
        self.overrun || self.blocks.iter().any(|b| b.guard().iter().any(|&x| x != POISON))
    }
}

impl Drop for Guarded {
    fn drop(&mut self) {
        for b in &self.blocks {
            unsafe { std::alloc::dealloc(b.ptr, block_layout(b.cap)) };
        }
    }
}

macro_rules! conv_attrib {
    ($a:expr, $f:expr) => {{
        let a = &$a;
        fbx::VertexAttrib {
            exists: a.exists,
            values: a.values.iter().map($f).collect(),
            indices: a.indices.to_vec(),
            value_reals: a.value_reals,
            unique_per_vertex: a.unique_per_vertex,
            values_w: a.values_w.to_vec(),
        }
    }};
}

fn boundary(b: ufbx::SubdivisionBoundary) -> fbx::SubdivisionBoundary {
    use fbx::SubdivisionBoundary as R;
    use ufbx::SubdivisionBoundary as C;
    match b {
        C::Default => R::Default,
        C::Legacy => R::Legacy,
        C::SharpCorners => R::SharpCorners,
        C::SharpNone => R::SharpNone,
        C::SharpBoundary => R::SharpBoundary,
        C::SharpInterior => R::SharpInterior,
    }
}

fn part(p: &ufbx::MeshPart) -> fbx::MeshPart {
    fbx::MeshPart {
        index: p.index,
        num_faces: p.num_faces,
        num_triangles: p.num_triangles,
        num_empty_faces: p.num_empty_faces,
        num_point_faces: p.num_point_faces,
        num_line_faces: p.num_line_faces,
        face_indices: p.face_indices.to_vec(),
    }
}

fn sub_result(s: &ufbx::SubdivisionResult) -> fbx::SubdivisionResult {
    let ranges = |l: &ufbx::List<ufbx::SubdivisionWeightRange>| {
        l.iter().map(|r| fbx::SubdivisionWeightRange { weight_begin: r.weight_begin, num_weights: r.num_weights }).collect()
    };
    let weights = |l: &ufbx::List<ufbx::SubdivisionWeight>| l.iter().map(|w| fbx::SubdivisionWeight { weight: w.weight, index: w.index }).collect();
    fbx::SubdivisionResult {
        source_vertex_ranges: ranges(&s.source_vertex_ranges),
        source_vertex_weights: weights(&s.source_vertex_weights),
        skin_cluster_ranges: ranges(&s.skin_cluster_ranges),
        skin_cluster_weights: weights(&s.skin_cluster_weights),
        ..Default::default()
    }
}

fn to_rs(m: &ufbx::Mesh) -> fbx::Mesh {
    fbx::Mesh {
        num_vertices: m.num_vertices,
        num_indices: m.num_indices,
        num_faces: m.num_faces,
        num_triangles: m.num_triangles,
        num_edges: m.num_edges,
        max_face_triangles: m.max_face_triangles,
        num_empty_faces: m.num_empty_faces,
        num_point_faces: m.num_point_faces,
        num_line_faces: m.num_line_faces,
        faces: m.faces.iter().map(|f| fbx::Face { index_begin: f.index_begin, num_indices: f.num_indices }).collect(),
        face_smoothing: m.face_smoothing.to_vec(),
        face_material: m.face_material.to_vec(),
        face_group: m.face_group.to_vec(),
        face_hole: m.face_hole.to_vec(),
        edges: m.edges.iter().map(|e| fbx::Edge { a: e.a, b: e.b }).collect(),
        // ufbx tells a list the file gave empty from none by its pointer; the port has these flags for it
        has_edges: !m.edges.data.is_null(),
        edge_smoothing: m.edge_smoothing.to_vec(),
        has_edge_smoothing: !m.edge_smoothing.data.is_null(),
        edge_crease: m.edge_crease.to_vec(),
        edge_visibility: m.edge_visibility.to_vec(),
        vertex_indices: m.vertex_indices.to_vec(),
        vertices: m.vertices.iter().map(v3).collect(),
        vertex_first_index: m.vertex_first_index.to_vec(),
        vertex_position: conv_attrib!(m.vertex_position, v3),
        vertex_normal: conv_attrib!(m.vertex_normal, v3),
        vertex_uv: conv_attrib!(m.vertex_uv, v2),
        vertex_tangent: conv_attrib!(m.vertex_tangent, v3),
        vertex_bitangent: conv_attrib!(m.vertex_bitangent, v3),
        vertex_color: conv_attrib!(m.vertex_color, v4),
        vertex_crease: conv_attrib!(m.vertex_crease, |&x: &f64| x),
        uv_sets: m
            .uv_sets
            .iter()
            .map(|s| fbx::UvSet {
                name: s.name.to_string(),
                index: s.index,
                vertex_uv: conv_attrib!(s.vertex_uv, v2),
                vertex_tangent: conv_attrib!(s.vertex_tangent, v3),
                vertex_bitangent: conv_attrib!(s.vertex_bitangent, v3),
            })
            .collect(),
        color_sets: m
            .color_sets
            .iter()
            .map(|s| fbx::ColorSet { name: s.name.to_string(), index: s.index, vertex_color: conv_attrib!(s.vertex_color, v4) })
            .collect(),
        materials: (0..m.materials.count).map(Some).collect(),
        face_groups: m.face_groups.iter().map(|g| fbx::FaceGroup { id: g.id, name: g.name.to_string() }).collect(),
        material_parts: m.material_parts.iter().map(part).collect(),
        face_group_parts: m.face_group_parts.iter().map(part).collect(),
        material_part_usage_order: m.material_part_usage_order.to_vec(),
        skinned_is_local: m.skinned_is_local,
        skinned_position: conv_attrib!(m.skinned_position, v3),
        skinned_normal: conv_attrib!(m.skinned_normal, v3),
        skin_deformers: (0..m.skin_deformers.count).collect(),
        subdivision_preview_levels: m.subdivision_preview_levels,
        subdivision_render_levels: m.subdivision_render_levels,
        subdivision_boundary: boundary(m.subdivision_boundary),
        subdivision_uv_boundary: boundary(m.subdivision_uv_boundary),
        reversed_winding: m.reversed_winding,
        generated_normals: m.generated_normals,
        subdivision_evaluated: m.subdivision_evaluated,
        subdivision_result: m.subdivision_result.as_ref().map(|s| Box::new(sub_result(s))),
        from_tessellated_nurbs: m.from_tessellated_nurbs,
        ..Default::default()
    }
}

fn scene_for(m: &ufbx::Mesh) -> fbx::Scene {
    let mut scene = fbx::Scene::default();
    for s in &m.skin_deformers {
        scene.skin_deformers.push(fbx::SkinDeformer {
            clusters: (0..s.clusters.count).collect(),
            vertices: s.vertices.iter().map(|v| fbx::SkinVertex { weight_begin: v.weight_begin, num_weights: v.num_weights, dq_weight: v.dq_weight }).collect(),
            weights: s.weights.iter().map(|w| fbx::SkinWeight { cluster_index: w.cluster_index, weight: w.weight }).collect(),
            ..Default::default()
        });
    }
    scene
}

fn eq<T: PartialEq + Debug>(t: &mut Tally, ctx: &str, name: &str, c: T, r: T) {
    t.check(c == r, || format!("{ctx} {name}: C {c:?}, port {r:?}").chars().take(300).collect());
}

fn list<T: PartialEq + Debug>(t: &mut Tally, ctx: &str, name: &str, c: &[T], r: &[T]) {
    if !t.check(c.len() == r.len(), || format!("{ctx} {name}: len C {}, port {}", c.len(), r.len())) {
        return;
    }
    let first = (0..c.len()).find(|&i| c[i] != r[i]);
    t.check(first.is_none(), || {
        let i = first.unwrap();
        format!("{ctx} {name}[{i}]: C {:?}, port {:?}", c[i], r[i]).chars().take(300).collect()
    });
}

macro_rules! cmp_attrib {
    ($t:expr, $ctx:expr, $name:expr, $c:expr, $r:expr, |$v:ident| $bits:expr) => {{
        let (t, ctx, c, r, name) = (&mut *$t, $ctx, &$c, &$r, $name);
        eq(t, ctx, &format!("{name}.exists"), c.exists, r.exists);
        eq(t, ctx, &format!("{name}.value_reals"), c.value_reals, r.value_reals);
        eq(t, ctx, &format!("{name}.unique_per_vertex"), c.unique_per_vertex, r.unique_per_vertex);
        let cb: Vec<u64> = c.values.iter().flat_map(|$v| $bits).collect();
        let rb: Vec<u64> = r.values.iter().flat_map(|$v| $bits).collect();
        list(t, ctx, &format!("{name}.values"), &cb, &rb);
        list(t, ctx, &format!("{name}.indices"), &*c.indices, &*r.indices);
        let cw: Vec<u64> = c.values_w.iter().map(|&w| canon(w)).collect();
        let rw: Vec<u64> = r.values_w.iter().map(|&w| canon(w)).collect();
        list(t, ctx, &format!("{name}.values_w"), &cw, &rw);
    }};
}

fn part_key(p: &fbx::MeshPart) -> (u32, usize, usize, usize, usize, usize, Vec<u32>) {
    (p.index, p.num_faces, p.num_triangles, p.num_empty_faces, p.num_point_faces, p.num_line_faces, p.face_indices.clone())
}

fn cmp_mesh(t: &mut Tally, ctx: &str, c: &ufbx::Mesh, r: &fbx::Mesh) {
    eq(t, ctx, "num_vertices", c.num_vertices, r.num_vertices);
    eq(t, ctx, "num_indices", c.num_indices, r.num_indices);
    eq(t, ctx, "num_faces", c.num_faces, r.num_faces);
    eq(t, ctx, "num_triangles", c.num_triangles, r.num_triangles);
    eq(t, ctx, "num_edges", c.num_edges, r.num_edges);
    eq(t, ctx, "max_face_triangles", c.max_face_triangles, r.max_face_triangles);
    eq(t, ctx, "num_bad_faces", (c.num_empty_faces, c.num_point_faces, c.num_line_faces), (r.num_empty_faces, r.num_point_faces, r.num_line_faces));
    let cf: Vec<_> = c.faces.iter().map(|f| (f.index_begin, f.num_indices)).collect();
    let rf: Vec<_> = r.faces.iter().map(|f| (f.index_begin, f.num_indices)).collect();
    list(t, ctx, "faces", &cf, &rf);
    list(t, ctx, "face_smoothing", &c.face_smoothing, &r.face_smoothing);
    list(t, ctx, "face_material", &c.face_material, &r.face_material);
    list(t, ctx, "face_group", &c.face_group, &r.face_group);
    list(t, ctx, "face_hole", &c.face_hole, &r.face_hole);
    let ce: Vec<_> = c.edges.iter().map(|e| (e.a, e.b)).collect();
    let re: Vec<_> = r.edges.iter().map(|e| (e.a, e.b)).collect();
    list(t, ctx, "edges", &ce, &re);
    list(t, ctx, "edge_smoothing", &c.edge_smoothing, &r.edge_smoothing);
    let cc: Vec<u64> = c.edge_crease.iter().map(|&x| canon(x)).collect();
    let rc: Vec<u64> = r.edge_crease.iter().map(|&x| canon(x)).collect();
    list(t, ctx, "edge_crease", &cc, &rc);
    list(t, ctx, "edge_visibility", &c.edge_visibility, &r.edge_visibility);
    list(t, ctx, "vertex_indices", &c.vertex_indices, &r.vertex_indices);
    let cv: Vec<u64> = c.vertices.iter().flat_map(|v| [v.x, v.y, v.z].map(canon)).collect();
    let rv: Vec<u64> = r.vertices.iter().flat_map(|v| [v.x, v.y, v.z].map(canon)).collect();
    list(t, ctx, "vertices", &cv, &rv);
    list(t, ctx, "vertex_first_index", &c.vertex_first_index, &r.vertex_first_index);

    cmp_attrib!(t, ctx, "vertex_position", c.vertex_position, r.vertex_position, |v| [v.x, v.y, v.z].map(canon));
    cmp_attrib!(t, ctx, "vertex_normal", c.vertex_normal, r.vertex_normal, |v| [v.x, v.y, v.z].map(canon));
    cmp_attrib!(t, ctx, "vertex_uv", c.vertex_uv, r.vertex_uv, |v| [v.x, v.y].map(canon));
    cmp_attrib!(t, ctx, "vertex_tangent", c.vertex_tangent, r.vertex_tangent, |v| [v.x, v.y, v.z].map(canon));
    cmp_attrib!(t, ctx, "vertex_bitangent", c.vertex_bitangent, r.vertex_bitangent, |v| [v.x, v.y, v.z].map(canon));
    cmp_attrib!(t, ctx, "vertex_color", c.vertex_color, r.vertex_color, |v| [v.x, v.y, v.z, v.w].map(canon));
    cmp_attrib!(t, ctx, "vertex_crease", c.vertex_crease, r.vertex_crease, |v| [canon(*v)]);
    cmp_attrib!(t, ctx, "skinned_position", c.skinned_position, r.skinned_position, |v| [v.x, v.y, v.z].map(canon));
    cmp_attrib!(t, ctx, "skinned_normal", c.skinned_normal, r.skinned_normal, |v| [v.x, v.y, v.z].map(canon));
    eq(t, ctx, "skinned_is_local", c.skinned_is_local, r.skinned_is_local);

    eq(t, ctx, "uv_sets.len", c.uv_sets.len(), r.uv_sets.len());
    for (i, (cs, rs)) in c.uv_sets.iter().zip(&r.uv_sets).enumerate() {
        eq(t, ctx, &format!("uv_sets[{i}].index"), cs.index, rs.index);
        cmp_attrib!(t, ctx, format!("uv_sets[{i}].vertex_uv"), cs.vertex_uv, rs.vertex_uv, |v| [v.x, v.y].map(canon));
        cmp_attrib!(t, ctx, format!("uv_sets[{i}].vertex_tangent"), cs.vertex_tangent, rs.vertex_tangent, |v| [v.x, v.y, v.z].map(canon));
        cmp_attrib!(t, ctx, format!("uv_sets[{i}].vertex_bitangent"), cs.vertex_bitangent, rs.vertex_bitangent, |v| [v.x, v.y, v.z].map(canon));
    }
    eq(t, ctx, "color_sets.len", c.color_sets.len(), r.color_sets.len());
    for (i, (cs, rs)) in c.color_sets.iter().zip(&r.color_sets).enumerate() {
        cmp_attrib!(t, ctx, format!("color_sets[{i}].vertex_color"), cs.vertex_color, rs.vertex_color, |v| [v.x, v.y, v.z, v.w].map(canon));
    }

    eq(t, ctx, "materials.len", c.materials.count, r.materials.len());
    let cp: Vec<_> = c.material_parts.iter().map(|p| part_key(&part(p))).collect();
    let rp: Vec<_> = r.material_parts.iter().map(part_key).collect();
    list(t, ctx, "material_parts", &cp, &rp);
    let cp: Vec<_> = c.face_group_parts.iter().map(|p| part_key(&part(p))).collect();
    let rp: Vec<_> = r.face_group_parts.iter().map(part_key).collect();
    list(t, ctx, "face_group_parts", &cp, &rp);
    list(t, ctx, "material_part_usage_order", &c.material_part_usage_order, &r.material_part_usage_order);
    let cg: Vec<i32> = c.face_groups.iter().map(|g| g.id).collect();
    let rg: Vec<i32> = r.face_groups.iter().map(|g| g.id).collect();
    list(t, ctx, "face_groups", &cg, &rg);
    eq(t, ctx, "generated_normals", c.generated_normals, r.generated_normals);
    eq(t, ctx, "subdivision_evaluated", c.subdivision_evaluated, r.subdivision_evaluated);
    eq(t, ctx, "reversed_winding", c.reversed_winding, r.reversed_winding);

    eq(t, ctx, "subdivision_result.is_some", c.subdivision_result.is_some(), r.subdivision_result.is_some());
    if let (Some(cs), Some(rs)) = (c.subdivision_result.as_ref(), r.subdivision_result.as_deref()) {
        let cs = sub_result(cs);
        let rng = |l: &[fbx::SubdivisionWeightRange]| l.iter().map(|r| (r.weight_begin, r.num_weights)).collect::<Vec<_>>();
        let wts = |l: &[fbx::SubdivisionWeight]| l.iter().map(|w| (w.index, canon(w.weight))).collect::<Vec<_>>();
        list(t, ctx, "source_vertex_ranges", &rng(&cs.source_vertex_ranges), &rng(&rs.source_vertex_ranges));
        list(t, ctx, "source_vertex_weights", &wts(&cs.source_vertex_weights), &wts(&rs.source_vertex_weights));
        list(t, ctx, "skin_cluster_ranges", &rng(&cs.skin_cluster_ranges), &rng(&rs.skin_cluster_ranges));
        list(t, ctx, "skin_cluster_weights", &wts(&cs.skin_cluster_weights), &wts(&rs.skin_cluster_weights));
    }
}

fn topo_bits(t: &ufbx::TopoEdge) -> [u32; 7] {
    [t.index, t.next, t.prev, t.twin, t.face, t.edge, flag_bits(t.flags)]
}

fn topo_bits_r(t: &fbx::TopoEdge) -> [u32; 7] {
    [t.index, t.next, t.prev, t.twin, t.face, t.edge, t.flags]
}

type SubOpts = (ufbx::SubdivideOpts, fbx::SubdivideOpts);

fn subdivide_opts(i: usize) -> (SubOpts, &'static [usize]) {
    let mut o: SubOpts = Default::default();
    let levels: &'static [usize] = match i {
        0 => &[1, 2],
        1 => {
            both!(o, interpolate_normals = true);
            both!(o, interpolate_tangents = true);
            both!(o, evaluate_source_vertices = true);
            both!(o, evaluate_skin_weights = true);
            &[1, 2]
        }
        2 => {
            both!(o, boundary = SubdivisionBoundary::SharpCorners);
            both!(o, uv_boundary = SubdivisionBoundary::SharpBoundary);
            both!(o, evaluate_source_vertices = true);
            both!(o, max_source_vertices = 2);
            both!(o, evaluate_skin_weights = true);
            both!(o, max_skin_weights = 2);
            &[2]
        }
        3 => {
            both!(o, boundary = SubdivisionBoundary::SharpInterior);
            both!(o, uv_boundary = SubdivisionBoundary::SharpNone);
            both!(o, ignore_normals = true);
            &[1]
        }
        _ => {
            both!(o, boundary = SubdivisionBoundary::SharpBoundary);
            both!(o, uv_boundary = SubdivisionBoundary::Legacy);
            both!(o, interpolate_normals = true);
            &[1]
        }
    };
    (o, levels)
}

const NUM_SUBDIVIDE_OPTS: usize = 5;

fn face_bucket(n: usize) -> &'static str {
    match n {
        3 => "triangles",
        4 => "quads",
        5..=11 => "faces of 5-11 corners",
        12..=127 => "faces of 12-127 corners",
        _ => "faces of 128+ corners",
    }
}

fn check_mesh(ch: &mut Child, ctx: &str, c: &ufbx::Mesh) {
    let port = OnceCell::new();
    let r = || port.get_or_init(|| to_rs(c));
    ch.step("triangulate+face_normal", |t| {
        t.count("meshes", 1);
        let r = r();
        for (fi, cf) in c.faces.iter().enumerate() {
            let n = cf.num_indices as usize;
            if n < 3 {
                continue;
            }
            t.count(face_bucket(n), 1);
            let rf = r.faces[fi];
            for extra in [0usize, 7] {
                let len = (n - 2) * 3 + extra;
                let mut cb = vec![0u32; len];
                let mut rb = vec![0u32; len];
                let cn = ufbx::triangulate_face(&mut cb, c, *cf);
                let rn = fbx::triangulate_face(&mut rb, r, rf);
                eq(t, ctx, &format!("face {fi} (n {n}, buf {len}) count"), cn, rn);
                list(t, ctx, &format!("face {fi} (n {n}, buf {len}) buffer"), &cb, &rb);
            }
            let cn = ufbx::get_weighted_face_normal(&c.vertex_position, *cf);
            let rn = fbx::get_weighted_face_normal(&r.vertex_position, rf);
            eq(t, ctx, &format!("face {fi} weighted normal"), [cn.x, cn.y, cn.z].map(canon), [rn.x, rn.y, rn.z].map(canon));
        }
    });
    ch.step("find_face_index", |t| {
        if c.num_indices < 4096 {
            for ix in [0, 1, c.num_indices / 2, c.num_indices.saturating_sub(1), c.num_indices, c.num_indices + 1] {
                let ci = unsafe { ufbx::ufbx_find_face_index(c as *const ufbx::Mesh as *mut ufbx::Mesh, ix) };
                eq(t, ctx, &format!("find_face_index({ix})"), ci, fbx::find_face_index(r(), ix));
            }
        }
    });
    let topology = || {
        let mut ct = vec![ufbx::TopoEdge::default(); c.num_indices];
        let mut rt = vec![fbx::TopoEdge::default(); c.num_indices];
        ufbx::compute_topology(c, &mut ct);
        fbx::compute_topology(r(), &mut rt);
        (ct, rt)
    };
    ch.step("compute_topology", |t| {
        let (ct, rt) = topology();
        let cb: Vec<_> = ct.iter().map(topo_bits).collect();
        let rb: Vec<_> = rt.iter().map(topo_bits_r).collect();
        list(t, ctx, "topology", &cb, &rb);
    });
    for assume_smooth in [false, true] {
        ch.step(&format!("normal_mapping+compute_normals(assume_smooth {assume_smooth})"), |t| {
            let (ct, rt) = topology();
            let mut cn = vec![0u32; c.num_indices];
            let mut rn = vec![0u32; c.num_indices];
            let cc = ufbx::generate_normal_mapping(c, &ct, &mut cn, assume_smooth);
            let rc = fbx::generate_normal_mapping(r(), &rt, &mut rn, assume_smooth);
            eq(t, ctx, "count", cc, rc);
            list(t, ctx, "normal_indices", &cn, &rn);
            let mut cv = vec![ufbx::Vec3::default(); cc];
            let mut rv = vec![fbx::Vec3::ZERO; rc];
            ufbx::compute_normals(c, &c.vertex_position, &cn, &mut cv);
            fbx::compute_normals(r(), &r().vertex_position, &rn, &mut rv);
            let cbits: Vec<_> = cv.iter().map(|v| [v.x, v.y, v.z].map(canon)).collect();
            let rbits: Vec<_> = rv.iter().map(|v| [v.x, v.y, v.z].map(canon)).collect();
            list(t, ctx, "normals", &cbits, &rbits);
        });
    }
    let scene = OnceCell::new();
    for oi in 0..NUM_SUBDIVIDE_OPTS {
        let (_, levels) = subdivide_opts(oi);
        for &level in levels {
            let limit = if level == 1 { 40_000 } else { 6_000 };
            if c.num_indices > limit {
                continue;
            }
            ch.step(&format!("subdivide(opts {oi}, level {level})"), |t| {
                let ((copts, ropts), _) = subdivide_opts(oi);
                let rres = fbx::subdivide_mesh(scene.get_or_init(|| scene_for(c)), r(), level, &ropts);
                let mut guarded = Guarded::default();
                let copts = ufbx::SubdivideOpts { temp_allocator: guarded.opts(), ..copts };
                let cres = t.known_c(C_DIED, ctx, || ufbx::subdivide_mesh(c, level, copts));
                match (cres, rres) {
                    (Ok(cm), Ok(rm)) => cmp_mesh(t, ctx, &cm, &rm),
                    (Err(ce), Err(re)) => eq(t, ctx, "error", ce.description.to_string(), re.description.clone()),
                    (cm, rm) => {
                        t.check(false, || format!("{ctx}: C {:?}, port {:?}", cm.err().map(|e| e.description.to_string()), rm.err().map(|e| e.description)));
                    }
                }
                // the write past the buffer lands in a guard, so C's result is often still right: compared, and only
                // where it differs is it C's failure
                if guarded.overrun() {
                    t.count("C subdivisions that wrote past a freed buffer", 1);
                    if t.diffs > 0 {
                        (t.checks, t.diffs) = (0, 0);
                        t.kept.clear();
                        t.known_failure(&format!("{KNOWN_C_FAILURE}: C wrote past a freed buffer, and its result differs"), ctx);
                    }
                }
            });
        }
    }
}

#[allow(clippy::approx_constant)] // 6.28 as the subdivide worker drew them, so the polygons stay the same
fn random_obj(seed: u64) -> String {
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut rnd = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let mut text = String::new();
    let mut base = 1usize;
    let sizes = [5usize, 6, 7, 8, 11, 12, 13, 20, 64, 127, 128, 200, 300, 600];
    for (k, &n) in sizes.iter().enumerate() {
        let kind = (seed as usize + k) % 5;
        let (ax, ay, az) = (rnd() * 6.28, rnd() * 6.28, rnd() * 6.28);
        let mut pts: Vec<[f64; 3]> = (0..n)
            .map(|i| {
                let a = i as f64 / n as f64 * std::f64::consts::TAU;
                let rad = match kind {
                    0 => 0.3 + rnd(),
                    1 => {
                        if i % 2 == 0 {
                            1.0
                        } else {
                            0.2 + 0.1 * rnd()
                        }
                    }
                    _ => 1.0 + 0.05 * rnd(),
                };
                let z = if kind == 2 { 0.3 * (rnd() - 0.5) } else { 0.0 };
                [rad * a.cos(), rad * a.sin(), z]
            })
            .collect();
        if kind == 3 {
            for i in (1..n).rev() {
                let j = (rnd() * (i + 1) as f64) as usize;
                pts.swap(i, j.min(i));
            }
        }
        if kind == 4 {
            for i in (0..n).step_by(3) {
                let j = (i + 1) % n;
                pts[j] = pts[i];
            }
        }
        for p in &pts {
            let (x, y, z) = (p[0], p[1], p[2]);
            let (y, z) = (y * ax.cos() - z * ax.sin(), y * ax.sin() + z * ax.cos());
            let (x, z) = (x * ay.cos() + z * ay.sin(), -x * ay.sin() + z * ay.cos());
            let (x, y) = (x * az.cos() - y * az.sin(), x * az.sin() + y * az.cos());
            text.push_str(&format!("v {x:e} {y:e} {z:e}\n"));
        }
        text.push('f');
        for i in 0..n {
            text.push_str(&format!(" {}", base + i));
        }
        text.push('\n');
        base += n;
    }
    text
}

fn jobs(a: &Args) -> Vec<Job> {
    let mut v: Vec<Job> = (0..a.random).map(|s| Job::generated(format!("gen:random:{s}"))).collect();
    v.extend(corpus_jobs(a, true, &["fbx", "obj"]));
    v
}

fn run(_: &Args, item: &str, ch: &mut Child) {
    if let Some(seed) = item.strip_prefix("gen:random:") {
        let text = random_obj(seed.parse().expect("a seed"));
        let scene =
            ufbx::load_memory(text.as_bytes(), ufbx::LoadOpts { file_format: ufbx::FileFormat::Obj, ..Default::default() }).expect("the random .obj loads");
        for (mi, mesh) in scene.meshes.iter().enumerate() {
            check_mesh(ch, &format!("mesh {mi}"), mesh);
        }
        return;
    }
    for gen_normals in [false, true] {
        let Ok(scene) = ufbx::load_file(item, ufbx::LoadOpts { generate_missing_normals: gen_normals, ..Default::default() }) else { continue };
        for (mi, mesh) in scene.meshes.iter().enumerate() {
            check_mesh(ch, &format!("mesh {mi} (gen_normals {gen_normals})"), mesh);
        }
    }
}

pub const SUBDIVIDE: Section = Section { name: "subdivide", jobs, run, header: None };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_block_grows_in_place_and_a_write_past_a_moved_one_is_found() {
        let mut g = Guarded::default();
        let small = g.alloc(32 * 16);
        assert_eq!(g.realloc(small, 64 * 16), small);
        assert!(!g.overrun());
        let big = g.alloc(1 << 20);
        let moved = g.realloc(big, 2 << 20);
        assert_ne!(moved, big);
        assert!(!g.overrun());
        unsafe { big.add(1 << 20).write(0) };
        assert!(g.overrun());
    }
}

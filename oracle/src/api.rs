use std::mem::{MaybeUninit, offset_of, size_of};

use load3d as rs;

use crate::common::{Rng, element_type, matrix, same, same_mat, same2, same3, transform, v3, v4};
use crate::runner::{Args, Child, Job, Section, Tally, corpus_jobs};

#[repr(C)]
struct RawList<T> {
    data: *const T,
    count: usize,
}

/// `ufbx::NurbsBasis` with its lists as raw pointers, so a synthetic one can point at Rust vectors
#[repr(C)]
struct RawBasis {
    order: u32,
    topology: u32,
    knot_vector: RawList<f64>,
    t_min: f64,
    t_max: f64,
    spans: RawList<f64>,
    is_2d: bool,
    num_wrap_control_points: usize,
    valid: bool,
}

const _: () = {
    assert!(size_of::<RawBasis>() == size_of::<ufbx::NurbsBasis>());
    assert!(offset_of!(RawBasis, t_min) == offset_of!(ufbx::NurbsBasis, t_min));
    assert!(offset_of!(RawBasis, spans) == offset_of!(ufbx::NurbsBasis, spans));
    assert!(offset_of!(RawBasis, num_wrap_control_points) == offset_of!(ufbx::NurbsBasis, num_wrap_control_points));
    assert!(offset_of!(RawBasis, valid) == offset_of!(ufbx::NurbsBasis, valid));
};

fn topology(t: u32) -> rs::NurbsTopology {
    match t {
        0 => rs::NurbsTopology::Open,
        1 => rs::NurbsTopology::Periodic,
        _ => rs::NurbsTopology::Closed,
    }
}

fn basis(b: &ufbx::NurbsBasis) -> rs::NurbsBasis {
    rs::NurbsBasis {
        order: b.order,
        topology: topology(b.topology as u32),
        knot_vector: b.knot_vector.to_vec(),
        t_min: b.t_min,
        t_max: b.t_max,
        spans: b.spans.to_vec(),
        is_2d: b.is_2d,
        num_wrap_control_points: b.num_wrap_control_points,
        valid: b.valid,
    }
}

fn points(p: &[ufbx::Vec4]) -> Vec<rs::Vec4> {
    p.iter().map(v4).collect()
}

fn curve(c: &ufbx::NurbsCurve) -> rs::NurbsCurve {
    rs::NurbsCurve { basis: basis(&c.basis), control_points: points(&c.control_points), ..Default::default() }
}

fn surface(s: &ufbx::NurbsSurface) -> rs::NurbsSurface {
    rs::NurbsSurface {
        basis_u: basis(&s.basis_u),
        basis_v: basis(&s.basis_v),
        num_control_points_u: s.num_control_points_u,
        num_control_points_v: s.num_control_points_v,
        control_points: points(&s.control_points),
        span_subdivision_u: s.span_subdivision_u,
        span_subdivision_v: s.span_subdivision_v,
        flip_normals: s.flip_normals,
        material: s.material.as_ref().map(|m| m.element.typed_id as usize),
        ..Default::default()
    }
}

struct SynthBasis {
    order: u32,
    topology: u32,
    knots: Vec<f64>,
    spans: Vec<f64>,
    t_min: f64,
    t_max: f64,
}

impl SynthBasis {
    fn random(rng: &mut Rng, max_order: u64, max_extra: u64) -> SynthBasis {
        let order = 2 + rng.below(max_order - 1) as u32;
        let degree = order as usize - 1;
        let count = 2 * degree + 1 + rng.below(max_extra) as usize;
        let mut knots = Vec::with_capacity(count);
        let mut t = rng.real() * 4.0 - 2.0;
        for i in 0..count {
            let clamp = rng.below(2) == 0 && (i < degree || i >= count - degree);
            if i > 0 && !(clamp || rng.below(5) == 0) {
                t += 0.05 + rng.real() * 2.0;
            }
            knots.push(t);
        }
        let t_min = knots[degree];
        let t_max = knots[count - degree - 1];
        let mut spans = Vec::new();
        let mut prev = f64::NEG_INFINITY;
        for &t in &knots[degree..degree + count - 2 * degree] {
            if t != prev {
                spans.push(t);
                prev = t;
            }
        }
        SynthBasis { order, topology: rng.below(3) as u32, knots, spans, t_min, t_max }
    }

    fn raw(&self) -> RawBasis {
        RawBasis {
            order: self.order,
            topology: self.topology,
            knot_vector: RawList { data: self.knots.as_ptr(), count: self.knots.len() },
            t_min: self.t_min,
            t_max: self.t_max,
            spans: RawList { data: self.spans.as_ptr(), count: self.spans.len() },
            is_2d: false,
            num_wrap_control_points: 0,
            valid: true,
        }
    }

    fn rs(&self) -> rs::NurbsBasis {
        rs::NurbsBasis {
            order: self.order,
            topology: topology(self.topology),
            knot_vector: self.knots.clone(),
            t_min: self.t_min,
            t_max: self.t_max,
            spans: self.spans.clone(),
            valid: true,
            ..Default::default()
        }
    }

    fn samples(&self, rng: &mut Rng) -> Vec<f64> {
        let mut us = vec![self.t_min - 0.5, self.t_min, self.t_max, self.t_max + 1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
        us.extend(self.knots.iter().copied());
        us.extend(self.spans.windows(2).map(|w| 0.5 * (w[0] + w[1])));
        for _ in 0..8 {
            us.push(self.t_min + (self.t_max - self.t_min) * rng.real());
        }
        us
    }
}

fn random_points(rng: &mut Rng, n: usize) -> Vec<ufbx::Vec4> {
    (0..n).map(|_| ufbx::Vec4 { x: rng.real() * 10.0 - 5.0, y: rng.real() * 10.0 - 5.0, z: rng.real() * 10.0 - 5.0, w: 0.2 + rng.real() * 1.8 }).collect()
}

fn check_basis(t: &mut Tally, what: &str, c: &ufbx::NurbsBasis, r: &rs::NurbsBasis, us: &[f64]) {
    let order = c.order as usize;
    for &u in us {
        for nd in [0, 1, order.saturating_sub(1), order] {
            for nw in [order, order.saturating_sub(1)] {
                let (mut cw, mut cd) = (vec![7.0; nw], vec![7.0; nd]);
                let (mut rw, mut rd) = (cw.clone(), cd.clone());
                let cb = ufbx::evaluate_nurbs_basis(c, u, &mut cw, &mut cd);
                let rb = rs::evaluate_nurbs_basis(r, u, &mut rw, &mut rd);
                let ok = cb == rb && cw.iter().zip(&rw).all(|(a, b)| same(*a, *b)) && cd.iter().zip(&rd).all(|(a, b)| same(*a, *b));
                t.check(ok, || format!("{what} basis u={u} nw={nw} nd={nd}: C {cb} {cw:?} {cd:?}, port {rb} {rw:?} {rd:?}"));
            }
        }
    }
}

fn check_curve(t: &mut Tally, what: &str, c: &ufbx::NurbsCurve, r: &rs::NurbsCurve, us: &[f64]) {
    check_basis(t, what, &c.basis, &r.basis, us);
    for &u in us {
        let cp = ufbx::evaluate_nurbs_curve(c, u);
        let rp = rs::evaluate_nurbs_curve(r, u);
        let ok = cp.valid == rp.valid && same3(&cp.position, &rp.position) && same3(&cp.derivative, &rp.derivative);
        t.check(ok, || format!("{what} curve u={u}: C {:?} {:?}, port {rp:?}", cp.position, cp.derivative));
    }
    for sub in [0, 1, 2, 3, 5] {
        let cl = ufbx::tessellate_nurbs_curve(c, ufbx::TessellateCurveOpts { span_subdivision: sub, ..Default::default() });
        let rl = rs::tessellate_nurbs_curve(r, &rs::TessellateCurveOpts { span_subdivision: sub });
        match (&cl, &rl) {
            (Ok(cl), Ok(rl)) => {
                t.count("curve tessellations, both ok", 1);
                let ok = cl.control_points.len() == rl.control_points.len()
                    && cl.control_points.iter().zip(&rl.control_points).all(|(a, b)| same3(a, b))
                    && *cl.point_indices == rl.point_indices[..]
                    && cl.segments.len() == rl.segments.len()
                    && cl.segments.iter().zip(&rl.segments).all(|(a, b)| a.index_begin == b.index_begin && a.num_indices == b.num_indices)
                    && same3(&cl.color, &rl.color)
                    && cl.from_tessellated_nurbs == rl.from_tessellated_nurbs
                    && cl.element.element_id == rl.element.element_id
                    && cl.element.typed_id == rl.element.typed_id
                    && element_type(cl.element.type_) == rl.element.element_type;
                t.check(ok, || format!("{what} tessellated curve sub={sub}: {} / {} points", cl.control_points.len(), rl.control_points.len()));
            }
            (Err(ce), Err(re)) => {
                t.count("curve tessellations, both failed", 1);
                t.check(ce.type_ as u32 == re.kind as u32 && ce.description.as_ref() == re.description, || {
                    format!("{what} curve error sub={sub}: C {:?} {}, port {:?} {}", ce.type_, ce.description, re.kind, re.description)
                });
            }
            _ => {
                t.check(false, || format!("{what} curve sub={sub}: C ok {}, port ok {}", cl.is_ok(), rl.is_ok()));
            }
        }
    }
}

fn check_attrib3(t: &mut Tally, what: &str, c: &ufbx::VertexVec3, r: &rs::VertexVec3, exact_values: bool) {
    let mut ok = c.exists == r.exists && *c.indices == r.indices[..] && c.value_reals == r.value_reals && c.unique_per_vertex == r.unique_per_vertex;
    if exact_values {
        ok &= c.values.len() == r.values.len() && c.values.iter().zip(&r.values).all(|(a, b)| same3(a, b));
    } else {
        // ufbx counts `dst_index` values though it fills one per grid point; read the filled ones past that count
        ok &= c.indices.iter().all(|&i| r.values.get(i as usize).is_some_and(|rv| same3(unsafe { &*c.values.data.add(i as usize) }, rv)));
    }
    t.check(ok, || format!("{what}: C {} values {} indices, port {} values {} indices", c.values.len(), c.indices.len(), r.values.len(), r.indices.len()));
}

fn check_mesh(t: &mut Tally, what: &str, c: &ufbx::Mesh, r: &rs::Mesh) {
    let counts = |m: &ufbx::Mesh| {
        [m.num_vertices, m.num_indices, m.num_faces, m.num_triangles, m.max_face_triangles, m.num_empty_faces, m.num_point_faces, m.num_line_faces]
    };
    let rcounts = [r.num_vertices, r.num_indices, r.num_faces, r.num_triangles, r.max_face_triangles, r.num_empty_faces, r.num_point_faces, r.num_line_faces];
    t.check(counts(c) == rcounts, || format!("{what} counts: C {:?}, port {rcounts:?}", counts(c)));
    let faces_ok =
        c.faces.len() == r.faces.len() && c.faces.iter().zip(&r.faces).all(|(a, b)| a.index_begin == b.index_begin && a.num_indices == b.num_indices);
    t.check(faces_ok, || format!("{what} faces"));
    t.check(*c.vertex_indices == r.vertex_indices[..], || format!("{what} vertex_indices"));
    t.check(c.vertices.len() == r.vertices.len() && c.vertices.iter().zip(&r.vertices).all(|(a, b)| same3(a, b)), || format!("{what} vertices"));
    t.check(*c.vertex_first_index == r.vertex_first_index[..], || format!("{what} vertex_first_index"));
    check_attrib3(t, &format!("{what} vertex_position"), &c.vertex_position, &r.vertex_position, true);
    check_attrib3(t, &format!("{what} vertex_normal"), &c.vertex_normal, &r.vertex_normal, true);
    check_attrib3(t, &format!("{what} vertex_tangent"), &c.vertex_tangent, &r.vertex_tangent, false);
    check_attrib3(t, &format!("{what} vertex_bitangent"), &c.vertex_bitangent, &r.vertex_bitangent, false);
    check_attrib3(t, &format!("{what} skinned_position"), &c.skinned_position, &r.skinned_position, true);
    check_attrib3(t, &format!("{what} skinned_normal"), &c.skinned_normal, &r.skinned_normal, true);
    let (cu, ru) = (&c.vertex_uv, &r.vertex_uv);
    let uv_ok = cu.exists == ru.exists
        && *cu.indices == ru.indices[..]
        && cu.value_reals == ru.value_reals
        && cu.indices.iter().all(|&i| ru.values.get(i as usize).is_some_and(|rv| same2(unsafe { &*cu.values.data.add(i as usize) }, rv)));
    t.check(uv_ok, || format!("{what} vertex_uv"));
    t.check(
        c.uv_sets.len() == r.uv_sets.len()
            && c.uv_sets.iter().zip(&r.uv_sets).all(|(a, b)| *a.vertex_uv.indices == b.vertex_uv.indices[..] && a.index == b.index),
        || format!("{what} uv_sets"),
    );
    t.check(c.skinned_is_local == r.skinned_is_local, || format!("{what} skinned_is_local"));
    let mats: Vec<Option<usize>> = c.materials.iter().map(|m| Some(m.element.typed_id as usize)).collect();
    t.check(mats == r.materials, || format!("{what} materials C {mats:?}, port {:?}", r.materials));
    t.check(*c.face_material == r.face_material[..], || format!("{what} face_material"));
    let parts_ok = c.material_parts.len() == r.material_parts.len()
        && c.material_parts.iter().zip(&r.material_parts).all(|(a, b)| {
            a.index == b.index
                && a.num_faces == b.num_faces
                && a.num_triangles == b.num_triangles
                && a.num_empty_faces == b.num_empty_faces
                && a.num_point_faces == b.num_point_faces
                && a.num_line_faces == b.num_line_faces
                && *a.face_indices == b.face_indices[..]
        });
    t.check(parts_ok, || format!("{what} material_parts"));
    t.check(*c.material_part_usage_order == r.material_part_usage_order[..], || format!("{what} material_part_usage_order"));
    let flags = |m: &ufbx::Mesh| (m.generated_normals, m.subdivision_evaluated, m.from_tessellated_nurbs, m.element.element_id, m.element.typed_id);
    let rflags = (r.generated_normals, r.subdivision_evaluated, r.from_tessellated_nurbs, r.element.element_id, r.element.typed_id);
    t.check(flags(c) == rflags && element_type(c.element.type_) == r.element.element_type, || format!("{what} flags C {:?}, port {rflags:?}", flags(c)));
}

fn check_surface(t: &mut Tally, what: &str, c: &ufbx::NurbsSurface, r: &rs::NurbsSurface, us: &[f64], vs: &[f64], subs: &[(usize, usize, bool)]) {
    check_basis(t, &format!("{what} u"), &c.basis_u, &r.basis_u, us);
    check_basis(t, &format!("{what} v"), &c.basis_v, &r.basis_v, vs);
    for &u in us {
        for &v in vs {
            let cp = ufbx::evaluate_nurbs_surface(c, u, v);
            let rp = rs::evaluate_nurbs_surface(r, u, v);
            let ok = cp.valid == rp.valid
                && same3(&cp.position, &rp.position)
                && same3(&cp.derivative_u, &rp.derivative_u)
                && same3(&cp.derivative_v, &rp.derivative_v);
            t.check(ok, || format!("{what} surface ({u}, {v}): C {:?}, port {rp:?}", cp.position));
        }
    }
    for &(su, sv, skip) in subs {
        let co = ufbx::TessellateSurfaceOpts { span_subdivision_u: su, span_subdivision_v: sv, skip_mesh_parts: skip, ..Default::default() };
        let ro = rs::TessellateSurfaceOpts { span_subdivision_u: su, span_subdivision_v: sv, skip_mesh_parts: skip };
        match (ufbx::tessellate_nurbs_surface(c, co), rs::tessellate_nurbs_surface(r, &ro)) {
            (Ok(cm), Ok(rm)) => {
                t.count("surface tessellations, both ok", 1);
                check_mesh(t, &format!("{what} mesh {su}x{sv}{}", if skip { " skip" } else { "" }), &cm, &rm)
            }
            (Err(ce), Err(re)) => {
                t.check(ce.type_ as u32 == re.kind as u32 && ce.description.as_ref() == re.description, || {
                    format!("{what} surface error: C {:?} {}, port {:?} {}", ce.type_, ce.description, re.kind, re.description)
                });
            }
            (cm, rm) => {
                t.check(false, || format!("{what} surface {su}x{sv}: C ok {}, port ok {}", cm.is_ok(), rm.is_ok()));
            }
        }
    }
}

/// Curves with random data whose element header is copied from a corpus curve, so that tessellation can retain that
/// curve's scene
fn synthetic_curves(t: &mut Tally, template: &ufbx::NurbsCurve, rng: &mut Rng, count: usize) {
    for n in 0..count {
        let b = SynthBasis::random(rng, 6, 10);
        let needed = b.knots.len() - b.order as usize;
        let num_points = 1 + rng.below(needed as u64 + 2) as usize;
        let points = random_points(rng, num_points);
        let mut raw = MaybeUninit::<ufbx::NurbsCurve>::zeroed();
        let c = unsafe {
            let p = raw.as_mut_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(&template.element as *const ufbx::Element as *const u8, p, size_of::<ufbx::Element>());
            (p.add(offset_of!(ufbx::NurbsCurve, basis)) as *mut RawBasis).write(b.raw());
            (p.add(offset_of!(ufbx::NurbsCurve, control_points)) as *mut RawList<ufbx::Vec4>).write(RawList { data: points.as_ptr(), count: points.len() });
            &*raw.as_ptr()
        };
        let r = rs::NurbsCurve { basis: b.rs(), control_points: self::points(&points), ..Default::default() };
        let us = b.samples(rng);
        check_curve(t, &format!("synthetic curve {n} (order {}, {} knots, {} points)", b.order, b.knots.len(), num_points), c, &r, &us);
    }
}

fn synthetic_surfaces(t: &mut Tally, template: &ufbx::NurbsSurface, rng: &mut Rng, count: usize) {
    for n in 0..count {
        // ufbx asserts on a surface basis of one span (`nb_ix < ix`)
        let mut spanned = || loop {
            let b = SynthBasis::random(rng, 4, 5);
            if b.spans.len() >= 2 {
                break b;
            }
        };
        let (bu, bv) = (spanned(), spanned());
        let nu = 1 + rng.below((bu.knots.len() - bu.order as usize) as u64 + 1) as usize;
        let nv = 1 + rng.below((bv.knots.len() - bv.order as usize) as u64 + 1) as usize;
        let mut points = random_points(rng, nu * nv);
        if rng.below(3) == 0 {
            for p in points.iter_mut().take(nu) {
                *p = ufbx::Vec4 { x: 1.0, y: 2.0, z: 3.0, w: 1.0 };
            }
        }
        let flip = rng.below(2) == 0;
        let mut raw = MaybeUninit::<ufbx::NurbsSurface>::zeroed();
        let c = unsafe {
            let p = raw.as_mut_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(&template.element as *const ufbx::Element as *const u8, p, size_of::<ufbx::Element>());
            (p.add(offset_of!(ufbx::NurbsSurface, basis_u)) as *mut RawBasis).write(bu.raw());
            (p.add(offset_of!(ufbx::NurbsSurface, basis_v)) as *mut RawBasis).write(bv.raw());
            (p.add(offset_of!(ufbx::NurbsSurface, num_control_points_u)) as *mut usize).write(nu);
            (p.add(offset_of!(ufbx::NurbsSurface, num_control_points_v)) as *mut usize).write(nv);
            (p.add(offset_of!(ufbx::NurbsSurface, control_points)) as *mut RawList<ufbx::Vec4>).write(RawList { data: points.as_ptr(), count: points.len() });
            (p.add(offset_of!(ufbx::NurbsSurface, flip_normals)) as *mut bool).write(flip);
            &*raw.as_ptr()
        };
        let r = rs::NurbsSurface {
            basis_u: bu.rs(),
            basis_v: bv.rs(),
            num_control_points_u: nu,
            num_control_points_v: nv,
            control_points: self::points(&points),
            flip_normals: flip,
            ..Default::default()
        };
        let (mut us, mut vs) = (bu.samples(rng), bv.samples(rng));
        us.truncate(12);
        vs.truncate(12);
        let subs = [(0, 0, false), (1, 1, true), (rng.below(4) as usize + 1, rng.below(4) as usize + 1, rng.below(2) == 0)];
        check_surface(t, &format!("synthetic surface {n} ({nu}x{nv} points)"), c, &r, &us, &vs, &subs);
    }
}

fn check_generate_indices(t: &mut Tally, rng: &mut Rng, rounds: usize) {
    fn stream<const N: usize>(data: &mut [u8]) -> ufbx::VertexStream<'_> {
        ufbx::VertexStream::new(data.as_chunks_mut::<N>().0)
    }
    for round in 0..rounds {
        let num_indices = rng.below(600) as usize;
        let pool = 1 + rng.below(80);
        let num_streams = 1 + rng.below(4) as usize;
        let sizes: Vec<usize> = (0..num_streams).map(|_| [1, 2, 3, 4, 7, 8, 12, 16, 24][rng.below(9) as usize]).collect();
        let ids: Vec<u64> = (0..num_indices).map(|_| rng.below(pool)).collect();
        let mut datas: Vec<Vec<u8>> = sizes
            .iter()
            .enumerate()
            .map(|(si, &size)| {
                let extra = rng.below(3) as usize;
                let mut d = Vec::with_capacity((num_indices + extra) * size);
                for i in 0..num_indices + extra {
                    let id = ids.get(i).copied().unwrap_or(999) ^ if rng.below(20) == 0 { 0x55 } else { 0 };
                    for b in 0..size {
                        d.push((id.wrapping_mul(2654435761).wrapping_add((si * 31 + b) as u64) >> (b % 5)) as u8);
                    }
                }
                d
            })
            .collect();
        let mut rdatas = datas.clone();

        let mut cidx = vec![0u32; num_indices];
        let cres = {
            let mut streams: Vec<ufbx::VertexStream> = datas
                .iter_mut()
                .zip(&sizes)
                .map(|(d, &size)| match size {
                    1 => stream::<1>(d),
                    2 => stream::<2>(d),
                    3 => stream::<3>(d),
                    4 => stream::<4>(d),
                    7 => stream::<7>(d),
                    8 => stream::<8>(d),
                    12 => stream::<12>(d),
                    16 => stream::<16>(d),
                    _ => stream::<24>(d),
                })
                .collect();
            ufbx::generate_indices(&mut streams, &mut cidx, ufbx::AllocatorOpts::default())
        };
        let mut ridx = vec![0u32; num_indices];
        let rres = {
            let mut streams: Vec<rs::VertexStream> =
                rdatas.iter_mut().zip(&sizes).map(|(d, &size)| rs::VertexStream { vertex_count: d.len() / size, vertex_size: size, data: d }).collect();
            rs::generate_indices(&mut streams, &mut ridx)
        };
        let ok = match (&cres, &rres) {
            (Ok(a), Ok(b)) => a == b && cidx == ridx && datas == rdatas,
            (Err(_), Err(_)) => true,
            _ => false,
        };
        t.check(ok, || format!("generate_indices round {round}: sizes {sizes:?}, C {:?}, port {:?}", cres.as_ref().ok(), rres.as_ref().ok()));
    }
}

fn named_scene(scene: &ufbx::Scene) -> rs::Scene {
    rs::Scene {
        elements: scene.elements.iter().map(|e| rs::ElementRef { element_type: element_type(e.type_), index: e.typed_id as usize }).collect(),
        elements_by_name: scene
            .elements_by_name
            .iter()
            .map(|n| rs::NameElement {
                name: n.name.to_string(),
                element_type: element_type(n.type_),
                internal_key: rs::props::name_key(n.name.as_bytes()),
                element: n.element.element_id,
            })
            .collect(),
        ..Default::default()
    }
}

fn check_find(t: &mut Tally, name: &str, scene: &ufbx::Scene) {
    let rscene = named_scene(scene);
    for n in scene.elements_by_name.iter() {
        t.count("named elements looked up", 1);
        for (query, ty) in [(n.name.to_string(), n.type_), (format!("{}_x", n.name), n.type_), (n.name.to_string(), ufbx::ElementType::Node)] {
            let c = ufbx::find_element(scene, ty, &query).map(|e| e.element_id);
            let r = rs::find_element(&rscene, element_type(ty), &query);
            t.check(c == r, || format!("{name} find_element {ty:?} {query:?}: C {c:?}, port {r:?}"));
        }
        let c = ufbx::find_node(scene, &n.name).map(|e| e.element.typed_id as usize);
        t.check(c == rs::find_node(&rscene, &n.name), || format!("{name} find_node {}", n.name));
        let c = ufbx::find_material(scene, &n.name).map(|e| e.element.typed_id as usize);
        t.check(c == rs::find_material(&rscene, &n.name), || format!("{name} find_material {}", n.name));
        let c = ufbx::find_anim_stack(scene, &n.name).map(|e| e.element.typed_id as usize);
        t.check(c == rs::find_anim_stack(&rscene, &n.name), || format!("{name} find_anim_stack {}", n.name));
    }
}

fn check_anim_props(t: &mut Tally, name: &str, scene: &ufbx::Scene) {
    for layer in scene.anim_layers.iter() {
        let mut props: Vec<rs::AnimProp> = layer
            .anim_props
            .iter()
            .map(|p| rs::AnimProp {
                element: p.element.element_id,
                internal_key: rs::props::name_key(p.prop_name.as_bytes()),
                prop_name: p.prop_name.to_string(),
                anim_value: p.anim_value.element.typed_id as usize,
            })
            .collect();
        let in_id_order = props.windows(2).all(|w| w[0].element <= w[1].element);
        t.check(in_id_order, || format!("{name}: C's anim props are not in element id order"));
        props.sort_by(|a, b| (a.element, a.internal_key, a.prop_name.as_bytes()).cmp(&(b.element, b.internal_key, b.prop_name.as_bytes())));
        let rlayer = rs::AnimLayer { anim_props: props, ..Default::default() };
        let mut ids: Vec<u32> = layer.anim_props.iter().map(|p| p.element.element_id).collect();
        ids.dedup();
        ids.push(0);
        for id in ids {
            t.count("anim prop element lookups", 1);
            let element = &scene.elements[id as usize];
            let key = |p: &str, v: usize| (p.to_string(), v);
            let c: Vec<_> = ufbx::find_anim_props(layer, element).iter().map(|p| key(&p.prop_name, p.anim_value.element.typed_id as usize)).collect();
            let r: Vec<_> = rs::find_anim_props(&rlayer, id).iter().map(|p| key(&p.prop_name, p.anim_value)).collect();
            t.check(c == r, || format!("{name} find_anim_props {id}: C {c:?}, port {r:?}"));
            for (prop, _) in c.iter().chain([&("Lcl Nothing".to_string(), 0)]) {
                let cp = ufbx::find_anim_prop(layer, element, prop).map(|p| p.anim_value.element.typed_id as usize);
                let rp = rs::find_anim_prop(&rlayer, id, prop).map(|p| p.anim_value);
                t.check(cp == rp, || format!("{name} find_anim_prop {id} {prop}: C {cp:?}, port {rp:?}"));
            }
        }
    }
}

fn check_normal_matrices(t: &mut Tally, name: &str, scene: &ufbx::Scene) {
    for node in scene.nodes.iter() {
        t.count("node normal matrices", 1);
        let rnode = rs::Node { geometry_transform: transform(&node.geometry_transform), node_to_world: matrix(&node.node_to_world), ..Default::default() };
        let c = ufbx::get_compatible_matrix_for_normals(node);
        t.check(same_mat(&c, &rs::get_compatible_matrix_for_normals(&rnode)), || format!("{name} normal matrix of {}", node.element.name));
    }
}

fn check_skins(t: &mut Tally, name: &str, scene: &ufbx::Scene) {
    let skin_clusters = scene
        .skin_clusters
        .iter()
        .map(|c| rs::SkinCluster {
            bone_node: c.bone_node.as_ref().map(|n| n.element.typed_id as usize),
            geometry_to_world: matrix(&c.geometry_to_world),
            geometry_to_world_transform: transform(&c.geometry_to_world_transform),
            ..Default::default()
        })
        .collect();
    let rscene = rs::Scene { skin_clusters, ..Default::default() };
    for skin in scene.skin_deformers.iter() {
        let rskin = rs::SkinDeformer {
            clusters: skin.clusters.iter().map(|c| c.element.typed_id as usize).collect(),
            vertices: skin
                .vertices
                .iter()
                .map(|v| rs::SkinVertex { weight_begin: v.weight_begin, num_weights: v.num_weights, dq_weight: v.dq_weight })
                .collect(),
            weights: skin.weights.iter().map(|w| rs::SkinWeight { cluster_index: w.cluster_index, weight: w.weight }).collect(),
            ..Default::default()
        };
        let fallback = ufbx::Matrix { m00: 2.0, m11: 3.0, m22: 4.0, m03: 1.0, ..Default::default() };
        let rfallback = matrix(&fallback);
        for v in 0..skin.vertices.len() {
            t.count(if skin.vertices[v].dq_weight > 0.0 { "skin vertices with dual quaternion weight" } else { "skin vertices, linear" }, 1);
            let c = ufbx::get_skin_vertex_matrix(skin, v, &fallback);
            let r = rs::get_skin_vertex_matrix(&rscene, &rskin, v, Some(&rfallback));
            t.check(same_mat(&c, &r), || format!("{name} skin {} vertex {v}: C {c:?}, port {r:?}", skin.element.name));
        }
    }
}

fn check_blends(t: &mut Tally, name: &str, scene: &ufbx::Scene, rng: &mut Rng) {
    let blend_shapes = scene
        .blend_shapes
        .iter()
        .map(|s| rs::BlendShape {
            num_offsets: s.num_offsets,
            offset_vertices: s.offset_vertices.to_vec(),
            position_offsets: s.position_offsets.iter().map(v3).collect(),
            offset_weights: s.offset_weights.to_vec(),
            ..Default::default()
        })
        .collect();
    let blend_channels = scene
        .blend_channels
        .iter()
        .map(|c| rs::BlendChannel {
            keyframes: c
                .keyframes
                .iter()
                .map(|k| rs::BlendKeyframe { shape: k.shape.element.typed_id as usize, target_weight: k.target_weight, effective_weight: k.effective_weight })
                .collect(),
            ..Default::default()
        })
        .collect();
    let rscene = rs::Scene { blend_shapes, blend_channels, ..Default::default() };
    for shape in scene.blend_shapes.iter() {
        let rshape = &rscene.blend_shapes[shape.element.typed_id as usize];
        let max_vertex = shape.offset_vertices.iter().copied().max().unwrap_or(0) as usize;
        for v in 0..max_vertex + 2 {
            let c = ufbx::get_blend_shape_vertex_offset(shape, v);
            t.check(same3(&c, &rs::get_blend_shape_vertex_offset(rshape, v)), || format!("{name} shape offset {v}"));
        }
        let len = max_vertex.saturating_sub(rng.below(3) as usize) + 1;
        let weight = rng.real() * 2.0 - 0.5;
        let mut cv: Vec<ufbx::Vec3> = (0..len).map(|i| ufbx::Vec3 { x: i as f64, y: 0.5, z: -1.0 }).collect();
        let mut rv: Vec<rs::Vec3> = cv.iter().map(v3).collect();
        ufbx::add_blend_shape_vertex_offsets(shape, &mut cv, weight);
        rs::add_blend_shape_vertex_offsets(rshape, &mut rv, weight);
        t.check(cv.iter().zip(&rv).all(|(a, b)| same3(a, b)), || format!("{name} add shape offsets"));
    }
    for blend in scene.blend_deformers.iter() {
        t.count("blend deformers", 1);
        let rblend = rs::BlendDeformer { channels: blend.channels.iter().map(|c| c.element.typed_id as usize).collect(), ..Default::default() };
        let max_vertex = scene.blend_shapes.iter().flat_map(|s| s.offset_vertices.iter().copied()).max().unwrap_or(0) as usize;
        for v in 0..max_vertex + 2 {
            let c = ufbx::get_blend_vertex_offset(blend, v);
            t.check(same3(&c, &rs::get_blend_vertex_offset(&rscene, &rblend, v)), || format!("{name} blend offset {v}"));
        }
        let mut cv: Vec<ufbx::Vec3> = (0..max_vertex + 1).map(|i| ufbx::Vec3 { x: 0.25, y: i as f64, z: 3.0 }).collect();
        let mut rv: Vec<rs::Vec3> = cv.iter().map(v3).collect();
        ufbx::add_blend_vertex_offsets(blend, &mut cv, 0.75);
        rs::add_blend_vertex_offsets(&rscene, &rblend, &mut rv, 0.75);
        t.check(cv.iter().zip(&rv).all(|(a, b)| same3(a, b)), || format!("{name} add blend offsets"));
    }
}

fn top_level_fbx(a: &Args) -> Vec<Job> {
    corpus_jobs(a, false, &["fbx"])
}

fn jobs(a: &Args) -> Vec<Job> {
    let mut v: Vec<Job> = ["gen:generate_indices", "gen:synthetic_curves", "gen:synthetic_surfaces"].iter().map(|s| Job::generated(s.to_string())).collect();
    v.extend(top_level_fbx(a));
    v
}

fn load(path: &str) -> Option<ufbx::SceneRoot> {
    ufbx::load_file(path, ufbx::LoadOpts::default()).ok()
}

fn template(a: &Args, has: impl Fn(&ufbx::Scene) -> bool) -> Option<ufbx::SceneRoot> {
    top_level_fbx(a).iter().filter_map(|j| load(&j.item)).find(|s| has(s))
}

fn run(a: &Args, item: &str, ch: &mut Child) {
    match item {
        "gen:generate_indices" => ch.step("generate_indices", |t| check_generate_indices(t, &mut Rng::named(&[item]), 400)),
        "gen:synthetic_curves" => ch.step("synthetic curves", |t| {
            let scene = template(a, |s| !s.nurbs_curves.is_empty()).expect("a corpus scene with a NURBS curve");
            synthetic_curves(t, &scene.nurbs_curves[0], &mut Rng::named(&[item]), 3000);
        }),
        "gen:synthetic_surfaces" => ch.step("synthetic surfaces", |t| {
            let scene = template(a, |s| !s.nurbs_surfaces.is_empty()).expect("a corpus scene with a NURBS surface");
            synthetic_surfaces(t, &scene.nurbs_surfaces[0], &mut Rng::named(&[item]), 600);
        }),
        path => {
            let Some(scene) = load(path) else { return };
            let name = std::path::Path::new(path).file_name().unwrap_or_default().to_string_lossy().to_string();
            ch.step("nurbs curves", |t| {
                t.count("scenes loaded", 1);
                let mut rng = Rng::named(&[&name, "curves"]);
                for (i, c) in scene.nurbs_curves.iter().enumerate() {
                    t.count("NURBS curves", 1);
                    let mut us = vec![c.basis.t_min, c.basis.t_max, c.basis.t_min - 1.0, c.basis.t_max + 1.0];
                    us.extend(c.basis.knot_vector.iter().copied());
                    for _ in 0..16 {
                        us.push(c.basis.t_min + (c.basis.t_max - c.basis.t_min) * rng.real());
                    }
                    check_curve(t, &format!("{name} curve {i}"), c, &curve(c), &us);
                }
            });
            ch.step("nurbs surfaces", |t| {
                let mut rng = Rng::named(&[&name, "surfaces"]);
                for (i, s) in scene.nurbs_surfaces.iter().enumerate() {
                    t.count("NURBS surfaces", 1);
                    let sample = |b: &ufbx::NurbsBasis, rng: &mut Rng| {
                        let mut us = vec![b.t_min, b.t_max, b.t_min - 1.0];
                        us.extend(b.spans.iter().copied());
                        for _ in 0..4 {
                            us.push(b.t_min + (b.t_max - b.t_min) * rng.real());
                        }
                        us
                    };
                    let (us, vs) = (sample(&s.basis_u, &mut rng), sample(&s.basis_v, &mut rng));
                    let subs =
                        [(0, 0, false), (1, 1, false), (2, 3, true), (5, 2, false), (s.span_subdivision_u as usize, s.span_subdivision_v as usize, false)];
                    check_surface(t, &format!("{name} surface {i}"), s, &surface(s), &us, &vs, &subs);
                }
            });
            ch.step("find functions", |t| check_find(t, &name, &scene));
            ch.step("anim props", |t| check_anim_props(t, &name, &scene));
            ch.step("normal matrices", |t| check_normal_matrices(t, &name, &scene));
            ch.step("skin matrices", |t| check_skins(t, &name, &scene));
            ch.step("blend offsets", |t| check_blends(t, &name, &scene, &mut Rng::named(&[&name, "blends"])));
        }
    }
}

pub const API: Section = Section { name: "api", jobs, run, header: None };

use crate::math::*;
use crate::search::unstable_sort;
use crate::types::*;

const KD_FAST_DEPTH: u32 = 6;
const KD_NUM_NODES: usize = 1 << (KD_FAST_DEPTH + 1);

pub(crate) fn stable_sort<T: Copy>(data: &mut [T], tmp: &mut [T], linear_size: usize, less: impl Fn(&T, &T) -> bool) {
    let size = data.len();
    let mut block_size = linear_size;
    let mut base = 0;
    while base < size {
        let i_end = (base + block_size).min(size);
        for i in base + 1..i_end {
            if !less(&data[i], &data[i - 1]) {
                continue;
            }
            let mut j = i - 1;
            tmp[0] = data[i];
            data[i] = data[j];
            while j != base {
                if !less(&tmp[0], &data[j - 1]) {
                    break;
                }
                data[j] = data[j - 1];
                j -= 1;
            }
            data[j] = tmp[0];
        }
        base += block_size;
    }

    let mut dst_is_tmp = false;
    while block_size < size {
        dst_is_tmp = !dst_is_tmp;
        let (src, dst): (&[T], &mut [T]) = if dst_is_tmp { (&*data, &mut *tmp) } else { (&*tmp, &mut *data) };
        let mut base = 0;
        while base < size {
            let (mut i, mut i_end) = (base, base + block_size);
            let (mut j, mut j_end) = (i_end, i_end + block_size);
            let mut k = base;
            i_end = i_end.min(size);
            j_end = j_end.min(size);
            while i < i_end && j < j_end {
                if less(&src[j], &src[i]) {
                    dst[k] = src[j];
                    j += 1;
                } else {
                    dst[k] = src[i];
                    i += 1;
                }
                k += 1;
            }
            dst[k..k + (i_end - i)].copy_from_slice(&src[i..i_end]);
            if j < j_end {
                let k = k + (i_end - i);
                dst[k..k + (j_end - j)].copy_from_slice(&src[j..j_end]);
            }
            base += block_size * 2;
        }
        block_size *= 2;
    }
    if dst_is_tmp {
        data.copy_from_slice(&tmp[..size]);
    }
}

fn lower_bound_eq<T>(data: &[T], begin: usize, size: usize, linear_size: usize, cmp: impl Fn(&T) -> bool, eq: impl Fn(&T) -> bool) -> Option<usize> {
    crate::search::lower_bound_eq(begin, size, linear_size, |i| cmp(&data[i]), |i| eq(&data[i]))
}

#[derive(Clone, Copy, Default)]
struct KdNode {
    split: Real,
    index_plus_one: u32,
    slow_left: u32,
    slow_right: u32,
    slow_end: u32,
}

struct NgonContext<'a> {
    face: Face,
    positions: &'a VertexVec3,
    axes: [Vec3; 3],
    kd_nodes: [KdNode; KD_NUM_NODES],
}

struct KdTriangle {
    min_t: [Real; 2],
    max_t: [Real; 2],
    points: [Vec2; 3],
    indices: [u32; 3],
}

impl NgonContext<'_> {
    fn point(&self, index: u32) -> Vec3 {
        self.positions.get(self.face.index_begin as usize + index as usize)
    }

    fn ngon_project(&self, index: u32) -> Vec2 {
        let point = self.point(index);
        Vec2::new(dot3(self.axes[0], point), dot3(self.axes[1], point))
    }
}

fn orient2d(a: Vec2, b: Vec2, c: Vec2) -> Real {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn kd_check_point(nc: &NgonContext, tri: &KdTriangle, index: u32) -> bool {
    if index == tri.indices[0] || index == tri.indices[1] || index == tri.indices[2] {
        return false;
    }
    let p = nc.ngon_project(index);
    let u = orient2d(p, tri.points[0], tri.points[1]);
    let v = orient2d(p, tri.points[1], tri.points[2]);
    let w = orient2d(p, tri.points[2], tri.points[0]);
    if u <= 0.0 && v <= 0.0 && w <= 0.0 {
        return true;
    }
    if u >= 0.0 && v >= 0.0 && w >= 0.0 {
        return true;
    }
    false
}

fn kd_check_slow(nc: &NgonContext, kd: &[u32], tri: &KdTriangle, mut begin: u32, mut count: u32, mut axis: u32) -> bool {
    while count > 0 {
        let num_left = count / 2;
        let begin_right = begin + num_left + 1;
        let num_right = count - (num_left + 1);

        let index = kd[(begin + num_left) as usize];
        let split = dot3(nc.point(index), nc.axes[axis as usize]);
        let hit_left = tri.min_t[axis as usize] <= split;
        let hit_right = tri.max_t[axis as usize] >= split;

        if hit_left && hit_right {
            if kd_check_point(nc, tri, index) {
                return true;
            }
            if kd_check_slow(nc, kd, tri, begin_right, num_right, axis ^ 1) {
                return true;
            }
        }

        axis ^= 1;
        if hit_left {
            count = num_left;
        } else {
            begin = begin_right;
            count = num_right;
        }
    }
    false
}

fn kd_check_fast(nc: &NgonContext, kd: &[u32], tri: &KdTriangle, mut kd_index: u32, mut axis: u32, mut depth: u32) -> bool {
    loop {
        let node = nc.kd_nodes[kd_index as usize];
        if node.index_plus_one == 0 {
            return false;
        }

        let hit_left = tri.min_t[axis as usize] <= node.split;
        let hit_right = tri.max_t[axis as usize] >= node.split;

        let side = if hit_left { 0 } else { 1 };
        let child_kd_index = kd_index * 2 + 1 + side;
        if hit_left && hit_right {
            let index = node.index_plus_one - 1;
            if kd_check_point(nc, tri, index) {
                return true;
            }

            if depth + 1 == KD_FAST_DEPTH {
                if kd_check_slow(nc, kd, tri, node.slow_right, node.slow_end - node.slow_right, axis ^ 1) {
                    return true;
                }
            } else if kd_check_fast(nc, kd, tri, child_kd_index + 1, axis ^ 1, depth + 1) {
                return true;
            }
        }

        depth += 1;
        axis ^= 1;
        kd_index = child_kd_index;

        if depth == KD_FAST_DEPTH {
            return if hit_left {
                kd_check_slow(nc, kd, tri, node.slow_left, node.slow_right - node.slow_left, axis)
            } else {
                kd_check_slow(nc, kd, tri, node.slow_right, node.slow_end - node.slow_right, axis)
            };
        }
    }
}

fn kd_check(nc: &NgonContext, kd: &[u32], points: &[Vec2], indices: &[u32]) -> bool {
    let tri = KdTriangle {
        points: [points[0], points[1], points[2]],
        indices: [indices[0], indices[1], indices[2]],
        min_t: [min_real(min_real(points[0].x, points[1].x), points[2].x), min_real(min_real(points[0].y, points[1].y), points[2].y)],
        max_t: [max_real(max_real(points[0].x, points[1].x), points[2].x), max_real(max_real(points[0].y, points[1].y), points[2].y)],
    };
    kd_check_fast(nc, kd, &tri, 0, 0, 0)
}

/// `buf[off..off + num]` are the points, `buf[tmp_off..]` the sort's scratch
#[allow(clippy::too_many_arguments)]
fn kd_build(nc: &mut NgonContext, buf: &mut [u32], off: usize, tmp_off: usize, num: u32, axis: u32, fast_index: u32, depth: u32) {
    if num == 0 {
        return;
    }

    let axis_dir = nc.axes[axis as usize];
    {
        let ctx = &*nc;
        let (lo, hi) = buf.split_at_mut(tmp_off);
        // `ufbxi_kd_index_less` in ufbx.c
        stable_sort(&mut lo[off..off + num as usize], &mut hi[..num as usize], 16, |&a, &b| dot3(axis_dir, ctx.point(a)) < dot3(axis_dir, ctx.point(b)));
    }

    let num_left = num / 2;
    let begin_right = num_left + 1;
    let num_right = num - begin_right;
    let mut dst_right = num_left + 1;
    if depth < KD_FAST_DEPTH {
        let skip_left = 1u32 << (KD_FAST_DEPTH - depth - 1);
        dst_right = dst_right.saturating_sub(skip_left);

        let index = buf[off + num_left as usize];
        let split = dot3(axis_dir, nc.point(index));
        let kd = &mut nc.kd_nodes[fast_index as usize];
        kd.split = split;
        kd.index_plus_one = index + 1;

        if depth + 1 == KD_FAST_DEPTH {
            kd.slow_left = off as u32;
            kd.slow_right = kd.slow_left + num_left;
            kd.slow_end = kd.slow_right + num_right;
        } else {
            kd.slow_left = u32::MAX;
            kd.slow_right = u32::MAX;
            kd.slow_end = u32::MAX;
        }
    }

    let child_fast = fast_index.wrapping_mul(2).wrapping_add(1);
    kd_build(nc, buf, off, tmp_off, num_left, axis ^ 1, child_fast, depth + 1);

    if dst_right != begin_right {
        let src = off + begin_right as usize;
        buf.copy_within(src..src + num_right as usize, off + dst_right as usize);
    }

    kd_build(nc, buf, off + dst_right as usize, tmp_off, num_right, axis ^ 1, child_fast.wrapping_add(1), depth + 1);
}

fn ngon_tri_weight(points: &[Vec2]) -> Real {
    let (p0, p1, p2) = (points[0], points[1], points[2]);
    let orient = orient2d(p0, p1, p2);
    if orient <= 0.0 {
        return -1.0;
    }

    let a = distsq2(p0, p1);
    let b = distsq2(p1, p2);
    let c = distsq2(p2, p0);
    let ab = (a + b - c) / (4.0 * a * b).sqrt();
    let bc = (b + c - a) / (4.0 * b * c).sqrt();
    let ca = (c + a - b) / (4.0 * c * a).sqrt();
    EPSILON.max(2.0 - ab.max(bc).max(ca))
}

/// Ear clipping with the reflex corners in a kd-tree; `buf` holds the kd-tree, the polygon's links at its end and finally the
/// triangles
fn triangulate_ngon(nc: &mut NgonContext, buf: &mut [u32]) -> u32 {
    let face = nc.face;
    let num_indices = buf.len();
    let n = face.num_indices;
    assert!(n > 4);

    let mut normal = get_weighted_face_normal(nc.positions, face);
    let len = length3(normal);
    if len > EPSILON {
        normal = mul3(normal, 1.0 / len);
    } else {
        normal = Vec3::new(1.0, 0.0, 0.0);
    }

    let axis = if normal.x * normal.x < 0.5 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
    // `ufbxi_slow_normalized_cross3` in ufbx.c
    nc.axes[0] = normalize3(cross3(axis, normal));
    nc.axes[1] = normalize3(cross3(normal, nc.axes[0]));
    nc.axes[2] = normal;

    let kd_tmp = n as usize;

    let mut num_kd_indices = 0u32;
    {
        let mut a = nc.ngon_project(n - 1);
        let mut b = nc.ngon_project(0);
        for i in 0..n {
            let next = if i + 1 < n { i + 1 } else { 0 };
            let c = nc.ngon_project(next);
            if orient2d(a, b, c) <= 0.0 {
                buf[num_kd_indices as usize] = i;
                num_kd_indices += 1;
            }
            a = b;
            b = c;
        }
    }

    let num_skip_indices = (1u32 << (KD_FAST_DEPTH + 1)) - 1;
    let kd_slow_indices = num_kd_indices.saturating_sub(num_skip_indices);
    assert!(kd_slow_indices as usize + n as usize * 2 <= num_indices);
    kd_build(nc, buf, 0, kd_tmp, num_kd_indices, 0, 0, 0);

    let edges = num_indices - n as usize * 2;

    for i in 0..n {
        buf[edges + (i * 2) as usize] = if i > 0 { i - 1 } else { n - 1 };
        buf[edges + (i * 2 + 1) as usize] = if i + 1 < n { i + 1 } else { 0 };
    }

    let mut indices_left = n;
    {
        let mut point_indices = [0u32, 1, 2, 3];
        let mut weights = [0.0 as Real; 2];
        let mut points = [Vec2::ZERO; 4];

        let mut num_steps = 0u32;
        while indices_left > 3 {
            for k in 0..4 {
                points[k] = nc.ngon_project(point_indices[k]);
            }

            weights[0] = ngon_tri_weight(&points[0..]);
            weights[1] = ngon_tri_weight(&points[1..]);

            let first_side = if weights[1] > weights[0] { 1 } else { 0 };
            let mut clipped = false;
            for side_ix in 0..2 {
                let side = side_ix ^ first_side;
                if !(weights[side] >= 0.0) {
                    break;
                }

                if !kd_check(nc, buf, &points[side..], &point_indices[side..]) {
                    let ia = point_indices[side];
                    let ib = point_indices[side + 1];
                    let ic = point_indices[side + 2];

                    buf[edges + (ib * 2) as usize] |= 0x8000_0000;
                    buf[edges + (ib * 2 + 1) as usize] |= 0x8000_0000;

                    buf[edges + (ic * 2) as usize] = ia;
                    buf[edges + (ia * 2 + 1) as usize] = ic;

                    indices_left -= 1;
                    num_steps = 0;

                    if side == 1 {
                        point_indices[2] = point_indices[3];
                        point_indices[3] = buf[edges + (point_indices[3] * 2 + 1) as usize];
                    } else {
                        point_indices[1] = point_indices[0];
                        point_indices[0] = buf[edges + (point_indices[0] * 2) as usize];
                    }

                    clipped = true;
                    break;
                }
            }
            if clipped {
                continue;
            }

            point_indices[0] = point_indices[1];
            point_indices[1] = point_indices[2];
            point_indices[2] = point_indices[3];
            point_indices[3] = buf[edges + (point_indices[3] * 2 + 1) as usize];
            num_steps += 1;

            if num_steps >= n * 2 {
                break;
            }
        }

        let mut ix = point_indices[1];
        while indices_left > 3 {
            let prev = buf[edges + (ix * 2) as usize];
            let next = buf[edges + (ix * 2 + 1) as usize];

            buf[edges + (ix * 2) as usize] |= 0x8000_0000;
            buf[edges + (ix * 2 + 1) as usize] |= 0x8000_0000;

            buf[edges + (prev * 2 + 1) as usize] = next;
            buf[edges + (next * 2) as usize] = prev;

            indices_left -= 1;
            ix = next;
        }

        buf[edges + (ix * 2) as usize] |= 0x8000_0000;
        buf[edges + (ix * 2 + 1) as usize] |= 0x8000_0000;
    }

    // The last four triangles may overlap the links still to be read, so they go through a stack buffer
    let max_triangles = n - 2;
    let mut num_triangles = 0u32;
    let mut num_last_triangles = 0u32;
    let mut last_triangles = [0u32; 4 * 3];

    let index_begin = face.index_begin;
    for ix in 0..n {
        let prev = buf[edges + (ix * 2) as usize];
        let next = buf[edges + (ix * 2 + 1) as usize];
        if prev & 0x8000_0000 == 0 {
            continue;
        }

        let tri = [index_begin + (prev & 0x7fff_ffff), index_begin + ix, index_begin + (next & 0x7fff_ffff)];
        if num_triangles + 4 >= max_triangles {
            let dst = (num_last_triangles * 3) as usize;
            last_triangles[dst..dst + 3].copy_from_slice(&tri);
            num_last_triangles += 1;
        } else {
            let dst = (num_triangles * 3) as usize;
            buf[dst..dst + 3].copy_from_slice(&tri);
        }
        num_triangles += 1;
    }

    assert!(num_triangles == max_triangles);
    let dst = ((max_triangles - num_last_triangles) * 3) as usize;
    let count = (num_last_triangles * 3) as usize;
    buf[dst..dst + count].copy_from_slice(&last_triangles[..count]);

    num_triangles
}

fn topo_less_index_prev_next(a: &TopoEdge, b: &TopoEdge) -> bool {
    if a.prev as i32 != b.prev as i32 {
        return (a.prev as i32) < (b.prev as i32);
    }
    (a.next as i32) < (b.next as i32)
}

fn topo_less_index_index(a: &TopoEdge, b: &TopoEdge) -> bool {
    (a.index as i32) < (b.index as i32)
}

/// `ufbxi_compute_topology` in ufbx.c
fn compute_topology_imp(mesh: &Mesh, topo: &mut [TopoEdge]) {
    let num_indices = mesh.num_indices;

    // `prev` and `next` hold the edge's two vertices until the end
    for fi in 0..mesh.num_faces as u32 {
        let face = mesh.faces[fi as usize];
        for pi in 0..face.num_indices {
            let ni = (pi + 1) % face.num_indices;
            let mut va = mesh.vertex_indices[(face.index_begin + pi) as usize];
            let mut vb = mesh.vertex_indices[(face.index_begin + ni) as usize];
            if vb < va {
                std::mem::swap(&mut va, &mut vb);
            }
            topo[(face.index_begin + pi) as usize] =
                TopoEdge { index: face.index_begin + pi, twin: NO_INDEX, edge: NO_INDEX, prev: va, next: vb, face: fi, flags: 0 };
        }
    }

    unstable_sort(&mut topo[..num_indices], topo_less_index_prev_next);

    if !mesh.edges.is_empty() || mesh.has_edges {
        for ei in 0..mesh.num_edges as u32 {
            let edge = mesh.edges[ei as usize];
            let mut va = mesh.vertex_indices[edge.a as usize];
            let mut vb = mesh.vertex_indices[edge.b as usize];
            if vb < va {
                std::mem::swap(&mut va, &mut vb);
            }

            let mut ix = lower_bound_eq(topo, 0, num_indices, 32, |a| if a.prev == va { a.next < vb } else { a.prev < va }, |a| a.prev == va && a.next == vb)
                .unwrap_or(num_indices);

            while ix < num_indices && topo[ix].prev == va && topo[ix].next == vb {
                topo[ix].edge = ei;
                ix += 1;
            }
        }
    }

    let mut i0 = 0;
    while i0 < num_indices {
        let mut i1 = i0;
        let (a, b) = (topo[i0].prev, topo[i0].next);
        while i1 + 1 < num_indices && topo[i1 + 1].prev == a && topo[i1 + 1].next == b {
            i1 += 1;
        }

        if i1 == i0 + 1 {
            topo[i0].twin = topo[i1].index;
            topo[i1].twin = topo[i0].index;
        } else if i1 > i0 + 1 {
            for t in &mut topo[i0..=i1] {
                t.flags |= TOPO_NON_MANIFOLD;
            }
        }

        i0 = i1 + 1;
    }

    unstable_sort(&mut topo[..num_indices], topo_less_index_index);

    for fi in 0..mesh.num_faces {
        let face = mesh.faces[fi];
        for i in 0..face.num_indices {
            let to = &mut topo[(face.index_begin + i) as usize];
            to.prev = face.index_begin + (i + face.num_indices - 1) % face.num_indices;
            to.next = face.index_begin + (i + 1) % face.num_indices;
        }
    }
}

fn is_edge_smooth(mesh: &Mesh, topo: &[TopoEdge], index: u32, assume_smooth: bool) -> bool {
    let te = topo[index as usize];
    if !mesh.edge_smoothing.is_empty() || mesh.has_edge_smoothing {
        let edge = te.edge;
        if edge != NO_INDEX && mesh.edge_smoothing.get(edge as usize).copied().unwrap_or(false) {
            return true;
        }
    }

    if !mesh.face_smoothing.is_empty() {
        if mesh.face_smoothing.get(te.face as usize).copied().unwrap_or(false) {
            return true;
        }
        let twin = te.twin;
        if twin != NO_INDEX && mesh.face_smoothing.get(topo[twin as usize].face as usize).copied().unwrap_or(false) {
            return true;
        }
    }

    if mesh.edge_smoothing.is_empty() && !mesh.has_edge_smoothing && mesh.face_smoothing.is_empty() && mesh.vertex_normal.exists {
        let twin = te.twin;
        if twin != NO_INDEX {
            let a0 = mesh.vertex_normal.get(index as usize);
            let a1 = mesh.vertex_normal.get(te.next as usize);
            let b0 = mesh.vertex_normal.get(topo[twin as usize].next as usize);
            let b1 = mesh.vertex_normal.get(twin as usize);
            if a0.x == b0.x && a0.y == b0.y && a0.z == b0.z {
                return true;
            }
            if a1.x == b1.x && a1.y == b1.y && a1.z == b1.z {
                return true;
            }
        }
    } else if assume_smooth {
        return true;
    }

    false
}

/// `NO_INDEX` if none
pub fn find_face_index(mesh: &Mesh, index: usize) -> u32 {
    if index > u32::MAX as usize {
        return NO_INDEX;
    }
    let ix = index as u32;
    lower_bound_eq(
        &mesh.faces,
        0,
        mesh.faces.len(),
        4,
        |a| a.index_begin.wrapping_add(a.num_indices) <= ix,
        |a| ix >= a.index_begin && ix < a.index_begin.wrapping_add(a.num_indices),
    )
    .map_or(NO_INDEX, |i| i as u32)
}

/// Writes `face`'s triangles into `indices` as mesh indices, three per triangle, and returns their number
pub fn triangulate_face(indices: &mut [u32], mesh: &Mesh, face: Face) -> u32 {
    if face.num_indices < 3 {
        return 0;
    }

    let required_indices = (face.num_indices as usize - 2) * 3;
    assert!(indices.len() >= required_indices, "Face needs at least {} indices for triangles, got space for {}", required_indices, indices.len());
    assert!((face.index_begin as usize) < mesh.num_indices, "Face index begin ({}) out of bounds ({})", face.index_begin, mesh.num_indices);
    assert!(
        mesh.num_indices - face.index_begin as usize >= face.num_indices as usize,
        "Face index end ({} + {}) out of bounds ({})",
        face.index_begin,
        face.num_indices,
        mesh.num_indices
    );

    if face.num_indices == 3 {
        indices[0] = face.index_begin;
        indices[1] = face.index_begin + 1;
        indices[2] = face.index_begin + 2;
        1
    } else if face.num_indices == 4 {
        // Split along the shorter diagonal unless a vertex crosses it
        let i0 = face.index_begin;
        let i1 = face.index_begin + 1;
        let i2 = face.index_begin + 2;
        let i3 = face.index_begin + 3;
        let pos = &mesh.vertex_position;
        let v0 = pos.get(i0 as usize);
        let v1 = pos.get(i1 as usize);
        let v2 = pos.get(i2 as usize);
        let v3 = pos.get(i3 as usize);

        let a = sub3(v2, v0);
        let b = sub3(v3, v1);

        let na1 = normalize3(cross3(a, sub3(v1, v0)));
        let na3 = normalize3(cross3(a, sub3(v0, v3)));
        let nb0 = normalize3(cross3(b, sub3(v1, v0)));
        let nb2 = normalize3(cross3(b, sub3(v2, v1)));

        let dot_aa = dot3(a, a);
        let dot_bb = dot3(b, b);
        let dot_na = dot3(na1, na3);
        let dot_nb = dot3(nb0, nb2);

        let mut split_a = dot_aa <= dot_bb;
        if dot_na < 0.0 || dot_nb < 0.0 {
            split_a = dot_na >= dot_nb;
        }

        if split_a {
            indices[..6].copy_from_slice(&[i0, i1, i2, i2, i3, i0]);
        } else {
            indices[..6].copy_from_slice(&[i1, i2, i3, i3, i0, i1]);
        }
        2
    } else {
        let mut nc = NgonContext { face, positions: &mesh.vertex_position, axes: [Vec3::ZERO; 3], kd_nodes: [KdNode::default(); KD_NUM_NODES] };

        let num_indices_u32 = indices.len().min(u32::MAX as usize);
        if num_indices_u32 < 12 {
            let mut local_indices = [0u32; 12];
            let num_tris = triangulate_ngon(&mut nc, &mut local_indices);
            let count = num_tris as usize * 3;
            indices[..count].copy_from_slice(&local_indices[..count]);
            num_tris
        } else {
            triangulate_ngon(&mut nc, &mut indices[..num_indices_u32])
        }
    }
}

/// `topo` must be at least `mesh.num_indices` long
pub fn compute_topology(mesh: &Mesh, topo: &mut [TopoEdge]) {
    assert!(topo.len() >= mesh.num_indices, "Required mesh.num_indices ({}) indices, got {}", mesh.num_indices, topo.len());
    compute_topology_imp(mesh, topo);
}

pub fn topo_next_vertex_edge(topo: &[TopoEdge], index: u32) -> u32 {
    if index == NO_INDEX {
        return NO_INDEX;
    }
    assert!((index as usize) < topo.len(), "index ({}) out of bounds ({})", index, topo.len());
    let twin = topo[index as usize].twin;
    if twin == NO_INDEX {
        return NO_INDEX;
    }
    assert!((twin as usize) < topo.len(), "Corrupted topology structure");
    topo[twin as usize].next
}

pub fn topo_prev_vertex_edge(topo: &[TopoEdge], index: u32) -> u32 {
    if index == NO_INDEX {
        return NO_INDEX;
    }
    assert!((index as usize) < topo.len(), "index ({}) out of bounds ({})", index, topo.len());
    topo[topo[index as usize].prev as usize].twin
}

/// The face's normal scaled by its area, by Newell's method above four corners
pub fn get_weighted_face_normal(positions: &VertexVec3, face: Face) -> Vec3 {
    let count = positions.indices.len();
    assert!(face.index_begin as usize <= count, "Face index begin ({}) out of bounds ({})", face.index_begin, count);
    assert!(
        count - face.index_begin as usize >= face.num_indices as usize,
        "Face index end ({} + {}) out of bounds ({})",
        face.index_begin,
        face.num_indices,
        count
    );

    let begin = face.index_begin as usize;
    if face.num_indices < 3 {
        Vec3::ZERO
    } else if face.num_indices == 3 {
        let a = positions.get(begin);
        let b = positions.get(begin + 1);
        let c = positions.get(begin + 2);
        cross3(sub3(b, a), sub3(c, a))
    } else if face.num_indices == 4 {
        let a = positions.get(begin);
        let b = positions.get(begin + 1);
        let c = positions.get(begin + 2);
        let d = positions.get(begin + 3);
        cross3(sub3(c, a), sub3(d, b))
    } else {
        let n = face.num_indices as usize;
        let mut result = Vec3::ZERO;
        for i in 0..n {
            let next = if i + 1 < n { i + 1 } else { 0 };
            let a = positions.get(begin + i);
            let b = positions.get(begin + next);
            result.x += (a.y - b.y) * (a.z + b.z);
            result.y += (a.z - b.z) * (a.x + b.x);
            result.z += (a.x - b.x) * (a.y + b.y);
        }
        result
    }
}

/// Assigns each mesh index a normal, shared around a vertex across smooth edges; returns the number of normals
pub fn generate_normal_mapping(mesh: &Mesh, topo: &[TopoEdge], normal_indices: &mut [u32], assume_smooth: bool) -> usize {
    let mut next_index = 0u32;
    assert!(normal_indices.len() >= mesh.num_indices, "Expected at least mesh.num_indices ({}), got {}", mesh.num_indices, normal_indices.len());

    for ix in &mut normal_indices[..mesh.num_indices] {
        *ix = NO_INDEX;
    }

    for vi in 0..mesh.num_vertices {
        let original_start = mesh.vertex_first_index[vi];
        if original_start == NO_INDEX {
            continue;
        }
        let mut start = original_start;
        let mut cur = start;

        loop {
            let prev = topo_next_vertex_edge(topo, cur);
            if !is_edge_smooth(mesh, topo, cur, assume_smooth) {
                start = cur;
            }
            if prev == NO_INDEX {
                start = cur;
                break;
            }
            if prev == original_start {
                break;
            }
            cur = prev;
        }

        normal_indices[start as usize] = next_index;
        next_index += 1;
        let mut next = start;
        loop {
            next = topo_prev_vertex_edge(topo, next);
            if next == NO_INDEX || next == start {
                break;
            }
            if !is_edge_smooth(mesh, topo, next, assume_smooth) {
                next_index += 1;
            }
            normal_indices[next as usize] = next_index - 1;
        }
    }

    for ix in &mut normal_indices[..mesh.num_indices] {
        if *ix == NO_INDEX {
            *ix = next_index;
            next_index += 1;
        }
    }

    next_index as usize
}

/// Sums the weighted face normals into `normals` by `normal_indices`, then normalizes them
pub fn compute_normals(mesh: &Mesh, positions: &VertexVec3, normal_indices: &[u32], normals: &mut [Vec3]) {
    assert!(normal_indices.len() >= mesh.num_indices, "Expected at least mesh.num_indices ({}), got {}", mesh.num_indices, normal_indices.len());

    for n in normals.iter_mut() {
        *n = Vec3::ZERO;
    }

    for fi in 0..mesh.num_faces {
        let face = mesh.faces[fi];
        let normal = get_weighted_face_normal(positions, face);
        for ix in 0..face.num_indices as usize {
            let index = normal_indices[face.index_begin as usize + ix];
            assert!((index as usize) < normals.len(), "Normal index ({}) out of bounds ({}) at {}", index, normals.len(), ix);
            let n = &mut normals[index as usize];
            *n = add3(*n, normal);
        }
    }

    for n in normals.iter_mut() {
        let len = length3(*n);
        if len > 0.0 {
            n.x /= len;
            n.y /= len;
            n.z /= len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn mesh_from(positions: &[[Real; 3]], faces: &[&[u32]]) -> Mesh {
        let mut mesh = Mesh::default();
        let mut begin = 0;
        for f in faces {
            mesh.faces.push(Face { index_begin: begin, num_indices: f.len() as u32 });
            mesh.vertex_indices.extend_from_slice(f);
            begin += f.len() as u32;
        }
        mesh.vertex_position.exists = true;
        mesh.vertex_position.values = positions.iter().map(|&p| Vec3::from_array(p)).collect();
        mesh.vertex_position.indices = mesh.vertex_indices.clone();
        crate::mesh::finalize_mesh(&mut mesh).unwrap();
        mesh
    }

    fn triangulate(mesh: &Mesh, fi: usize) -> Vec<u32> {
        let face = mesh.faces[fi];
        let mut out = vec![0u32; (face.num_indices as usize - 2) * 3];
        let n = triangulate_face(&mut out, mesh, face);
        assert_eq!(n, face.num_indices - 2);
        out
    }

    #[test]
    fn quads_split_on_the_short_diagonal_unless_concave() {
        let kite = mesh_from(&[[-0.5, 0.0, 0.0], [0.0, -2.0, 0.0], [0.5, 0.0, 0.0], [0.0, 2.0, 0.0]], &[&[0, 1, 2, 3]]);
        assert_eq!(triangulate(&kite, 0), [0, 1, 2, 2, 3, 0]);
        // An arrowhead whose short diagonal 0-2 runs outside it, past the reflex corner 3
        let arrow = mesh_from(&[[-1.0, 0.0, 0.0], [0.0, -10.0, 0.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]], &[&[0, 1, 2, 3]]);
        assert_eq!(triangulate(&arrow, 0), [1, 2, 3, 3, 0, 1]);
    }

    #[test]
    fn ngons_triangulate_inside_the_polygon() {
        let hexagon: Vec<[Real; 3]> = (0..6)
            .map(|i| {
                let t = i as Real * std::f64::consts::PI / 3.0;
                [t.cos(), t.sin(), 0.0]
            })
            .collect();
        let mesh = mesh_from(&hexagon, &[&[0, 1, 2, 3, 4, 5]]);
        let tris = triangulate(&mesh, 0);
        assert_eq!(tris.len(), 12);
        let mut area = 0.0;
        for t in tris.chunks(3) {
            let p: Vec<Vec3> = t.iter().map(|&i| mesh.vertex_position.get(i as usize)).collect();
            area += cross3(sub3(p[1], p[0]), sub3(p[2], p[0])).z * 0.5;
        }
        assert!((area - 1.5 * 3.0f64.sqrt()).abs() < 1e-12);

        let comb = [
            [0.0, 0.0, 0.0],
            [5.0, 0.0, 0.0],
            [5.0, 3.0, 0.0],
            [4.0, 3.0, 0.0],
            [4.0, 1.0, 0.0],
            [3.0, 1.0, 0.0],
            [3.0, 3.0, 0.0],
            [2.0, 3.0, 0.0],
            [2.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 3.0, 0.0],
            [0.0, 3.0, 0.0],
        ];
        let idx: Vec<u32> = (0..12).collect();
        let mesh = mesh_from(&comb, &[&idx]);
        let tris = triangulate(&mesh, 0);
        let mut area = 0.0;
        for t in tris.chunks(3) {
            let p: Vec<Vec3> = t.iter().map(|&i| mesh.vertex_position.get(i as usize)).collect();
            let a = cross3(sub3(p[1], p[0]), sub3(p[2], p[0])).z * 0.5;
            assert!(a >= 0.0, "triangle {t:?} is inverted");
            area += a;
        }
        assert!((area - 11.0).abs() < 1e-12);
    }

    fn comb(teeth: i64, varied: bool) -> Mesh {
        let mut pts = vec![[0.0, 0.0, 0.0], [(2 * teeth - 1) as Real, 0.0, 0.0]];
        for j in (0..teeth).rev() {
            let h = if varied { 3 + (j * 7) % 5 } else { 3 } as Real;
            pts.push([(2 * j + 1) as Real, h, 0.0]);
            pts.push([(2 * j) as Real, h, 0.0]);
            if j > 0 {
                pts.push([(2 * j) as Real, 1.0, 0.0]);
                pts.push([(2 * j - 1) as Real, 1.0, 0.0]);
            }
        }
        let idx: Vec<u32> = (0..pts.len() as u32).collect();
        mesh_from(&pts, &[&idx])
    }

    #[test]
    fn ngons_match_the_c_library() {
        // Expected values printed by ufbx 0.23.1 itself for the same polygons
        assert_eq!(triangulate(&comb(3, false), 0), [11, 0, 9, 0, 1, 5, 1, 2, 3, 1, 3, 4, 1, 4, 5, 0, 5, 8, 5, 6, 7, 5, 7, 8, 0, 8, 9, 10, 11, 9]);
        // 300 corners, 148 of them reflex: past the kd-tree's 127 fast nodes
        let tris = triangulate(&comb(75, true), 0);
        let mut hash: u64 = 0xcbf29ce484222325;
        for &i in &tris {
            hash = (hash ^ i as u64).wrapping_mul(0x100000001b3);
        }
        assert_eq!(hash, 0xf0b689502025eb7b);
    }

    #[test]
    fn sorts_match_their_contract() {
        let mut state = 12345u32;
        let mut next = || {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            (state >> 16) % 50
        };
        for len in [0usize, 1, 2, 15, 16, 17, 33, 100, 257] {
            let pairs: Vec<(u32, u32)> = (0..len as u32).map(|i| (next(), i)).collect();
            let mut stable = pairs.clone();
            let mut tmp = vec![(0, 0); len.max(1)];
            stable_sort(&mut stable, &mut tmp, 16, |a, b| a.0 < b.0);
            let mut expected = pairs.clone();
            expected.sort_by_key(|p| p.0);
            assert_eq!(stable, expected);

            let mut heap = pairs.clone();
            unstable_sort(&mut heap, |a, b| a < b);
            let mut expected = pairs.clone();
            expected.sort();
            assert_eq!(heap, expected);
        }
    }

    pub(crate) fn cube() -> Mesh {
        let p = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        let faces: [&[u32]; 6] = [&[0, 3, 2, 1], &[4, 5, 6, 7], &[0, 1, 5, 4], &[1, 2, 6, 5], &[2, 3, 7, 6], &[3, 0, 4, 7]];
        mesh_from(&p, &faces)
    }

    #[test]
    fn cube_topology_is_closed_and_consistent() {
        let mesh = cube();
        let mut topo = vec![TopoEdge::default(); mesh.num_indices];
        compute_topology(&mesh, &mut topo);
        for (i, t) in topo.iter().enumerate() {
            assert_eq!(t.index, i as u32);
            assert_eq!(t.face, i as u32 / 4);
            assert_eq!(t.flags, 0);
            assert_ne!(t.twin, NO_INDEX);
            let tw = topo[t.twin as usize];
            assert_eq!(tw.twin, i as u32);
            assert_eq!(mesh.vertex_indices[i], mesh.vertex_indices[tw.next as usize]);
            assert_eq!(mesh.vertex_indices[t.next as usize], mesh.vertex_indices[t.twin as usize]);
            assert_eq!(topo[t.next as usize].prev, i as u32);
        }
        for start in 0..mesh.num_indices as u32 {
            let mut n = 0;
            let mut cur = start;
            loop {
                cur = topo_next_vertex_edge(&topo, cur);
                n += 1;
                if cur == start {
                    break;
                }
            }
            assert_eq!(n, 3);
            assert_eq!(topo_prev_vertex_edge(&topo, topo_next_vertex_edge(&topo, start)), start);
        }

        let mut normal_indices = vec![0u32; mesh.num_indices];
        assert_eq!(generate_normal_mapping(&mesh, &topo, &mut normal_indices, false), 24);
        assert_eq!(generate_normal_mapping(&mesh, &topo, &mut normal_indices, true), 8);
        let mut normals = vec![Vec3::ZERO; 8];
        compute_normals(&mesh, &mesh.vertex_position, &normal_indices, &mut normals);
        let k = 1.0 / 3.0f64.sqrt();
        for (i, &vi) in mesh.vertex_indices.iter().enumerate() {
            let p = mesh.vertices[vi as usize];
            let n = normals[normal_indices[i] as usize];
            assert!((n.x - p.x * k).abs() < 1e-15 && (n.y - p.y * k).abs() < 1e-15 && (n.z - p.z * k).abs() < 1e-15);
        }
        assert_eq!(get_weighted_face_normal(&mesh.vertex_position, mesh.faces[1]), Vec3::new(0.0, 0.0, 8.0));
    }

    #[test]
    fn faces_are_found_by_index() {
        let mesh = cube();
        for ix in 0..24 {
            assert_eq!(find_face_index(&mesh, ix), ix as u32 / 4);
        }
        assert_eq!(find_face_index(&mesh, 24), NO_INDEX);
    }
}

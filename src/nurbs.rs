use crate::error::{Res, check};
use crate::math::normalize3;
use crate::mesh::{finalize_mesh, finalize_mesh_material};
use crate::search::lower_bound_eq;
use crate::types::*;

const MAX_NURBS_ORDER: usize = 128;

fn nurbs_weight(knots: &[Real], knot: usize, degree: usize, u: Real) -> Real {
    if knot >= knots.len() {
        return 0.0;
    }
    if knots.len() - knot < degree {
        return 0.0;
    }
    // ufbx reads one past the knots when exactly `degree` remain; only a basis built by hand gets here
    let (prev_u, Some(&next_u)) = (knots[knot], knots.get(knot + degree)) else { return 0.0 };
    if prev_u >= next_u {
        return 0.0;
    }
    if u <= prev_u {
        return 0.0;
    }
    if u >= next_u {
        return 1.0;
    }
    (u - prev_u) / (next_u - prev_u)
}

fn nurbs_deriv(knots: &[Real], knot: usize, degree: usize) -> Real {
    if knot >= knots.len() {
        return 0.0;
    }
    if knots.len() - knot < degree {
        return 0.0;
    }
    let (prev_u, Some(&next_u)) = (knots[knot], knots.get(knot + degree)) else { return 0.0 };
    if prev_u >= next_u {
        return 0.0;
    }
    degree as Real / (next_u - prev_u)
}

/// The first of the `order` control points the weights belong to, `usize::MAX` where there is no span
pub fn evaluate_nurbs_basis(basis: &NurbsBasis, mut u: Real, weights: &mut [Real], derivatives: &mut [Real]) -> usize {
    if basis.order == 0 {
        return usize::MAX;
    }
    if !basis.valid {
        return usize::MAX;
    }

    let degree = basis.order as usize - 1;
    let knots = &basis.knot_vector[..];
    let mut knot = usize::MAX;

    if u <= basis.t_min {
        knot = degree;
        u = basis.t_min;
    } else if u >= basis.t_max {
        knot = knots.len().wrapping_sub(degree).wrapping_sub(2);
        u = basis.t_max;
    } else {
        let at = |i: usize| knots.get(i).copied();
        if let Some(i) = lower_bound_eq(
            0,
            knots.len().wrapping_sub(1),
            8,
            |i| at(i + 1).is_some_and(|b| b <= u),
            |i| matches!((at(i), at(i + 1)), (Some(a), Some(b)) if a <= u && u < b),
        ) {
            knot = i;
        }
    }

    if knot < degree {
        return usize::MAX;
    }

    let num_derivatives = derivatives.len();
    let has_derivs = num_derivatives > 0;
    if weights.len() < basis.order as usize {
        return knot.wrapping_sub(degree);
    }

    weights[0] = 1.0;
    for p in 1..=degree {
        let mut prev: Real = 0.0;
        let mut g = 1.0 - nurbs_weight(knots, knot.wrapping_sub(p).wrapping_add(1), p, u);
        let mut dg: Real = 0.0;
        if has_derivs && p == degree {
            dg = nurbs_deriv(knots, knot.wrapping_sub(p).wrapping_add(1), p);
        }

        for i in (1..=p).rev() {
            let f = nurbs_weight(knots, knot.wrapping_sub(p).wrapping_add(i), p, u);
            let weight = weights[i - 1];
            weights[i] = f * weight + g * prev;

            if has_derivs && p == degree {
                let df = nurbs_deriv(knots, knot.wrapping_sub(p).wrapping_add(i), p);
                if i < num_derivatives {
                    derivatives[i] = df * weight - dg * prev;
                }
                dg = df;
            }

            prev = weight;
            g = 1.0 - f;
        }

        weights[0] = g * prev;
        if has_derivs && p == degree {
            derivatives[0] = -dg * prev;
        }
    }

    knot.wrapping_sub(degree)
}

pub fn evaluate_nurbs_curve(curve: &NurbsCurve, u: Real) -> CurvePoint {
    let mut result = CurvePoint::default();

    let mut weights = [0.0; MAX_NURBS_ORDER];
    let mut derivs = [0.0; MAX_NURBS_ORDER];
    let base = evaluate_nurbs_basis(&curve.basis, u, &mut weights, &mut derivs);
    if base == usize::MAX {
        return result;
    }

    let mut p = Vec4::ZERO;
    let mut d = Vec4::ZERO;

    let order = curve.basis.order as usize;
    if order > MAX_NURBS_ORDER {
        return result;
    }
    let num_points = curve.control_points.len();
    if num_points == 0 {
        return result;
    }

    for i in 0..order {
        let ix = base.wrapping_add(i) % num_points;
        let cp = curve.control_points[ix];
        let (weight, deriv) = (weights[i] * cp.w, derivs[i] * cp.w);

        p.x += cp.x * weight;
        p.y += cp.y * weight;
        p.z += cp.z * weight;
        p.w += weight;

        d.x += cp.x * deriv;
        d.y += cp.y * deriv;
        d.z += cp.z * deriv;
        d.w += deriv;
    }

    let rcp_w = 1.0 / p.w;
    result.valid = true;
    result.position.x = p.x * rcp_w;
    result.position.y = p.y * rcp_w;
    result.position.z = p.z * rcp_w;
    result.derivative.x = (d.x - d.w * result.position.x) * rcp_w;
    result.derivative.y = (d.y - d.w * result.position.y) * rcp_w;
    result.derivative.z = (d.z - d.w * result.position.z) * rcp_w;
    result
}

pub fn evaluate_nurbs_surface(surface: &NurbsSurface, u: Real, v: Real) -> SurfacePoint {
    let mut result = SurfacePoint::default();

    let (mut weights_u, mut weights_v) = ([0.0; MAX_NURBS_ORDER], [0.0; MAX_NURBS_ORDER]);
    let (mut derivs_u, mut derivs_v) = ([0.0; MAX_NURBS_ORDER], [0.0; MAX_NURBS_ORDER]);
    let base_u = evaluate_nurbs_basis(&surface.basis_u, u, &mut weights_u, &mut derivs_u);
    let base_v = evaluate_nurbs_basis(&surface.basis_v, v, &mut weights_v, &mut derivs_v);
    if base_u == usize::MAX || base_v == usize::MAX {
        return result;
    }

    let mut p = Vec4::ZERO;
    let mut du = Vec4::ZERO;
    let mut dv = Vec4::ZERO;

    let num_u = surface.num_control_points_u;
    let num_v = surface.num_control_points_v;
    let order_u = surface.basis_u.order as usize;
    let order_v = surface.basis_v.order as usize;
    if order_u > MAX_NURBS_ORDER || order_v > MAX_NURBS_ORDER {
        return result;
    }
    if num_u == 0 || num_v == 0 {
        return result;
    }

    for vi in 0..order_v {
        let vix = base_v.wrapping_add(vi) % num_v;
        let (weight_v, deriv_v) = (weights_v[vi], derivs_v[vi]);

        for ui in 0..order_u {
            let uix = base_u.wrapping_add(ui) % num_u;
            let (weight_u, deriv_u) = (weights_u[ui], derivs_u[ui]);
            let Some(&cp) = surface.control_points.get(vix.wrapping_mul(num_u).wrapping_add(uix)) else { return SurfacePoint::default() };

            let weight = weight_u * weight_v * cp.w;
            let wderiv_u = deriv_u * weight_v * cp.w;
            let wderiv_v = deriv_v * weight_u * cp.w;

            p.x += cp.x * weight;
            p.y += cp.y * weight;
            p.z += cp.z * weight;
            p.w += weight;

            du.x += cp.x * wderiv_u;
            du.y += cp.y * wderiv_u;
            du.z += cp.z * wderiv_u;
            du.w += wderiv_u;

            dv.x += cp.x * wderiv_v;
            dv.y += cp.y * wderiv_v;
            dv.z += cp.z * wderiv_v;
            dv.w += wderiv_v;
        }
    }

    let rcp_w = 1.0 / p.w;
    result.valid = true;
    result.position.x = p.x * rcp_w;
    result.position.y = p.y * rcp_w;
    result.position.z = p.z * rcp_w;
    result.derivative_u.x = (du.x - du.w * result.position.x) * rcp_w;
    result.derivative_u.y = (du.y - du.w * result.position.y) * rcp_w;
    result.derivative_u.z = (du.z - du.w * result.position.z) * rcp_w;
    result.derivative_v.x = (dv.x - dv.w * result.position.x) * rcp_w;
    result.derivative_v.y = (dv.y - dv.w * result.position.y) * rcp_w;
    result.derivative_v.z = (dv.z - dv.w * result.position.z) * rcp_w;
    result
}

fn does_overflow(total: usize, a: usize, b: usize) -> bool {
    if ((a | b) >> (size_of::<usize>() * 4)) != 0 && a != 0 && total / a != b {
        return true;
    }
    false
}

fn tessellated_element(element_type: ElementType) -> Element {
    Element { element_type, typed_id: u32::MAX, element_id: u32::MAX, ..Default::default() }
}

pub fn tessellate_nurbs_curve(curve: &NurbsCurve, opts: &TessellateCurveOpts) -> Res<LineCurve> {
    tessellate_nurbs_curve_imp(curve, opts).map_err(|e| e.fix_error_type("Failed to tessellate"))
}

fn tessellate_nurbs_curve_imp(curve: &NurbsCurve, opts: &TessellateCurveOpts) -> Res<LineCurve> {
    let num_sub = if opts.span_subdivision == 0 { 4 } else { opts.span_subdivision };

    check!(curve.basis.valid && !curve.control_points.is_empty(), "Bad NURBS geometry");

    let spans = &curve.basis.spans;
    let num_spans = spans.len();

    let over_spans = num_spans.wrapping_mul(2).wrapping_mul(size_of::<Real>());
    let over = over_spans.wrapping_mul(num_sub);
    check!(!does_overflow(over, over_spans, num_sub));

    let is_open = curve.basis.topology == NurbsTopology::Open;

    let num_indices = num_spans.wrapping_add(num_spans.wrapping_sub(1).wrapping_mul(num_sub - 1));
    let num_vertices = num_indices.wrapping_sub(if is_open { 0 } else { 1 });
    check!(num_indices <= i32::MAX as usize);
    // a wrapped vertex count is where ufbx's allocation fails
    check!(num_vertices <= num_indices);

    let mut indices = vec![0u32; num_indices];
    let mut vertices = vec![Vec3::ZERO; num_vertices];

    for span_ix in 0..num_spans {
        let num_splits = if span_ix + 1 == num_spans { 1 } else { num_sub };

        for sub_ix in 0..num_splits {
            let ix = span_ix * num_sub + sub_ix;

            if ix < num_vertices {
                let mut u = spans[span_ix];
                if sub_ix > 0 {
                    let t = sub_ix as Real / num_sub as Real;
                    u = u * (1.0 - t) + t * spans[span_ix + 1];
                }

                let point = evaluate_nurbs_curve(curve, u);
                vertices[ix] = point.position;
                indices[ix] = ix as u32;
            } else {
                indices[ix] = 0;
            }
        }
    }

    Ok(LineCurve {
        element: tessellated_element(ElementType::LineCurve),
        color: Vec3::ONE,
        control_points: vertices,
        point_indices: indices,
        segments: vec![LineSegment { index_begin: 0, num_indices: num_indices as u32 }],
        from_tessellated_nurbs: true,
    })
}

pub fn tessellate_nurbs_surface(surface: &NurbsSurface, opts: &TessellateSurfaceOpts) -> Res<Mesh> {
    tessellate_nurbs_surface_imp(surface, opts).map_err(|e| e.fix_error_type("Failed to tessellate"))
}

fn tessellate_nurbs_surface_imp(surface: &NurbsSurface, opts: &TessellateSurfaceOpts) -> Res<Mesh> {
    let sub_u = if opts.span_subdivision_u == 0 { 4 } else { opts.span_subdivision_u };
    let sub_v = if opts.span_subdivision_v == 0 { 4 } else { opts.span_subdivision_v };

    check!(surface.basis_u.valid && surface.basis_v.valid && surface.num_control_points_u > 0 && surface.num_control_points_v > 0, "Bad NURBS geometry");

    let open_u = surface.basis_u.topology == NurbsTopology::Open;
    let open_v = surface.basis_v.topology == NurbsTopology::Open;

    let (knots_u, knots_v) = (&surface.basis_u.spans, &surface.basis_v.spans);
    let spans_u = knots_u.len();
    let spans_v = knots_v.len();

    {
        let over_spans_u = spans_u.wrapping_mul(2).wrapping_mul(size_of::<Real>());
        let over_spans_v = spans_v.wrapping_mul(2).wrapping_mul(size_of::<Real>());
        let over_u = over_spans_u.wrapping_mul(sub_u);
        let over_v = over_spans_v.wrapping_mul(sub_v);
        let over_uv = over_u.wrapping_mul(over_v);
        check!(!does_overflow(over_u, over_spans_u, sub_u));
        check!(!does_overflow(over_v, over_spans_v, sub_v));
        check!(!does_overflow(over_uv, over_u, over_v));
    }
    // ufbx's counts wrap and it reads out of bounds for a basis without spans (knots at minus infinity)
    check!(spans_u > 0 && spans_v > 0);

    let faces_u = (spans_u - 1) * sub_u;
    let faces_v = (spans_v - 1) * sub_v;

    let indices_u = spans_u + (spans_u - 1) * (sub_u - 1);
    let indices_v = spans_v + (spans_v - 1) * (sub_v - 1);

    let num_faces = faces_u * faces_v;
    let num_indices = indices_u * indices_v;
    check!(num_indices <= i32::MAX as usize);

    let mut position_ix = vec![0u32; num_indices];
    let mut positions = vec![Vec3::ZERO; num_indices];
    let mut uvs = vec![Vec2::ZERO; num_indices];
    let mut tangents = vec![Vec3::ZERO; num_indices];
    let mut bitangents = vec![Vec3::ZERO; num_indices];

    let mut num_positions: u32 = 0;

    for span_v in 0..spans_v {
        let splits_v = if span_v + 1 == spans_v { 1 } else { sub_v };

        for split_v in 0..splits_v {
            let ix_v = span_v * sub_v + split_v;

            let mut v = knots_v[span_v];
            if split_v > 0 {
                let t = split_v as Real / splits_v as Real;
                v = v * (1.0 - t) + t * knots_v[span_v + 1];
            }
            let original_v = v;
            if span_v + 1 == spans_v && !open_v {
                v = knots_v[0];
            }

            for span_u in 0..spans_u {
                let splits_u = if span_u + 1 == spans_u { 1 } else { sub_u };
                for split_u in 0..splits_u {
                    let ix_u = span_u * sub_u + split_u;

                    let mut u = knots_u[span_u];
                    if split_u > 0 {
                        let t = split_u as Real / splits_u as Real;
                        u = u * (1.0 - t) + t * knots_u[span_u + 1];
                    }
                    let original_u = u;
                    if span_u + 1 == spans_u && !open_u {
                        u = knots_u[0];
                    }

                    let point = evaluate_nurbs_surface(surface, u, v);
                    let pos = point.position;

                    let tangent_u = normalize3(point.derivative_u);
                    let tangent_v = normalize3(point.derivative_v);

                    let mut neighbors = [0usize; 5];
                    let mut num_neighbors = 0;

                    if (span_v == 0 && (span_u > 0 || split_u > 0)) || (span_u == 0 && (span_v > 0 || split_v > 0)) {
                        neighbors[num_neighbors] = 0;
                        num_neighbors += 1;
                    }
                    if span_v + 1 == spans_v {
                        neighbors[num_neighbors] = ix_u;
                        num_neighbors += 1;
                        if span_u > 0 || split_u > 0 {
                            neighbors[num_neighbors] = ix_v * indices_u;
                            num_neighbors += 1;
                        }
                    }
                    if span_u + 1 == spans_u {
                        neighbors[num_neighbors] = ix_v * indices_u;
                        num_neighbors += 1;
                        if span_v > 0 || split_v > 0 {
                            neighbors[num_neighbors] = indices_u - 1;
                            num_neighbors += 1;
                        }
                    }

                    let ix = ix_v * indices_u + ix_u;

                    let mut pos_ix = num_positions;
                    for &nb_ix in &neighbors[..num_neighbors] {
                        let nb_pos_ix = position_ix[nb_ix];
                        let nb_pos = positions[nb_pos_ix as usize];
                        let dx = nb_pos.x - pos.x;
                        let dy = nb_pos.y - pos.y;
                        let dz = nb_pos.z - pos.z;
                        let delta = dx * dx + dy * dy + dz * dz;
                        if delta < 0.0000001f32 as Real {
                            pos_ix = nb_pos_ix;
                            break;
                        }
                    }

                    position_ix[ix] = pos_ix;
                    if pos_ix == num_positions {
                        positions[pos_ix as usize] = pos;
                        num_positions = pos_ix + 1;
                    }
                    uvs[ix] = Vec2::new(original_u, original_v);
                    tangents[ix] = tangent_u;
                    bitangents[ix] = tangent_v;
                }
            }
        }
    }

    let mut faces = vec![Face::default(); num_faces];
    let mut vertex_ix = vec![0u32; num_faces * 4];
    let mut attrib_ix = vec![0u32; num_faces * 4];

    let mut face_ix = 0;
    let mut dst_index = 0;
    let mut num_triangles = 0;

    for face_v in 0..faces_v {
        for face_u in 0..faces_u {
            attrib_ix[dst_index] = (face_v * indices_u + face_u) as u32;
            attrib_ix[dst_index + 1] = (face_v * indices_u + (face_u + 1)) as u32;
            attrib_ix[dst_index + 2] = ((face_v + 1) * indices_u + (face_u + 1)) as u32;
            attrib_ix[dst_index + 3] = ((face_v + 1) * indices_u + face_u) as u32;

            for k in 0..4 {
                vertex_ix[dst_index + k] = position_ix[attrib_ix[dst_index + k] as usize];
            }

            let mut is_triangle = false;
            for prev_ix in 0..4 {
                let next_ix = (prev_ix + 1) % 4;
                if vertex_ix[dst_index + prev_ix] == vertex_ix[dst_index + next_ix] {
                    for i in next_ix..3 {
                        attrib_ix[dst_index + i] = attrib_ix[dst_index + i + 1];
                        vertex_ix[dst_index + i] = vertex_ix[dst_index + i + 1];
                    }
                    is_triangle = true;
                    break;
                }
            }

            faces[face_ix].index_begin = dst_index as u32;
            faces[face_ix].num_indices = if is_triangle { 3 } else { 4 };
            dst_index += if is_triangle { 3 } else { 4 };
            num_triangles += if is_triangle { 1 } else { 2 };
            face_ix += 1;
        }
    }

    let num_positions = num_positions as usize;
    positions.truncate(num_positions);
    vertex_ix.truncate(dst_index);
    attrib_ix.truncate(dst_index);

    // ufbx gives these `dst_index` values though it fills `num_indices`; keep every filled one
    let num_attrib_values = dst_index.max(num_indices);
    uvs.resize(num_attrib_values, Vec2::ZERO);
    tangents.resize(num_attrib_values, Vec3::ZERO);
    bitangents.resize(num_attrib_values, Vec3::ZERO);

    let by_vertex =
        |values: Vec<Vec3>, unique_per_vertex: bool| VertexVec3 { exists: true, values, indices: vertex_ix.clone(), unique_per_vertex, ..Default::default() };
    let by_attrib = |values: Vec<Vec3>| VertexVec3 { exists: true, values, indices: attrib_ix.clone(), ..Default::default() };

    let mut mesh = Mesh {
        element: tessellated_element(ElementType::Mesh),
        vertices: positions.clone(),
        num_vertices: num_positions,
        vertex_indices: vertex_ix.clone(),
        faces,
        vertex_position: by_vertex(positions, true),
        vertex_uv: VertexVec2 { exists: true, values: uvs, indices: attrib_ix.clone(), ..Default::default() },
        vertex_normal: by_vertex(vec![Vec3::ZERO; num_positions], false),
        vertex_tangent: by_attrib(tangents),
        vertex_bitangent: by_attrib(bitangents),
        num_faces,
        num_triangles,
        num_indices: dst_index,
        max_face_triangles: 2,
        ..Default::default()
    };

    // ufbx fills a zeroed `face_material` but leaves its count at zero, so the mesh shows none
    if let Some(material) = surface.material {
        mesh.materials = vec![Some(material)];
    }

    if !opts.skip_mesh_parts {
        mesh.material_parts = vec![MeshPart::default()];
    }

    finalize_mesh_material(&mut mesh)?;
    finalize_mesh(&mut mesh)?;

    mesh.generated_normals = true;
    let mut normals = std::mem::take(&mut mesh.vertex_normal.values);
    crate::topology::compute_normals(&mesh, &mesh.vertex_position, &mesh.vertex_normal.indices, &mut normals);

    if surface.flip_normals {
        for normal in &mut normals {
            normal.x *= -1.0;
            normal.y *= -1.0;
            normal.z *= -1.0;
        }
    }
    // ufbx's skinned normals share the normals' memory, filled after `finalize_mesh` copied the attribute
    mesh.skinned_normal.values = normals.clone();
    mesh.vertex_normal.values = normals;

    mesh.subdivision_evaluated = true;
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorType;

    fn basis(order: u32, knots: &[Real], topology: NurbsTopology) -> NurbsBasis {
        let degree = order as usize - 1;
        let mut spans = Vec::new();
        for &t in &knots[degree..knots.len() - degree] {
            if spans.last() != Some(&t) {
                spans.push(t);
            }
        }
        NurbsBasis {
            order,
            topology,
            knot_vector: knots.to_vec(),
            t_min: knots[degree],
            t_max: knots[knots.len() - degree - 1],
            spans,
            valid: true,
            ..Default::default()
        }
    }

    fn close(a: Real, b: Real) -> bool {
        (a - b).abs() < 1e-12
    }

    fn bezier_curve() -> NurbsCurve {
        NurbsCurve {
            basis: basis(4, &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0], NurbsTopology::Open),
            control_points: vec![Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(1.0, 2.0, 0.0, 1.0), Vec4::new(3.0, 2.0, 0.0, 1.0), Vec4::new(4.0, 0.0, 0.0, 1.0)],
            ..Default::default()
        }
    }

    fn plane(sub_material: Option<usize>) -> NurbsSurface {
        let b = basis(2, &[0.0, 0.0, 1.0, 1.0], NurbsTopology::Open);
        NurbsSurface {
            basis_u: b.clone(),
            basis_v: b,
            num_control_points_u: 2,
            num_control_points_v: 2,
            control_points: vec![Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(2.0, 0.0, 0.0, 1.0), Vec4::new(0.0, 2.0, 0.0, 1.0), Vec4::new(2.0, 2.0, 0.0, 1.0)],
            material: sub_material,
            ..Default::default()
        }
    }

    #[test]
    fn basis_of_a_cubic_bezier_is_bernstein() {
        let b = basis(4, &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0], NurbsTopology::Open);
        let (mut w, mut d) = ([0.0; 4], [0.0; 4]);
        assert_eq!(evaluate_nurbs_basis(&b, 0.5, &mut w, &mut d), 0);
        for (got, want) in w.iter().zip([0.125, 0.375, 0.375, 0.125]) {
            assert!(close(*got, want), "{w:?}");
        }
        for (got, want) in d.iter().zip([-0.75, -0.75, 0.75, 0.75]) {
            assert!(close(*got, want), "{d:?}");
        }

        assert_eq!(evaluate_nurbs_basis(&b, -3.0, &mut w, &mut []), 0);
        assert_eq!(w, [1.0, 0.0, 0.0, 0.0]);

        let mut short = [7.0; 3];
        assert_eq!(evaluate_nurbs_basis(&b, 0.5, &mut short, &mut []), 0);
        assert_eq!(short, [7.0; 3]);

        let invalid = NurbsBasis { valid: false, ..b.clone() };
        assert_eq!(evaluate_nurbs_basis(&invalid, 0.5, &mut w, &mut d), usize::MAX);
        assert_eq!(evaluate_nurbs_basis(&NurbsBasis::default(), 0.5, &mut w, &mut d), usize::MAX);
    }

    #[test]
    fn basis_finds_the_span_of_u() {
        let b = basis(3, &[0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0], NurbsTopology::Open);
        let mut w = [0.0; 3];
        assert_eq!(evaluate_nurbs_basis(&b, 0.5, &mut w, &mut []), 0);
        assert_eq!(evaluate_nurbs_basis(&b, 1.0, &mut w, &mut []), 1);
        assert_eq!(evaluate_nurbs_basis(&b, 2.5, &mut w, &mut []), 2);
        assert_eq!(evaluate_nurbs_basis(&b, 9.0, &mut w, &mut []), 2);
        assert!(close(w.iter().sum(), 1.0));
    }

    #[test]
    fn rational_quadratic_is_a_circle() {
        let h = std::f64::consts::FRAC_1_SQRT_2;
        let curve = NurbsCurve {
            basis: basis(3, &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0], NurbsTopology::Open),
            control_points: vec![Vec4::new(1.0, 0.0, 0.0, 1.0), Vec4::new(1.0, 1.0, 0.0, h), Vec4::new(0.0, 1.0, 0.0, 1.0)],
            ..Default::default()
        };
        for u in [0.0, 0.1, 0.5, 0.9, 1.0] {
            let p = evaluate_nurbs_curve(&curve, u);
            assert!(p.valid);
            assert!(close(p.position.x.hypot(p.position.y), 1.0), "{u}: {p:?}");
            assert!(close(p.position.x * p.derivative.x + p.position.y * p.derivative.y, 0.0), "{u}: {p:?}");
        }
        assert!(!evaluate_nurbs_curve(&NurbsCurve::default(), 0.5).valid);
        assert!(!evaluate_nurbs_curve(&NurbsCurve { control_points: Vec::new(), ..curve }, 0.5).valid);
    }

    #[test]
    fn bilinear_surface_interpolates_its_corners() {
        let s = plane(None);
        let p = evaluate_nurbs_surface(&s, 0.5, 0.25);
        assert!(p.valid);
        assert_eq!(p.position, Vec3::new(1.0, 0.5, 0.0));
        assert_eq!(p.derivative_u, Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(p.derivative_v, Vec3::new(0.0, 2.0, 0.0));
        assert!(!evaluate_nurbs_surface(&NurbsSurface { num_control_points_v: 0, ..s }, 0.5, 0.5).valid);
    }

    #[test]
    fn curve_tessellation_open_and_closed() {
        let curve = bezier_curve();
        let line = tessellate_nurbs_curve(&curve, &TessellateCurveOpts::default()).unwrap();
        assert_eq!(line.point_indices, [0, 1, 2, 3, 4]);
        assert_eq!(line.control_points.len(), 5);
        assert_eq!(line.control_points[0], Vec3::ZERO);
        assert_eq!(line.control_points[4], Vec3::new(4.0, 0.0, 0.0));
        assert_eq!(line.control_points[2], evaluate_nurbs_curve(&curve, 0.5).position);
        assert_eq!(line.segments, [LineSegment { index_begin: 0, num_indices: 5 }]);
        assert_eq!(line.color, Vec3::ONE);
        assert!(line.from_tessellated_nurbs);
        assert_eq!((line.element.element_type, line.element.element_id, line.element.typed_id), (ElementType::LineCurve, u32::MAX, u32::MAX));

        let closed = NurbsCurve { basis: NurbsBasis { topology: NurbsTopology::Closed, ..curve.basis.clone() }, ..curve.clone() };
        let line = tessellate_nurbs_curve(&closed, &TessellateCurveOpts { span_subdivision: 2 }).unwrap();
        assert_eq!(line.point_indices, [0, 1, 0]);
        assert_eq!(line.control_points.len(), 2);

        let bad = NurbsCurve { control_points: Vec::new(), ..curve };
        let err = tessellate_nurbs_curve(&bad, &TessellateCurveOpts::default()).unwrap_err();
        assert_eq!((err.kind, err.description.as_str()), (ErrorType::BadNurbs, "Bad NURBS geometry"));
    }

    #[test]
    fn plane_tessellates_into_quads() {
        let mesh =
            tessellate_nurbs_surface(&plane(Some(3)), &TessellateSurfaceOpts { span_subdivision_u: 2, span_subdivision_v: 2, skip_mesh_parts: false }).unwrap();
        assert_eq!((mesh.num_vertices, mesh.num_faces, mesh.num_indices, mesh.num_triangles), (9, 4, 16, 8));
        assert_eq!(mesh.faces[1], Face { index_begin: 4, num_indices: 4 });
        assert_eq!(&mesh.vertex_indices[..4], [0, 1, 4, 3]);
        assert_eq!(mesh.vertices[4], Vec3::new(1.0, 1.0, 0.0));
        assert!(mesh.vertex_normal.values.iter().all(|n| *n == Vec3::new(0.0, 0.0, 1.0)));
        assert_eq!(mesh.skinned_normal.values, mesh.vertex_normal.values);
        assert_eq!(mesh.vertex_uv.get(6), Vec2::new(1.0, 0.5));
        assert_eq!(mesh.vertex_tangent.get(0), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(mesh.vertex_bitangent.get(0), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(mesh.materials, [Some(3)]);
        assert!(mesh.face_material.is_empty());
        assert_eq!(mesh.material_parts.len(), 1);
        assert_eq!(mesh.material_parts[0].face_indices, [0, 1, 2, 3]);
        assert_eq!(mesh.uv_sets.len(), 1);
        assert!(mesh.generated_normals && mesh.subdivision_evaluated && !mesh.from_tessellated_nurbs);

        let flipped = NurbsSurface { flip_normals: true, ..plane(None) };
        let mesh = tessellate_nurbs_surface(&flipped, &TessellateSurfaceOpts { skip_mesh_parts: true, ..Default::default() }).unwrap();
        assert_eq!((mesh.num_vertices, mesh.num_faces), (25, 16));
        assert!(mesh.vertex_normal.values.iter().all(|n| *n == Vec3::new(0.0, 0.0, -1.0)));
        assert!(mesh.materials.is_empty() && mesh.material_parts.is_empty());
    }

    #[test]
    fn collapsed_edge_makes_triangles() {
        let mut cone = plane(None);
        cone.control_points[2] = Vec4::new(1.0, 2.0, 0.0, 1.0);
        cone.control_points[3] = Vec4::new(1.0, 2.0, 0.0, 1.0);
        let mesh = tessellate_nurbs_surface(&cone, &TessellateSurfaceOpts { span_subdivision_u: 2, span_subdivision_v: 1, ..Default::default() }).unwrap();
        assert_eq!(mesh.num_vertices, 4);
        assert_eq!(mesh.faces, [Face { index_begin: 0, num_indices: 3 }, Face { index_begin: 3, num_indices: 3 }]);
        assert_eq!(mesh.vertex_indices, [0, 1, 3, 1, 2, 3]);
        assert_eq!(mesh.num_triangles, 2);
    }
}

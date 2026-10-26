mod ufbxt;
use ufbxt::*;

const ROTATION_ORDERS: [RotationOrder; 6] =
    [RotationOrder::Xyz, RotationOrder::Xzy, RotationOrder::Yzx, RotationOrder::Yxz, RotationOrder::Zxy, RotationOrder::Zyx];

fn quat_error(a: Quat, b: Quat) -> Real {
    let pos = (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs() + (a.w - b.w).abs();
    let neg = (a.x + b.x).abs() + (a.y + b.y).abs() + (a.z + b.z).abs() + (a.w + b.w).abs();
    if pos < neg { pos } else { neg }
}

fn xorshift32(state: &mut u32) -> u32 {
    // Algorithm "xor" from p. 4 of Marsaglia, "Xorshift RNGs"
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

fn xorshift32_real(state: &mut u32) -> Real {
    let u = xorshift32(state);
    u as Real * 2.3283064365386962890625e-10
}

#[test]
fn quat_to_euler_structured() {
    let mut err = DiffError::default();

    for order in ROTATION_ORDERS {
        for x in (-360..=360).step_by(45) {
            for y in (-360..=360).step_by(45) {
                for z in (-360..=360).step_by(45) {
                    let v = Vec3::new(x as Real, y as Real, z as Real);

                    let q = euler_to_quat(v, order);
                    let v2 = quat_to_euler(q, order);
                    let q2 = euler_to_quat(v2, order);

                    assert_close_real(&mut err, quat_error(q, q2), 0.0);
                    assert_close_real(&mut err, quat_error(q, q2), 0.0);
                }
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn quat_to_euler_random() {
    let fuzz = begin_fuzz();
    let steps = if fuzz { 10000000 } else { 100000 };
    let mut err = DiffError::default();

    for order in ROTATION_ORDERS {
        let mut state = 1u32;

        for i in 0..steps {
            if fuzz && fuzz_should_skip(i >> 8, fuzz_quality()) {
                continue;
            }

            let mut q = Quat::default();
            q.x = xorshift32_real(&mut state) * 2.0 - 1.0;
            q.y = xorshift32_real(&mut state) * 2.0 - 1.0;
            q.z = xorshift32_real(&mut state) * 2.0 - 1.0;
            q.w = xorshift32_real(&mut state) * 2.0 - 1.0;
            let qm = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
            q.x /= qm;
            q.y /= qm;
            q.z /= qm;
            q.w /= qm;

            let v = quat_to_euler(q, order);
            let q2 = euler_to_quat(v, order);

            assert_close_real(&mut err, quat_error(q, q2), 0.0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn matrix_to_transform_structured() {
    let mut err = DiffError::default();

    for sx in -2..=2 {
        for sy in -2..=2 {
            for sz in -2..=2 {
                for rx in -2..=2 {
                    for ry in -2..=2 {
                        for rz in -2..=2 {
                            for rw in -2..=2 {
                                if sx == 0 || sy == 0 || sz == 0 {
                                    continue;
                                }
                                if rx == 0 && ry == 0 && rz == 0 && rw == 0 {
                                    continue;
                                }

                                let mut t = Transform::default();

                                let q = Quat::new(rx as Real / 3.0, ry as Real / 3.0, rz as Real / 3.0, rw as Real / 3.0);
                                let qm = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
                                t.rotation.x = q.x / qm;
                                t.rotation.y = q.y / qm;
                                t.rotation.z = q.z / qm;
                                t.rotation.w = q.w / qm;

                                t.translation.x = 1.0;
                                t.translation.y = 2.0;
                                t.translation.z = 3.0;
                                t.scale.x = sx as Real / 2.0;
                                t.scale.y = sy as Real / 2.0;
                                t.scale.z = sz as Real / 2.0;

                                let m = transform_to_matrix(&t);
                                let t2 = matrix_to_transform(&m);

                                if sx < 0 || sy < 0 || sz < 0 {
                                    // Flipped signs cannot be uniquely recovered, check that the transforms are identical
                                    let m2 = transform_to_matrix(&t2);
                                    assert_close_matrix(&mut err, m, m2);
                                } else {
                                    assert_close_vec3(&mut err, t.translation, t2.translation);
                                    assert_close_vec3(&mut err, t.scale, t2.scale);
                                    assert_close_real(&mut err, quat_error(t.rotation, t2.rotation), 0.0);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn matrix_to_transform_random() {
    let mut err = DiffError::default();

    let mut state = 1u32;
    let fuzz = begin_fuzz();
    let steps = if fuzz { 1000000 } else { 100000 };

    for i in 0..steps {
        if fuzz && fuzz_should_skip(i >> 4, fuzz_quality()) {
            continue;
        }

        let mut t = Transform::default();

        let mut q = Quat::default();
        q.x = xorshift32_real(&mut state) * 2.0 - 1.0;
        q.y = xorshift32_real(&mut state) * 2.0 - 1.0;
        q.z = xorshift32_real(&mut state) * 2.0 - 1.0;
        q.w = xorshift32_real(&mut state) * 2.0 - 1.0;
        let qm = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
        t.rotation.x = q.x / qm;
        t.rotation.y = q.y / qm;
        t.rotation.z = q.z / qm;
        t.rotation.w = q.w / qm;

        t.translation.x = xorshift32_real(&mut state) * 20.0 - 10.0;
        t.translation.y = xorshift32_real(&mut state) * 20.0 - 10.0;
        t.translation.z = xorshift32_real(&mut state) * 20.0 - 10.0;
        t.scale.x = xorshift32_real(&mut state) * 10.0 + 0.01f32 as Real;
        t.scale.y = xorshift32_real(&mut state) * 10.0 + 0.01f32 as Real;
        t.scale.z = xorshift32_real(&mut state) * 10.0 + 0.01f32 as Real;

        let mut flip = xorshift32(&mut state);

        // Prevent most of the inputs being flips
        if flip & 8 != 0 {
            flip = 0;
        }

        if flip & 1 != 0 {
            t.scale.x *= -1.0;
        }
        if flip & 2 != 0 {
            t.scale.y *= -1.0;
        }
        if flip & 4 != 0 {
            t.scale.z *= -1.0;
        }

        let m = transform_to_matrix(&t);
        let t2 = matrix_to_transform(&m);

        if flip != 0 {
            // Flipped signs cannot be uniquely recovered, check that the transforms are identical
            let m2 = transform_to_matrix(&t2);
            assert_close_matrix(&mut err, m, m2);
        } else {
            assert_close_vec3(&mut err, t.translation, t2.translation);
            assert_close_vec3(&mut err, t.scale, t2.scale);
            assert_close_real(&mut err, quat_error(t.rotation, t2.rotation), 0.0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn matrix_inverse_simple() {
    let mut err = DiffError::default();

    {
        let mut m = Matrix::ZERO;
        m.m00 = 2.0;
        m.m11 = 0.5;
        m.m22 = 0.25;
        m.m03 = 1.0;
        m.m13 = 2.0;
        m.m23 = 3.0;

        let im = matrix_invert(&m);
        assert_close_real(&mut err, im.m00, 0.5);
        assert_close_real(&mut err, im.m10, 0.0);
        assert_close_real(&mut err, im.m20, 0.0);
        assert_close_real(&mut err, im.m01, 0.0);
        assert_close_real(&mut err, im.m11, 2.0);
        assert_close_real(&mut err, im.m21, 0.0);
        assert_close_real(&mut err, im.m02, 0.0);
        assert_close_real(&mut err, im.m12, 0.0);
        assert_close_real(&mut err, im.m22, 4.0);
        assert_close_real(&mut err, im.m03, -0.5);
        assert_close_real(&mut err, im.m13, -4.0);
        assert_close_real(&mut err, im.m23, -12.0);
    }

    {
        let mut m = Matrix::ZERO;
        m.m00 = 1.0;
        m.m12 = -1.0;
        m.m21 = 1.0;
        m.m13 = 1.0;
        m.m23 = 2.0;

        let im = matrix_invert(&m);
        assert_close_real(&mut err, im.m00, 1.0);
        assert_close_real(&mut err, im.m10, 0.0);
        assert_close_real(&mut err, im.m20, 0.0);
        assert_close_real(&mut err, im.m01, 0.0);
        assert_close_real(&mut err, im.m11, 0.0);
        assert_close_real(&mut err, im.m21, -1.0);
        assert_close_real(&mut err, im.m02, 0.0);
        assert_close_real(&mut err, im.m12, 1.0);
        assert_close_real(&mut err, im.m22, 0.0);
        assert_close_real(&mut err, im.m03, 0.0);
        assert_close_real(&mut err, im.m13, -2.0);
        assert_close_real(&mut err, im.m23, 1.0);
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn matrix_inverse_random() {
    let mut err = DiffError::default();

    let fuzz = begin_fuzz();
    let steps = if fuzz { 1000000 } else { 10000 };

    let mut state = 1u32;

    for i in 0..steps {
        if fuzz && fuzz_should_skip(i >> 4, fuzz_quality()) {
            continue;
        }

        let mut t = Transform::default();

        let mut q = Quat::default();
        q.x = xorshift32_real(&mut state) * 2.0 - 1.0;
        q.y = xorshift32_real(&mut state) * 2.0 - 1.0;
        q.z = xorshift32_real(&mut state) * 2.0 - 1.0;
        q.w = xorshift32_real(&mut state) * 2.0 - 1.0;
        let qm = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
        t.rotation.x = q.x / qm;
        t.rotation.y = q.y / qm;
        t.rotation.z = q.z / qm;
        t.rotation.w = q.w / qm;

        t.translation.x = xorshift32_real(&mut state) * 20.0 - 10.0;
        t.translation.y = xorshift32_real(&mut state) * 20.0 - 10.0;
        t.translation.z = xorshift32_real(&mut state) * 20.0 - 10.0;
        t.scale.x = xorshift32_real(&mut state) * 10.0 + 0.1f32 as Real;
        t.scale.y = xorshift32_real(&mut state) * 10.0 + 0.1f32 as Real;
        t.scale.z = xorshift32_real(&mut state) * 10.0 + 0.1f32 as Real;

        let mut flip = xorshift32(&mut state);

        // Prevent most of the inputs being flips
        if flip & 8 != 0 {
            flip = 0;
        }

        if flip & 1 != 0 {
            t.scale.x *= -1.0;
        }
        if flip & 2 != 0 {
            t.scale.y *= -1.0;
        }
        if flip & 4 != 0 {
            t.scale.z *= -1.0;
        }

        let m = transform_to_matrix(&t);
        let im = matrix_invert(&m);
        let identity = matrix_mul(&m, &im);

        assert_close_real(&mut err, identity.m00, 1.0);
        assert_close_real(&mut err, identity.m10, 0.0);
        assert_close_real(&mut err, identity.m20, 0.0);
        assert_close_real(&mut err, identity.m01, 0.0);
        assert_close_real(&mut err, identity.m11, 1.0);
        assert_close_real(&mut err, identity.m21, 0.0);
        assert_close_real(&mut err, identity.m02, 0.0);
        assert_close_real(&mut err, identity.m12, 0.0);
        assert_close_real(&mut err, identity.m22, 1.0);
        assert_close_real(&mut err, identity.m03, 0.0);
        assert_close_real(&mut err, identity.m13, 0.0);
        assert_close_real(&mut err, identity.m23, 0.0);
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn quat_normalize() {
    let mut err = DiffError::default();

    {
        let q = Quat::new(1.0, -1.0, 2.0, 0.0);
        let n = load3d::quat_normalize(q);

        let len = ((1 * 1 + 1 * 1 + 2 * 2) as Real).sqrt();
        let mut reference = Quat::default();
        reference.x = 1.0 / len;
        reference.y = -1.0 / len;
        reference.z = 2.0 / len;
        reference.w = 0.0;
        assert_close_quat(&mut err, n, reference);
    }

    {
        let q = Quat::new(0.0, 0.0, 0.0, 0.0);
        let n = load3d::quat_normalize(q);
        assert_close_quat(&mut err, n, Quat::IDENTITY);
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn quat_mul() {
    let mut err = DiffError::default();

    for order in ROTATION_ORDERS {
        let indices: [usize; 3] = match order {
            RotationOrder::Xyz => [0, 1, 2],
            RotationOrder::Xzy => [0, 2, 1],
            RotationOrder::Yzx => [1, 2, 0],
            RotationOrder::Yxz => [1, 0, 2],
            RotationOrder::Zxy => [2, 0, 1],
            RotationOrder::Zyx => [2, 1, 0],
            RotationOrder::Spheric => panic!("Bad order"),
        };

        for x in (-360..=360).step_by(45) {
            for y in (-360..=360).step_by(45) {
                for z in (-360..=360).step_by(45) {
                    let v = Vec3::new(x as Real, y as Real, z as Real);
                    let mut vs = [Vec3::ZERO; 3];
                    vs[0].x = v.x;
                    vs[1].y = v.y;
                    vs[2].z = v.z;

                    let q0 = euler_to_quat(vs[indices[2]], RotationOrder::Xyz);
                    let q1 = euler_to_quat(vs[indices[1]], RotationOrder::Yzx);
                    let q2 = euler_to_quat(vs[indices[0]], RotationOrder::Zxy);

                    let q01 = load3d::quat_mul(q0, q1);
                    let q012 = load3d::quat_mul(q01, q2);

                    let reference = euler_to_quat(v, order);

                    assert_close_quat(&mut err, q012, reference);
                    assert_close_quat(&mut err, q012, reference);
                }
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn matrix_for_normals_scale() {
    let mut err = DiffError::default();

    {
        let mut geometry_to_world = Matrix::ZERO;
        geometry_to_world.m00 = 2.0;
        geometry_to_world.m11 = 2.0;
        geometry_to_world.m22 = 2.0;
        geometry_to_world.m03 = 1.0;
        geometry_to_world.m13 = 2.0;
        geometry_to_world.m23 = 3.0;

        let normal_to_world = matrix_for_normals(&geometry_to_world);

        let basis = [Vec3::new(4.0, 0.0, 0.0), Vec3::new(0.0, 4.0, 0.0), Vec3::new(0.0, 0.0, 4.0)];
        assert_close_vec3(&mut err, normal_to_world.col(0), basis[0]);
        assert_close_vec3(&mut err, normal_to_world.col(1), basis[1]);
        assert_close_vec3(&mut err, normal_to_world.col(2), basis[2]);
        assert_close_vec3(&mut err, normal_to_world.col(3), Vec3::ZERO);
    }

    {
        let mut geometry_to_world = Matrix::ZERO;
        geometry_to_world.m00 = 2.0;
        geometry_to_world.m11 = 1.0;
        geometry_to_world.m22 = 1.0;

        let normal_to_world = matrix_for_normals(&geometry_to_world);

        let basis = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 0.0, 2.0)];
        assert_close_vec3(&mut err, normal_to_world.col(0), basis[0]);
        assert_close_vec3(&mut err, normal_to_world.col(1), basis[1]);
        assert_close_vec3(&mut err, normal_to_world.col(2), basis[2]);
        assert_close_vec3(&mut err, normal_to_world.col(3), Vec3::ZERO);

        let geo_normal = Vec3::new(1.0, 1.0, 0.0);
        let world_normal = transform_direction(&normal_to_world, geo_normal);
        let geo_tangent = Vec3::new(1.0, -1.0, 0.0);
        let geo_bitangent = Vec3::new(0.0, 0.0, -1.0);
        let world_tangent = transform_direction(&geometry_to_world, geo_tangent);
        let world_bitangent = transform_direction(&geometry_to_world, geo_bitangent);

        let normal = vec3_normalize(world_normal);
        let ref_normal = vec3_normalize(cross3(world_tangent, world_bitangent));
        assert_close_vec3(&mut err, normal, ref_normal);
    }

    {
        let mut geometry_to_world = Matrix::ZERO;
        geometry_to_world.m00 = 1.0;
        geometry_to_world.m11 = 0.0;
        geometry_to_world.m22 = 1.0;

        let normal_to_world = matrix_for_normals(&geometry_to_world);

        let basis = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 0.0)];
        assert_close_vec3(&mut err, normal_to_world.col(0), basis[0]);
        assert_close_vec3(&mut err, normal_to_world.col(1), basis[1]);
        assert_close_vec3(&mut err, normal_to_world.col(2), basis[2]);
        assert_close_vec3(&mut err, normal_to_world.col(3), Vec3::ZERO);

        let geo_normal = Vec3::new(1.0, 1.0, 0.0);
        let world_normal = transform_direction(&normal_to_world, geo_normal);
        let geo_tangent = Vec3::new(1.0, -1.0, 0.0);
        let geo_bitangent = Vec3::new(0.0, 0.0, -1.0);
        let world_tangent = transform_direction(&geometry_to_world, geo_tangent);
        let world_bitangent = transform_direction(&geometry_to_world, geo_bitangent);

        let normal = vec3_normalize(world_normal);
        let ref_normal = vec3_normalize(cross3(world_tangent, world_bitangent));
        assert_close_vec3(&mut err, normal, ref_normal);

        let up_normal = Vec3::new(0.0, 1.0, 0.0);
        assert_close_vec3(&mut err, normal, up_normal);
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

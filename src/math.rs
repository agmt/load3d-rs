//! Arithmetic stays operation for operation as ufbx's, so that every value comes out with its bits.

use std::ops::{Index, IndexMut};

pub type Real = f64;

/// Small enough that its square is not denormal
pub const EPSILON: Real = 1.4916681462400413e-154;
pub(crate) const DPI: f64 = std::f64::consts::PI;
pub(crate) const DEG_TO_RAD_DOUBLE: f64 = DPI / 180.0;
pub(crate) const RAD_TO_DEG_DOUBLE: f64 = 180.0 / DPI;
pub(crate) const DEG_TO_RAD: Real = DPI / 180.0;
pub(crate) const RAD_TO_DEG: Real = 180.0 / DPI;
pub(crate) const MM_TO_INCH: Real = 0.0393700787;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: Real,
    pub y: Real,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: Real,
    pub y: Real,
    pub z: Real,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec4 {
    pub x: Real,
    pub y: Real,
    pub z: Real,
    pub w: Real,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: Real,
    pub y: Real,
    pub z: Real,
    pub w: Real,
}

// zeroed, as ufbx's memory is wherever it sets no value; `IDENTITY` is the unit rotation
impl Default for Quat {
    fn default() -> Quat {
        Quat { x: 0.0, y: 0.0, z: 0.0, w: 0.0 }
    }
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub const fn new(x: Real, y: Real) -> Vec2 {
        Vec2 { x, y }
    }
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Vec3 = Vec3 { x: 1.0, y: 1.0, z: 1.0 };
    pub const fn new(x: Real, y: Real, z: Real) -> Vec3 {
        Vec3 { x, y, z }
    }
    pub fn to_array(self) -> [Real; 3] {
        [self.x, self.y, self.z]
    }
    pub fn from_array(a: [Real; 3]) -> Vec3 {
        Vec3 { x: a[0], y: a[1], z: a[2] }
    }
}

impl Vec4 {
    pub const ZERO: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const fn new(x: Real, y: Real, z: Real, w: Real) -> Vec4 {
        Vec4 { x, y, z, w }
    }
}

impl Quat {
    pub const IDENTITY: Quat = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
    pub const fn new(x: Real, y: Real, z: Real, w: Real) -> Quat {
        Quat { x, y, z, w }
    }
}

macro_rules! index_impl {
    ($t:ty, $($i:literal => $f:ident),*) => {
        impl Index<usize> for $t {
            type Output = Real;
            fn index(&self, i: usize) -> &Real {
                match i { $($i => &self.$f,)* _ => panic!("index out of range") }
            }
        }
        impl IndexMut<usize> for $t {
            fn index_mut(&mut self, i: usize) -> &mut Real {
                match i { $($i => &mut self.$f,)* _ => panic!("index out of range") }
            }
        }
    };
}
index_impl!(Vec2, 0 => x, 1 => y);
index_impl!(Vec3, 0 => x, 1 => y, 2 => z);
index_impl!(Vec4, 0 => x, 1 => y, 2 => z, 3 => w);
index_impl!(Quat, 0 => x, 1 => y, 2 => z, 3 => w);

/// Translation, rotation (a quaternion, not Euler angles) and scale
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Transform {
        Transform { translation: Vec3::ZERO, rotation: Quat::default(), scale: Vec3::ZERO }
    }
}

impl Transform {
    pub const IDENTITY: Transform = Transform { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };
}

/// An affine 4x3 matrix: `cols()[0..3]` the basis vectors and `cols()[3]` the translation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub m00: Real,
    pub m10: Real,
    pub m20: Real,
    pub m01: Real,
    pub m11: Real,
    pub m21: Real,
    pub m02: Real,
    pub m12: Real,
    pub m22: Real,
    pub m03: Real,
    pub m13: Real,
    pub m23: Real,
}

impl Default for Matrix {
    fn default() -> Matrix {
        Matrix::ZERO
    }
}

impl Matrix {
    pub const IDENTITY: Matrix =
        Matrix { m00: 1.0, m10: 0.0, m20: 0.0, m01: 0.0, m11: 1.0, m21: 0.0, m02: 0.0, m12: 0.0, m22: 1.0, m03: 0.0, m13: 0.0, m23: 0.0 };
    pub const ZERO: Matrix = Matrix { m00: 0.0, m10: 0.0, m20: 0.0, m01: 0.0, m11: 0.0, m21: 0.0, m02: 0.0, m12: 0.0, m22: 0.0, m03: 0.0, m13: 0.0, m23: 0.0 };

    /// The twelve values in ufbx's memory order
    pub fn v(&self) -> [Real; 12] {
        [self.m00, self.m10, self.m20, self.m01, self.m11, self.m21, self.m02, self.m12, self.m22, self.m03, self.m13, self.m23]
    }

    pub fn from_v(v: [Real; 12]) -> Matrix {
        Matrix { m00: v[0], m10: v[1], m20: v[2], m01: v[3], m11: v[4], m21: v[5], m02: v[6], m12: v[7], m22: v[8], m03: v[9], m13: v[10], m23: v[11] }
    }

    pub fn col(&self, i: usize) -> Vec3 {
        match i {
            0 => Vec3::new(self.m00, self.m10, self.m20),
            1 => Vec3::new(self.m01, self.m11, self.m21),
            2 => Vec3::new(self.m02, self.m12, self.m22),
            _ => Vec3::new(self.m03, self.m13, self.m23),
        }
    }

    pub fn cols(&self) -> [Vec3; 4] {
        [self.col(0), self.col(1), self.col(2), self.col(3)]
    }

    pub fn set_col(&mut self, i: usize, c: Vec3) {
        match i {
            0 => (self.m00, self.m10, self.m20) = (c.x, c.y, c.z),
            1 => (self.m01, self.m11, self.m21) = (c.x, c.y, c.z),
            2 => (self.m02, self.m12, self.m22) = (c.x, c.y, c.z),
            _ => (self.m03, self.m13, self.m23) = (c.x, c.y, c.z),
        }
    }

    pub fn from_cols(c: [Vec3; 4]) -> Matrix {
        let mut m = Matrix::ZERO;
        for (i, v) in c.into_iter().enumerate() {
            m.set_col(i, v);
        }
        m
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CoordinateAxis {
    // zeroed memory in ufbx, as every field a C struct leaves unset
    #[default]
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
    Unknown,
}

impl CoordinateAxis {
    pub fn from_index(i: i64) -> CoordinateAxis {
        match i {
            0 => CoordinateAxis::PositiveX,
            1 => CoordinateAxis::NegativeX,
            2 => CoordinateAxis::PositiveY,
            3 => CoordinateAxis::NegativeY,
            4 => CoordinateAxis::PositiveZ,
            5 => CoordinateAxis::NegativeZ,
            _ => CoordinateAxis::Unknown,
        }
    }
}

/// What world direction the X, Y and Z axes point. `front` is the opposite of forward
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoordinateAxes {
    pub right: CoordinateAxis,
    pub up: CoordinateAxis,
    pub front: CoordinateAxis,
}

impl CoordinateAxes {
    pub const RIGHT_HANDED_Y_UP: CoordinateAxes =
        CoordinateAxes { right: CoordinateAxis::PositiveX, up: CoordinateAxis::PositiveY, front: CoordinateAxis::PositiveZ };
    pub const RIGHT_HANDED_Z_UP: CoordinateAxes =
        CoordinateAxes { right: CoordinateAxis::PositiveX, up: CoordinateAxis::PositiveZ, front: CoordinateAxis::NegativeY };
    pub const LEFT_HANDED_Y_UP: CoordinateAxes =
        CoordinateAxes { right: CoordinateAxis::PositiveX, up: CoordinateAxis::PositiveY, front: CoordinateAxis::NegativeZ };
    pub const LEFT_HANDED_Z_UP: CoordinateAxes =
        CoordinateAxes { right: CoordinateAxis::PositiveX, up: CoordinateAxis::PositiveZ, front: CoordinateAxis::PositiveY };
    pub const UNKNOWN: CoordinateAxes = CoordinateAxes { right: CoordinateAxis::Unknown, up: CoordinateAxis::Unknown, front: CoordinateAxis::Unknown };
}

/// The order the axes are applied in, `Xyz` being `Z*Y*X`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RotationOrder {
    #[default]
    Xyz,
    Xzy,
    Yzx,
    Yxz,
    Zxy,
    Zyx,
    Spheric,
}

impl RotationOrder {
    pub fn from_index(i: i64) -> RotationOrder {
        match i {
            1 => RotationOrder::Xzy,
            2 => RotationOrder::Yzx,
            3 => RotationOrder::Yxz,
            4 => RotationOrder::Zxy,
            5 => RotationOrder::Zyx,
            6 => RotationOrder::Spheric,
            _ => RotationOrder::Xyz,
        }
    }
}

pub(crate) fn add3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
pub(crate) fn sub3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
pub(crate) fn mul3(a: Vec3, b: Real) -> Vec3 {
    Vec3::new(a.x * b, a.y * b, a.z * b)
}
pub(crate) fn lerp3(a: Vec3, b: Vec3, t: Real) -> Vec3 {
    let u = 1.0 - t;
    Vec3::new(a.x * u + b.x * t, a.y * u + b.y * t, a.z * u + b.z * t)
}

pub(crate) fn dot3(a: Vec3, b: Vec3) -> Real {
    a.x * b.x + a.y * b.y + a.z * b.z
}
pub(crate) fn length3(v: Vec3) -> Real {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}
pub(crate) fn min_real(a: Real, b: Real) -> Real {
    if a < b { a } else { b }
}
pub(crate) fn max_real(a: Real, b: Real) -> Real {
    if a < b { b } else { a }
}
pub(crate) fn min3(v: Vec3) -> Real {
    min_real(min_real(v.x, v.y), v.z)
}
pub(crate) fn cross3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x)
}
pub(crate) fn normalize3(a: Vec3) -> Vec3 {
    let len = dot3(a, a).sqrt();
    if len > EPSILON { mul3(a, 1.0 / len) } else { Vec3::ZERO }
}
pub(crate) fn neg3(a: Vec3) -> Vec3 {
    Vec3::new(-a.x, -a.y, -a.z)
}
pub(crate) fn distsq2(a: Vec2, b: Vec2) -> Real {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    dx * dx + dy * dy
}
pub(crate) fn is_vec3_zero(v: Vec3) -> bool {
    v.x == 0.0 && v.y == 0.0 && v.z == 0.0
}
/// As ufbx's, it looks at x, y and z only
pub(crate) fn is_vec4_zero(v: Vec4) -> bool {
    v.x == 0.0 && v.y == 0.0 && v.z == 0.0
}
pub(crate) fn is_vec3_one(v: Vec3) -> bool {
    v.x == 1.0 && v.y == 1.0 && v.z == 1.0
}
pub(crate) fn is_quat_identity(v: Quat) -> bool {
    v.x == 0.0 && v.y == 0.0 && v.z == 0.0 && v.w == 1.0
}
pub(crate) fn is_transform_identity(t: &Transform) -> bool {
    is_vec3_zero(t.translation) && is_quat_identity(t.rotation) && is_vec3_one(t.scale)
}
pub(crate) fn matrix_all_zero(m: &Matrix) -> bool {
    m.v().iter().all(|&x| x == 0.0)
}

pub(crate) fn mul_quat(a: Quat, b: Quat) -> Quat {
    Quat {
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    }
}

pub(crate) fn add_translate(t: &mut Transform, v: Vec3) {
    t.translation.x += v.x;
    t.translation.y += v.y;
    t.translation.z += v.z;
}
pub(crate) fn sub_translate(t: &mut Transform, v: Vec3) {
    t.translation.x -= v.x;
    t.translation.y -= v.y;
    t.translation.z -= v.z;
}
pub(crate) fn mul_scale(t: &mut Transform, v: Vec3) {
    t.translation.x *= v.x;
    t.translation.y *= v.y;
    t.translation.z *= v.z;
    t.scale.x *= v.x;
    t.scale.y *= v.y;
    t.scale.z *= v.z;
}
pub(crate) fn mul_scale_real(t: &mut Transform, v: Real) {
    t.translation.x *= v;
    t.translation.y *= v;
    t.translation.z *= v;
    t.scale.x *= v;
    t.scale.y *= v;
    t.scale.z *= v;
}
pub(crate) fn add_weighted_vec3(r: &mut Vec3, b: Vec3, w: Real) {
    r.x += b.x * w;
    r.y += b.y * w;
    r.z += b.z * w;
}
pub(crate) fn add_weighted_quat(r: &mut Quat, b: Quat, w: Real) {
    r.x += b.x * w;
    r.y += b.y * w;
    r.z += b.z * w;
    r.w += b.w * w;
}
pub(crate) fn add_weighted_mat(r: &mut Matrix, b: &Matrix, w: Real) {
    let mut c = r.cols();
    let bc = b.cols();
    for i in 0..4 {
        add_weighted_vec3(&mut c[i], bc[i], w);
    }
    *r = Matrix::from_cols(c);
}

fn premul_rotation(t: &mut Transform, q: Quat) {
    if t.rotation.w != 1.0 {
        t.rotation = mul_quat(q, t.rotation);
    } else {
        t.rotation = q;
    }
    if !is_vec3_zero(t.translation) {
        t.translation = quat_rotate_vec3(q, t.translation);
    }
}

pub(crate) fn mul_rotate(t: &mut Transform, v: Vec3, order: RotationOrder) {
    if is_vec3_zero(v) {
        return;
    }
    premul_rotation(t, euler_to_quat(v, order));
}

pub(crate) fn mul_rotate_quat(t: &mut Transform, q: Quat) {
    if is_quat_identity(q) {
        return;
    }
    premul_rotation(t, q);
}

pub(crate) fn mul_inv_rotate(t: &mut Transform, v: Vec3, order: RotationOrder) {
    if is_vec3_zero(v) {
        return;
    }
    let mut q = euler_to_quat(v, order);
    q.x = -q.x;
    q.y = -q.y;
    q.z = -q.z;
    premul_rotation(t, q);
}

pub fn coordinate_axes_valid(axes: CoordinateAxes) -> bool {
    let valid = |a: CoordinateAxis| a != CoordinateAxis::Unknown;
    if !valid(axes.right) || !valid(axes.up) || !valid(axes.front) {
        return false;
    }
    let mut mask = 0u32;
    mask |= 1 << (axes.right as u32 >> 1);
    mask |= 1 << (axes.up as u32 >> 1);
    mask |= 1 << (axes.front as u32 >> 1);
    mask & 0x7 == 0x7
}

pub fn quat_mul(a: Quat, b: Quat) -> Quat {
    mul_quat(a, b)
}

pub fn vec3_normalize(v: Vec3) -> Vec3 {
    normalize3(v)
}

pub fn quat_dot(a: Quat, b: Quat) -> Real {
    a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w
}

pub fn quat_normalize(mut q: Quat) -> Quat {
    let norm = quat_dot(q, q);
    if norm == 0.0 {
        return Quat::IDENTITY;
    }
    let norm = norm.sqrt();
    q.x /= norm;
    q.y /= norm;
    q.z /= norm;
    q.w /= norm;
    q
}

pub fn quat_fix_antipodal(mut q: Quat, reference: Quat) -> Quat {
    if quat_dot(q, reference) < 0.0 {
        q.x = -q.x;
        q.y = -q.y;
        q.z = -q.z;
        q.w = -q.w;
    }
    q
}

pub fn quat_slerp(a: Quat, mut b: Quat, t: Real) -> Quat {
    let mut dot = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
    if dot < 0.0 {
        dot = -dot;
        b.x = -b.x;
        b.y = -b.y;
        b.z = -b.z;
        b.w = -b.w;
    }
    // fmax/fmin, which take a NaN to 0 as `clamp` would not
    let omega = dot.max(0.0).min(1.0).acos();
    if omega <= 1.175494351e-38f32 as f64 {
        return a;
    }
    let rcp_so = 1.0 / omega.sin();
    let af = ((1.0 - t) * omega).sin() * rcp_so;
    let bf = (t * omega).sin() * rcp_so;
    let x = af * a.x + bf * b.x;
    let y = af * a.y + bf * b.y;
    let z = af * a.z + bf * b.z;
    let w = af * a.w + bf * b.w;
    let rcp_len = 1.0 / (x * x + y * y + z * z + w * w).sqrt();
    Quat { x: x * rcp_len, y: y * rcp_len, z: z * rcp_len, w: w * rcp_len }
}

pub fn quat_rotate_vec3(q: Quat, v: Vec3) -> Vec3 {
    let xy = q.x * v.y - q.y * v.x;
    let xz = q.x * v.z - q.z * v.x;
    let yz = q.y * v.z - q.z * v.y;
    Vec3 { x: 2.0 * (q.w * yz + q.y * xy + q.z * xz) + v.x, y: 2.0 * (-q.x * xy - q.w * xz + q.z * yz) + v.y, z: 2.0 * (-q.x * xz - q.y * yz + q.w * xy) + v.z }
}

pub fn euler_to_quat(v: Vec3, order: RotationOrder) -> Quat {
    let vx = v.x * (DEG_TO_RAD_DOUBLE * 0.5);
    let vy = v.y * (DEG_TO_RAD_DOUBLE * 0.5);
    let vz = v.z * (DEG_TO_RAD_DOUBLE * 0.5);
    let (cx, sx) = (vx.cos(), vx.sin());
    let (cy, sy) = (vy.cos(), vy.sin());
    let (cz, sz) = (vz.cos(), vz.sin());
    use RotationOrder::*;
    match order {
        Xyz => Quat { x: -cx * sy * sz + cy * cz * sx, y: cx * cz * sy + cy * sx * sz, z: cx * cy * sz - cz * sx * sy, w: cx * cy * cz + sx * sy * sz },
        Xzy => Quat { x: cx * sy * sz + cy * cz * sx, y: cx * cz * sy + cy * sx * sz, z: cx * cy * sz - cz * sx * sy, w: cx * cy * cz - sx * sy * sz },
        Yzx => Quat { x: -cx * sy * sz + cy * cz * sx, y: cx * cz * sy - cy * sx * sz, z: cx * cy * sz + cz * sx * sy, w: cx * cy * cz + sx * sy * sz },
        Yxz => Quat { x: -cx * sy * sz + cy * cz * sx, y: cx * cz * sy + cy * sx * sz, z: cx * cy * sz + cz * sx * sy, w: cx * cy * cz - sx * sy * sz },
        Zxy => Quat { x: cx * sy * sz + cy * cz * sx, y: cx * cz * sy - cy * sx * sz, z: cx * cy * sz - cz * sx * sy, w: cx * cy * cz + sx * sy * sz },
        Zyx => Quat { x: cx * sy * sz + cy * cz * sx, y: cx * cz * sy - cy * sx * sz, z: cx * cy * sz + cz * sx * sy, w: cx * cy * cz - sx * sy * sz },
        Spheric => Quat::IDENTITY,
    }
}

pub fn quat_to_euler(q: Quat, order: RotationOrder) -> Vec3 {
    let eps = 0.999999999;
    let (qx, qy, qz, qw) = (q.x, q.y, q.z, q.w);
    let half_pi = |t: f64| (DPI * 0.5).copysign(t);
    use RotationOrder::*;
    let (vx, vy, vz);
    match order {
        Xyz => {
            let t = 2.0 * (qw * qy - qx * qz);
            if t.abs() < eps {
                vy = t.asin();
                vz = (2.0 * (qw * qz + qx * qy)).atan2(2.0 * (qw * qw + qx * qx) - 1.0);
                vx = -(-2.0 * (qw * qx + qy * qz)).atan2(2.0 * (qw * qw + qz * qz) - 1.0);
            } else {
                vy = half_pi(t);
                vz = (-2.0 * t * (qw * qx - qy * qz)).atan2(t * (2.0 * qw * qy + 2.0 * qx * qz));
                vx = 0.0;
            }
        }
        Xzy => {
            let t = 2.0 * (qw * qz + qx * qy);
            if t.abs() < eps {
                vz = t.asin();
                vy = (2.0 * (qw * qy - qx * qz)).atan2(2.0 * (qw * qw + qx * qx) - 1.0);
                vx = -(-2.0 * (qw * qx - qy * qz)).atan2(2.0 * (qw * qw + qy * qy) - 1.0);
            } else {
                vz = half_pi(t);
                vy = (2.0 * t * (qw * qx + qy * qz)).atan2(-t * (2.0 * qx * qy - 2.0 * qw * qz));
                vx = 0.0;
            }
        }
        Yzx => {
            let t = 2.0 * (qw * qz - qx * qy);
            if t.abs() < eps {
                vz = t.asin();
                vx = (2.0 * (qw * qx + qy * qz)).atan2(2.0 * (qw * qw + qy * qy) - 1.0);
                vy = -(-2.0 * (qw * qy + qx * qz)).atan2(2.0 * (qw * qw + qx * qx) - 1.0);
            } else {
                vz = half_pi(t);
                vx = (-2.0 * t * (qw * qy - qx * qz)).atan2(t * (2.0 * qw * qz + 2.0 * qx * qy));
                vy = 0.0;
            }
        }
        Yxz => {
            let t = 2.0 * (qw * qx + qy * qz);
            if t.abs() < eps {
                vx = t.asin();
                vz = (2.0 * (qw * qz - qx * qy)).atan2(2.0 * (qw * qw + qy * qy) - 1.0);
                vy = -(-2.0 * (qw * qy - qx * qz)).atan2(2.0 * (qw * qw + qz * qz) - 1.0);
            } else {
                vx = half_pi(t);
                vz = (2.0 * t * (qw * qy + qx * qz)).atan2(-t * (2.0 * qy * qz - 2.0 * qw * qx));
                vy = 0.0;
            }
        }
        Zxy => {
            let t = 2.0 * (qw * qx - qy * qz);
            if t.abs() < eps {
                vx = t.asin();
                vy = (2.0 * (qw * qy + qx * qz)).atan2(2.0 * (qw * qw + qz * qz) - 1.0);
                vz = -(-2.0 * (qw * qz + qx * qy)).atan2(2.0 * (qw * qw + qy * qy) - 1.0);
            } else {
                vx = half_pi(t);
                vy = (-2.0 * t * (qw * qz - qx * qy)).atan2(t * (2.0 * qw * qx + 2.0 * qy * qz));
                vz = 0.0;
            }
        }
        Zyx => {
            let t = 2.0 * (qw * qy + qx * qz);
            if t.abs() < eps {
                vy = t.asin();
                vx = (2.0 * (qw * qx - qy * qz)).atan2(2.0 * (qw * qw + qz * qz) - 1.0);
                vz = -(-2.0 * (qw * qz - qx * qy)).atan2(2.0 * (qw * qw + qx * qx) - 1.0);
            } else {
                vy = half_pi(t);
                vx = (2.0 * t * (qw * qz + qx * qy)).atan2(-t * (2.0 * qx * qz - 2.0 * qw * qy));
                vz = 0.0;
            }
        }
        Spheric => {
            vx = 0.0;
            vy = 0.0;
            vz = 0.0;
        }
    }
    Vec3 { x: vx * RAD_TO_DEG_DOUBLE, y: vy * RAD_TO_DEG_DOUBLE, z: vz * RAD_TO_DEG_DOUBLE }
}

pub fn matrix_mul(a: &Matrix, b: &Matrix) -> Matrix {
    Matrix {
        m03: a.m00 * b.m03 + a.m01 * b.m13 + a.m02 * b.m23 + a.m03,
        m13: a.m10 * b.m03 + a.m11 * b.m13 + a.m12 * b.m23 + a.m13,
        m23: a.m20 * b.m03 + a.m21 * b.m13 + a.m22 * b.m23 + a.m23,
        m00: a.m00 * b.m00 + a.m01 * b.m10 + a.m02 * b.m20,
        m10: a.m10 * b.m00 + a.m11 * b.m10 + a.m12 * b.m20,
        m20: a.m20 * b.m00 + a.m21 * b.m10 + a.m22 * b.m20,
        m01: a.m00 * b.m01 + a.m01 * b.m11 + a.m02 * b.m21,
        m11: a.m10 * b.m01 + a.m11 * b.m11 + a.m12 * b.m21,
        m21: a.m20 * b.m01 + a.m21 * b.m11 + a.m22 * b.m21,
        m02: a.m00 * b.m02 + a.m01 * b.m12 + a.m02 * b.m22,
        m12: a.m10 * b.m02 + a.m11 * b.m12 + a.m12 * b.m22,
        m22: a.m20 * b.m02 + a.m21 * b.m12 + a.m22 * b.m22,
    }
}

pub fn matrix_determinant(m: &Matrix) -> Real {
    -m.m02 * m.m11 * m.m20 + m.m01 * m.m12 * m.m20 + m.m02 * m.m10 * m.m21 - m.m00 * m.m12 * m.m21 - m.m01 * m.m10 * m.m22 + m.m00 * m.m11 * m.m22
}

pub fn matrix_invert(m: &Matrix) -> Matrix {
    let det = matrix_determinant(m);
    if det.abs() <= EPSILON {
        return Matrix::ZERO;
    }
    let rcp = 1.0 / det;
    Matrix {
        m00: (-m.m12 * m.m21 + m.m11 * m.m22) * rcp,
        m10: (m.m12 * m.m20 - m.m10 * m.m22) * rcp,
        m20: (-m.m11 * m.m20 + m.m10 * m.m21) * rcp,
        m01: (m.m02 * m.m21 - m.m01 * m.m22) * rcp,
        m11: (-m.m02 * m.m20 + m.m00 * m.m22) * rcp,
        m21: (m.m01 * m.m20 - m.m00 * m.m21) * rcp,
        m02: (-m.m02 * m.m11 + m.m01 * m.m12) * rcp,
        m12: (m.m02 * m.m10 - m.m00 * m.m12) * rcp,
        m22: (-m.m01 * m.m10 + m.m00 * m.m11) * rcp,
        m03: (m.m03 * m.m12 * m.m21 - m.m02 * m.m13 * m.m21 - m.m03 * m.m11 * m.m22 + m.m01 * m.m13 * m.m22 + m.m02 * m.m11 * m.m23 - m.m01 * m.m12 * m.m23)
            * rcp,
        m13: (m.m02 * m.m13 * m.m20 - m.m03 * m.m12 * m.m20 + m.m03 * m.m10 * m.m22 - m.m00 * m.m13 * m.m22 - m.m02 * m.m10 * m.m23 + m.m00 * m.m12 * m.m23)
            * rcp,
        m23: (m.m03 * m.m11 * m.m20 - m.m01 * m.m13 * m.m20 - m.m03 * m.m10 * m.m21 + m.m00 * m.m13 * m.m21 + m.m01 * m.m10 * m.m23 - m.m00 * m.m11 * m.m23)
            * rcp,
    }
}

pub fn matrix_for_normals(m: &Matrix) -> Matrix {
    let det = matrix_determinant(m);
    let s = if det >= 0.0 { 1.0 } else { -1.0 };
    Matrix {
        m00: (-m.m12 * m.m21 + m.m11 * m.m22) * s,
        m01: (m.m12 * m.m20 - m.m10 * m.m22) * s,
        m02: (-m.m11 * m.m20 + m.m10 * m.m21) * s,
        m10: (m.m02 * m.m21 - m.m01 * m.m22) * s,
        m11: (-m.m02 * m.m20 + m.m00 * m.m22) * s,
        m12: (m.m01 * m.m20 - m.m00 * m.m21) * s,
        m20: (-m.m02 * m.m11 + m.m01 * m.m12) * s,
        m21: (m.m02 * m.m10 - m.m00 * m.m12) * s,
        m22: (-m.m01 * m.m10 + m.m00 * m.m11) * s,
        m03: 0.0,
        m13: 0.0,
        m23: 0.0,
    }
}

pub fn transform_position(m: &Matrix, v: Vec3) -> Vec3 {
    Vec3 {
        x: m.m00 * v.x + m.m01 * v.y + m.m02 * v.z + m.m03,
        y: m.m10 * v.x + m.m11 * v.y + m.m12 * v.z + m.m13,
        z: m.m20 * v.x + m.m21 * v.y + m.m22 * v.z + m.m23,
    }
}

pub fn transform_direction(m: &Matrix, v: Vec3) -> Vec3 {
    Vec3 { x: m.m00 * v.x + m.m01 * v.y + m.m02 * v.z, y: m.m10 * v.x + m.m11 * v.y + m.m12 * v.z, z: m.m20 * v.x + m.m21 * v.y + m.m22 * v.z }
}

pub fn transform_to_matrix(t: &Transform) -> Matrix {
    let q = t.rotation;
    let (sx, sy, sz) = (2.0 * t.scale.x, 2.0 * t.scale.y, 2.0 * t.scale.z);
    let (xx, xy, xz, xw) = (q.x * q.x, q.x * q.y, q.x * q.z, q.x * q.w);
    let (yy, yz, yw) = (q.y * q.y, q.y * q.z, q.y * q.w);
    let (zz, zw) = (q.z * q.z, q.z * q.w);
    Matrix {
        m00: sx * (-yy - zz + 0.5),
        m10: sx * (xy + zw),
        m20: sx * (-yw + xz),
        m01: sy * (-zw + xy),
        m11: sy * (-xx - zz + 0.5),
        m21: sy * (xw + yz),
        m02: sz * (xz + yw),
        m12: sz * (-xw + yz),
        m22: sz * (-xx - yy + 0.5),
        m03: t.translation.x,
        m13: t.translation.y,
        m23: t.translation.z,
    }
}

pub(crate) fn unscaled_transform_to_matrix(t: &Transform) -> Matrix {
    transform_to_matrix(&Transform { scale: Vec3::ONE, ..*t })
}

pub fn matrix_to_transform(m: &Matrix) -> Transform {
    let det = matrix_determinant(m);
    let c = m.cols();
    let mut t = Transform { translation: c[3], rotation: Quat::IDENTITY, scale: Vec3::new(length3(c[0]), length3(c[1]), length3(c[2])) };
    let (mut sign_x, mut sign_y, mut sign_z) = (1.0, 1.0, 1.0);
    if det < 0.0 {
        if t.scale.x > 0.0 {
            sign_x = -1.0;
        } else if t.scale.y > 0.0 {
            sign_y = -1.0;
        } else if t.scale.z > 0.0 {
            sign_z = -1.0;
        }
    }
    let x = mul3(c[0], if t.scale.x > 0.0 { sign_x / t.scale.x } else { 0.0 });
    let y = mul3(c[1], if t.scale.y > 0.0 { sign_y / t.scale.y } else { 0.0 });
    let z = mul3(c[2], if t.scale.z > 0.0 { sign_z / t.scale.z } else { 0.0 });
    let trace = x.x + y.y + z.z;
    let r = &mut t.rotation;
    if trace > 0.0 {
        let a = (trace + 1.0).max(0.0).sqrt();
        let b = if a != 0.0 { 0.5 / a } else { 0.0 };
        *r = Quat { x: (y.z - z.y) * b, y: (z.x - x.z) * b, z: (x.y - y.x) * b, w: 0.5 * a };
    } else if x.x > y.y && x.x > z.z {
        let a = (1.0 + x.x - y.y - z.z).max(0.0).sqrt();
        let b = if a != 0.0 { 0.5 / a } else { 0.0 };
        *r = Quat { x: 0.5 * a, y: (y.x + x.y) * b, z: (z.x + x.z) * b, w: (y.z - z.y) * b };
    } else if y.y > z.z {
        let a = (1.0 - x.x + y.y - z.z).max(0.0).sqrt();
        let b = if a != 0.0 { 0.5 / a } else { 0.0 };
        *r = Quat { x: (y.x + x.y) * b, y: 0.5 * a, z: (z.y + y.z) * b, w: (z.x - x.z) * b };
    } else {
        let a = (1.0 - x.x - y.y + z.z).max(0.0).sqrt();
        let b = if a != 0.0 { 0.5 / a } else { 0.0 };
        *r = Quat { x: (z.x + x.z) * b, y: (z.y + y.z) * b, z: 0.5 * a, w: (x.y - y.x) * b };
    }
    let len = r.x * r.x + r.y * r.y + r.z * r.z + r.w * r.w;
    if (len - 1.0).abs() > EPSILON {
        if len.abs() <= EPSILON {
            *r = Quat::IDENTITY;
        } else {
            r.x /= len;
            r.y /= len;
            r.z /= len;
            r.w /= len;
        }
    }
    t.scale.x *= sign_x;
    t.scale.y *= sign_y;
    t.scale.z *= sign_z;
    t
}

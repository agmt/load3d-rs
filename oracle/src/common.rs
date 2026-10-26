use load3d as rs;

macro_rules! both {
    ($o:ident, $f:ident = $t:ident :: $v:ident) => {{
        $o.0.$f = ufbx::$t::$v;
        $o.1.$f = load3d::$t::$v;
    }};
    ($o:ident, $f:ident = $v:expr) => {{
        $o.0.$f = $v;
        $o.1.$f = $v;
    }};
}
pub(crate) use both;

/// Equal bits, or both NaN: a NaN's sign and payload follow operand order, which IEEE leaves open
pub fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

pub fn same2(a: &ufbx::Vec2, b: &rs::Vec2) -> bool {
    same(a.x, b.x) && same(a.y, b.y)
}

pub fn same3(a: &ufbx::Vec3, b: &rs::Vec3) -> bool {
    same(a.x, b.x) && same(a.y, b.y) && same(a.z, b.z)
}

pub fn same_quat(a: &ufbx::Quat, b: &rs::Quat) -> bool {
    same(a.x, b.x) && same(a.y, b.y) && same(a.z, b.z) && same(a.w, b.w)
}

pub fn same_mat(a: &ufbx::Matrix, b: &rs::Matrix) -> bool {
    matrix_v(a).iter().zip(b.v()).all(|(x, y)| same(*x, y))
}

pub fn same_transform(a: &ufbx::Transform, b: &rs::Transform) -> bool {
    same3(&a.translation, &b.translation) && same_quat(&a.rotation, &b.rotation) && same3(&a.scale, &b.scale)
}

pub fn canon(x: f64) -> u64 {
    if x.is_nan() { 0x7ff8_0000_0000_0000 } else { x.to_bits() }
}

pub fn v2(v: &ufbx::Vec2) -> rs::Vec2 {
    rs::Vec2::new(v.x, v.y)
}

pub fn v3(v: &ufbx::Vec3) -> rs::Vec3 {
    rs::Vec3::new(v.x, v.y, v.z)
}

pub fn v4(v: &ufbx::Vec4) -> rs::Vec4 {
    rs::Vec4::new(v.x, v.y, v.z, v.w)
}

pub fn quat(q: &ufbx::Quat) -> rs::Quat {
    rs::Quat::new(q.x, q.y, q.z, q.w)
}

fn matrix_v(m: &ufbx::Matrix) -> [f64; 12] {
    [m.m00, m.m10, m.m20, m.m01, m.m11, m.m21, m.m02, m.m12, m.m22, m.m03, m.m13, m.m23]
}

pub fn matrix(m: &ufbx::Matrix) -> rs::Matrix {
    rs::Matrix::from_v(matrix_v(m))
}

pub fn transform(t: &ufbx::Transform) -> rs::Transform {
    rs::Transform { translation: v3(&t.translation), rotation: quat(&t.rotation), scale: v3(&t.scale) }
}

pub fn element_type(t: ufbx::ElementType) -> rs::ElementType {
    rs::ElementType::from_index(t as usize)
}

/// xorshift64, seeded from a name so that every input draws the same numbers in every run
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn named(parts: &[&str]) -> Rng {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for p in parts {
            for &b in p.as_bytes().iter().chain(b"/") {
                h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
            }
        }
        Rng::new(h ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }

    pub fn real(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

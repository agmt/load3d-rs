use std::cmp::Ordering;
use std::io::{Seek, SeekFrom};
use std::sync::OnceLock;

use load3d::*;

use super::check_scene::{check_mesh, check_scene};
use super::runner::data_path;

#[derive(Debug, Clone, Copy, Default)]
pub struct DiffError {
    pub num: usize,
    pub sum: f64,
    pub max: f64,
}

impl DiffError {
    fn add(&mut self, err: f64) {
        self.num += 1;
        self.sum += err;
        if err > self.max {
            self.max = err;
        }
    }
}

pub fn assert_close_real(p_err: &mut DiffError, a: Real, b: Real) {
    let err = (a - b).abs();
    ufbxt_soft_assert!(err < 0.001);
    p_err.add(err);
}

pub fn assert_close_double(p_err: &mut DiffError, a: f64, b: f64) {
    let err = (a - b).abs();
    ufbxt_soft_assert!(err < 0.001);
    p_err.add(err);
}

pub fn assert_close_vec2(p_err: &mut DiffError, a: Vec2, b: Vec2) {
    assert_close_real(p_err, a.x, b.x);
    assert_close_real(p_err, a.y, b.y);
}

pub fn assert_close_vec3(p_err: &mut DiffError, a: Vec3, b: Vec3) {
    assert_close_real(p_err, a.x, b.x);
    assert_close_real(p_err, a.y, b.y);
    assert_close_real(p_err, a.z, b.z);
}

pub fn assert_close_vec3_xyz(p_err: &mut DiffError, a: Vec3, x: Real, y: Real, z: Real) {
    assert_close_real(p_err, a.x, x);
    assert_close_real(p_err, a.y, y);
    assert_close_real(p_err, a.z, z);
}

pub fn assert_close_vec4(p_err: &mut DiffError, a: Vec4, b: Vec4) {
    assert_close_real(p_err, a.x, b.x);
    assert_close_real(p_err, a.y, b.y);
    assert_close_real(p_err, a.z, b.z);
    assert_close_real(p_err, a.w, b.w);
}

pub fn assert_close_quat(p_err: &mut DiffError, a: Quat, b: Quat) {
    assert_close_real(p_err, a.x, b.x);
    assert_close_real(p_err, a.y, b.y);
    assert_close_real(p_err, a.z, b.z);
    assert_close_real(p_err, a.w, b.w);
}

pub fn assert_close_matrix(p_err: &mut DiffError, a: Matrix, b: Matrix) {
    assert_close_vec3(p_err, Vec3::new(a.m00, a.m10, a.m20), Vec3::new(b.m00, b.m10, b.m20));
    assert_close_vec3(p_err, Vec3::new(a.m01, a.m11, a.m21), Vec3::new(b.m01, b.m11, b.m21));
    assert_close_vec3(p_err, Vec3::new(a.m02, a.m12, a.m22), Vec3::new(b.m02, b.m12, b.m22));
    assert_close_vec3(p_err, Vec3::new(a.m03, a.m13, a.m23), Vec3::new(b.m03, b.m13, b.m23));
}

pub fn assert_close_real_threshold(p_err: &mut DiffError, a: Real, b: Real, threshold: Real) {
    let err = (a - b).abs();
    ufbxt_assert!(err < threshold);
    p_err.add(err);
}

pub fn assert_close_vec2_threshold(p_err: &mut DiffError, a: Vec2, b: Vec2, threshold: Real) {
    assert_close_real_threshold(p_err, a.x, b.x, threshold);
    assert_close_real_threshold(p_err, a.y, b.y, threshold);
}

pub fn assert_close_vec3_threshold(p_err: &mut DiffError, a: Vec3, b: Vec3, threshold: Real) {
    assert_close_real_threshold(p_err, a.x, b.x, threshold);
    assert_close_real_threshold(p_err, a.y, b.y, threshold);
    assert_close_real_threshold(p_err, a.z, b.z, threshold);
}

pub fn assert_close_vec4_threshold(p_err: &mut DiffError, a: Vec4, b: Vec4, threshold: Real) {
    assert_close_real_threshold(p_err, a.x, b.x, threshold);
    assert_close_real_threshold(p_err, a.y, b.y, threshold);
    assert_close_real_threshold(p_err, a.z, b.z, threshold);
    assert_close_real_threshold(p_err, a.w, b.w, threshold);
}

pub fn assert_close_quat_threshold(p_err: &mut DiffError, a: Quat, b: Quat, threshold: Real) {
    assert_close_real_threshold(p_err, a.x, b.x, threshold);
    assert_close_real_threshold(p_err, a.y, b.y, threshold);
    assert_close_real_threshold(p_err, a.z, b.z, threshold);
    assert_close_real_threshold(p_err, a.w, b.w, threshold);
}

pub fn read_file(name: &str) -> Option<Vec<u8>> {
    std::fs::read(name).ok()
}

pub fn dot2(a: Vec2, b: Vec2) -> Real {
    a.x * b.x + a.y * b.y
}

pub fn dot3(a: Vec3, b: Vec3) -> Real {
    a.x * b.x + a.y * b.y + a.z * b.z
}

pub fn length2(a: Vec2) -> Real {
    dot2(a, a).sqrt()
}

pub fn length3(a: Vec3) -> Real {
    dot3(a, a).sqrt()
}

pub fn add2(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x + b.x, a.y + b.y)
}

pub fn add3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

pub fn sub2(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

pub fn sub3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

pub fn mul2(a: Vec2, b: Real) -> Vec2 {
    Vec2::new(a.x * b, a.y * b)
}

pub fn mul3(a: Vec3, b: Real) -> Vec3 {
    Vec3::new(a.x * b, a.y * b, a.z * b)
}

pub fn cross3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x)
}

pub fn eq3(a: Vec3, b: Vec3) -> bool {
    a.x == b.x && a.y == b.y && a.z == b.z
}

pub fn normalize(a: Vec3) -> Vec3 {
    let len = dot3(a, a).sqrt();
    if len != 0.0 { mul3(a, 1.0 / len) } else { Vec3::ZERO }
}

pub fn min(a: Real, b: Real) -> Real {
    if a < b { a } else { b }
}

pub fn max(a: Real, b: Real) -> Real {
    if a < b { b } else { a }
}

pub fn clamp(v: Real, min_v: Real, max_v: Real) -> Real {
    min(max(v, min_v), max_v)
}

/// Indices count from the file's first value; each attribute's values are those read before the mesh's last index.
#[derive(Debug, Clone, Default)]
pub struct ObjMesh {
    pub groups: Vec<String>,
    pub original_group: Option<String>,
    pub num_faces: usize,
    pub num_indices: usize,
    pub faces: Vec<Face>,
    pub vertex_position: VertexVec3,
    pub vertex_normal: VertexVec3,
    pub vertex_uv: VertexVec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjExporter {
    #[default]
    Unknown,
    Blender,
}

#[derive(Debug, Clone)]
pub struct ObjFile {
    pub meshes: Vec<ObjMesh>,
    pub bad_normals: bool,
    pub bad_order: bool,
    pub bad_uvs: bool,
    pub bad_faces: bool,
    pub has_empty: bool,
    pub missing_uvs: bool,
    pub line_faces: bool,
    pub point_faces: bool,
    pub no_subdivision: bool,
    pub root_groups_at_bone: bool,
    pub remove_namespaces: bool,
    pub match_by_order: bool,
    pub negate_xz: bool,
    pub bind_pose: bool,
    pub ignore_unskinned: bool,
    pub transform_skin: bool,
    pub no_bake: bool,
    pub tolerance: Real,
    pub uv_tolerance: Real,
    pub allow_missing: u32,
    pub animation_frame: i32,
    pub normalize_units: bool,
    pub ignore_duplicates: bool,
    pub exporter: ObjExporter,
    pub position_scale: f64,
    pub position_scale_float: f64,
    pub position_scale_bake: f64,
    pub position_scale_compensate: f64,
    pub fbx_position_scale: f64,
    pub fbx_rotation: Quat,
    pub animation_name: String,
    pub model_name: String,
}

/// `load_obj` does not read it, as in ufbx
#[derive(Debug, Clone, Default)]
pub struct LoadObjOpts {
    pub name_end_chars: Option<String>,
}

fn cmp_obj_mesh(a: &ObjMesh, b: &ObjMesh) -> Ordering {
    a.groups.len().cmp(&b.groups.len())
}

fn read_u16(p: &[u8]) -> u16 {
    u16::from_le_bytes([p[0], p[1]])
}

fn read_u32(p: &[u8]) -> u32 {
    u32::from_le_bytes([p[0], p[1], p[2], p[3]])
}

fn decompress_gzip(gz: &[u8]) -> Option<Vec<u8>> {
    let gz_size = gz.len();
    if gz_size < 10 {
        return None;
    }
    if gz[0] != 0x1f || gz[1] != 0x8b || gz[2] != 0x08 {
        return None;
    }
    let flags = gz[3];

    let mut offset = 10;

    if flags & 0x4 != 0 {
        // FEXTRA
        if gz_size - offset < 2 {
            return None;
        }
        let xlen = read_u16(&gz[offset..]) as usize;
        offset += 2;
        if gz_size - offset < xlen {
            return None;
        }
        offset += xlen;
    }

    if flags & 0x8 != 0 {
        // FNAME
        let end = gz[offset..].iter().position(|&c| c == 0)?;
        offset += end + 1;
    }

    if flags & 0x10 != 0 {
        // FCOMMENT
        let end = gz[offset..].iter().position(|&c| c == 0)?;
        offset += end + 1;
    }

    if flags & 0x2 != 0 {
        // FHCRC
        if gz_size - offset < 2 {
            return None;
        }
        offset += 2;
    }

    let isize = read_u32(&gz[gz_size - 4..]) as usize;

    let mut dst = vec![0u8; isize];
    let mut retain = InflateRetain::new();
    let mut input = InflateInput { data: &gz[offset..], total_size: gz_size - offset, no_header: true, no_checksum: true, ..Default::default() };

    let result = inflate(&mut dst, &mut input, &mut retain);
    if result != isize as isize {
        return None;
    }
    Some(dst)
}

const MAX_EXPONENT: i32 = 325;

fn init_parsing() -> &'static [f64] {
    static POW10_TABLE: OnceLock<Vec<f64>> = OnceLock::new();
    POW10_TABLE.get_or_init(|| (0..=MAX_EXPONENT * 2).map(|i| 10f64.powf((i - MAX_EXPONENT) as f64)).collect())
}

fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

fn parse_ints(p_result: &mut [i32], max_count: usize, s: &mut &[u8]) -> usize {
    let st = *s;
    let mut c = 0;

    let mut num = 0;
    while num < max_count {
        if at(st, c) == b'/' {
            c += 1;
            p_result[num] = 0;
            num += 1;
            continue;
        }

        let mut sign = 1;
        if at(st, c) == b'+' {
            c += 1;
        } else if at(st, c) == b'-' {
            sign = -1;
            c += 1;
        }

        if !at(st, c).is_ascii_digit() {
            return num;
        }

        const MAX_VALUE: u32 = 100000000;
        let mut value: u32 = 0;

        loop {
            if value < MAX_VALUE {
                value = value * 10 + (at(st, c) - b'0') as u32;
                c += 1;
            } else {
                return num;
            }
            if !at(st, c).is_ascii_digit() {
                break;
            }
        }

        p_result[num] = value as i32 * sign;

        if at(st, c) != b'/' {
            return num;
        }
        c += 1;
        num += 1;
    }

    *s = &st[c..];
    num
}

fn parse_reals(p_result: &mut [Real], count: usize, s: &[u8]) -> bool {
    let pow10_table = init_parsing();
    let mut c = 0;

    for i in 0..count {
        while at(s, c) == b' ' {
            c += 1;
        }

        const MAX_VALUE: u64 = 1000000000000000000;
        let mut value: u64 = 0;

        let mut sign = 1.0f64;
        if at(s, c) == b'-' {
            sign = -1.0;
            c += 1;
        } else if at(s, c) == b'+' {
            c += 1;
        }

        let mut exponent: i32 = 0;

        if !at(s, c).is_ascii_digit() {
            if s[c..].starts_with(b"nan(ind)") {
                p_result[0] = Real::NAN;
                c += 8;
                continue;
            }

            return false;
        }

        loop {
            if value < MAX_VALUE {
                value = value * 10 + (at(s, c) - b'0') as u64;
            } else {
                exponent -= 1;
            }
            c += 1;
            if !at(s, c).is_ascii_digit() {
                break;
            }
        }

        if at(s, c) == b'.' {
            c += 1;

            while at(s, c).is_ascii_digit() {
                if value < MAX_VALUE {
                    value = value * 10 + (at(s, c) - b'0') as u64;
                    exponent -= 1;
                }
                c += 1;
            }

            if (at(s, c) | 0x20) == b'e' {
                c += 1;
                let mut expsign = 1;
                if at(s, c) == b'+' {
                    c += 1;
                } else if at(s, c) == b'-' {
                    expsign = -1;
                    c += 1;
                }

                let mut exp: i32 = 0;
                while at(s, c).is_ascii_digit() {
                    if exp < 4096 {
                        exp = exp * 10 + (at(s, c) - b'0') as i32;
                    }
                    c += 1;
                }

                exponent += expsign * exp;
            }
        }

        if (-MAX_EXPONENT..=MAX_EXPONENT).contains(&exponent) {
            p_result[i] = (sign * pow10_table[(exponent + MAX_EXPONENT) as usize] * value as f64) as Real;
        } else if exponent < 0 {
            p_result[i] = 0.0;
        } else {
            p_result[i] = Real::INFINITY;
        }
    }

    true
}

fn find_newline(src: &[u8]) -> Option<usize> {
    src.iter().position(|&c| c == b'\r' || c == b'\n')
}

fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\r' || c == b'\n'
}

struct ObjLines<'a> {
    data: &'a [u8],
    pos: Option<usize>,
}

impl<'a> Iterator for ObjLines<'a> {
    type Item = (&'a [u8], bool);

    fn next(&mut self) -> Option<Self::Item> {
        let rest = &self.data[self.pos?..];
        match find_newline(rest) {
            Some(end) => {
                self.pos = self.pos.map(|p| p + end + 1);
                Some((&rest[..end], true))
            }
            None => {
                self.pos = None;
                Some((rest, false))
            }
        }
    }
}

fn obj_lines(data: &[u8]) -> ObjLines<'_> {
    ObjLines { data, pos: Some(0) }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn atoi(s: &[u8]) -> i32 {
    let mut c = 0;
    while c < s.len() && isspace(s[c]) {
        c += 1;
    }
    let mut sign = 1i32;
    if at(s, c) == b'+' || at(s, c) == b'-' {
        if s[c] == b'-' {
            sign = -1;
        }
        c += 1;
    }
    let mut value = 0i32;
    while at(s, c).is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((s[c] - b'0') as i32);
        c += 1;
    }
    value.wrapping_mul(sign)
}

fn scan_f64(line: &[u8], prefix: &[u8]) -> Option<f64> {
    let s = line.strip_prefix(prefix)?;
    let s = &s[s.iter().take_while(|&&c| isspace(c)).count()..];
    let digits = |from: usize| s[from..].iter().take_while(|c| c.is_ascii_digit()).count();

    let mut n = 0;
    if at(s, n) == b'+' || at(s, n) == b'-' {
        n += 1;
    }
    for word in ["infinity", "inf", "nan"] {
        if s.len() >= n + word.len() && s[n..n + word.len()].eq_ignore_ascii_case(word.as_bytes()) {
            return std::str::from_utf8(&s[..n + word.len()]).ok()?.parse().ok();
        }
    }
    let int_digits = digits(n);
    n += int_digits;
    let mut frac_digits = 0;
    if at(s, n) == b'.' {
        frac_digits = digits(n + 1);
        if int_digits + frac_digits > 0 {
            n += 1 + frac_digits;
        }
    }
    if int_digits + frac_digits == 0 {
        return None;
    }
    if at(s, n) == b'e' || at(s, n) == b'E' {
        let mut m = n + 1;
        if at(s, m) == b'+' || at(s, m) == b'-' {
            m += 1;
        }
        let exp_digits = digits(m);
        if exp_digits > 0 {
            n = m + exp_digits;
        }
    }
    std::str::from_utf8(&s[..n]).ok()?.parse().ok()
}

fn scan_int(line: &[u8], prefix: &[u8]) -> Option<i32> {
    let s = line.strip_prefix(prefix)?;
    let s = &s[s.iter().take_while(|&&c| isspace(c)).count()..];
    let sign = usize::from(at(s, 0) == b'+' || at(s, 0) == b'-');
    let digits = s[sign..].iter().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    std::str::from_utf8(&s[..sign + digits]).ok()?.parse::<i64>().ok().map(|v| v as i32)
}

fn c_string(buf: &[u8]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..len]).into_owned()
}

pub fn load_obj(obj_data: &[u8], _opts: Option<&LoadObjOpts>) -> ObjFile {
    let data = &obj_data[..obj_data.iter().position(|&c| c == 0).unwrap_or(obj_data.len())];

    let mut num_meshes = 0;

    let mut merge_groups = false;
    let mut allow_default = false;

    for (line, _) in obj_lines(data) {
        if line.starts_with(b"v ") || line.starts_with(b"vt ") || line.starts_with(b"vn ") || line.starts_with(b"f ") {
            continue;
        }
        if line.starts_with(b"g defau") && !allow_default {
            continue;
        }
        if (line.starts_with(b"g ") && !merge_groups) || line.starts_with(b"o ") {
            num_meshes += 1;
        } else if contains(line, b"ufbx:merge_groups") {
            merge_groups = true;
        } else if contains(line, b"ufbx:allow_default") {
            allow_default = true;
        }
    }

    let implicit_mesh = num_meshes == 0;

    let mut obj = ObjFile {
        meshes: Vec::new(),
        bad_normals: false,
        bad_order: false,
        bad_uvs: false,
        bad_faces: false,
        has_empty: false,
        missing_uvs: false,
        line_faces: false,
        point_faces: false,
        no_subdivision: false,
        root_groups_at_bone: false,
        remove_namespaces: false,
        match_by_order: false,
        negate_xz: false,
        bind_pose: false,
        ignore_unskinned: false,
        transform_skin: false,
        no_bake: false,
        tolerance: 0.001,
        uv_tolerance: 0.001,
        allow_missing: 0,
        animation_frame: -1,
        normalize_units: false,
        ignore_duplicates: false,
        exporter: ObjExporter::Unknown,
        position_scale: 1.0,
        position_scale_float: 1.0,
        position_scale_bake: 1.0,
        position_scale_compensate: 1.0,
        fbx_position_scale: 1.0,
        fbx_rotation: Quat::IDENTITY,
        animation_name: String::new(),
        model_name: String::new(),
    };
    let mut animation_name = [0u8; 128];
    let mut model_name = [0u8; 128];

    let mut positions: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<Vec2> = Vec::new();
    // per mesh, how many positions, normals and UVs were read at its last index
    let mut value_counts: Vec<[usize; 3]> = Vec::new();
    let mut mesh: Option<usize> = None;

    if implicit_mesh {
        obj.meshes.push(ObjMesh::default());
        value_counts.push([0; 3]);
        mesh = Some(0);
    }

    for (full_line, has_end) in obj_lines(data) {
        let mut line = full_line;

        if line.starts_with(b"v ") {
            line = &line[2..];
            let mut v = [0.0; 3];
            ufbxt_assert!(parse_reals(&mut v, 3, line));
            positions.push(Vec3::new(v[0], v[1], v[2]));
        } else if line.starts_with(b"vt ") {
            line = &line[3..];
            let mut v = [0.0; 2];
            ufbxt_assert!(parse_reals(&mut v, 2, line));
            uvs.push(Vec2::new(v[0], v[1]));
        } else if line.starts_with(b"vn ") {
            line = &line[3..];
            let mut v = [0.0; 3];
            ufbxt_assert!(parse_reals(&mut v, 3, line));
            normals.push(Vec3::new(v[0], v[1], v[2]));
        } else if line.starts_with(b"f ") {
            ufbxt_assert!(mesh.is_some());
            let mi = mesh.unwrap();
            let m = &mut obj.meshes[mi];

            let mut face = Face { index_begin: m.num_indices as u32, num_indices: 0 };

            let mut begin = Some(&line[2..]);
            while let Some(b) = begin {
                let (mut token, end) = match b.iter().position(|&c| c == b' ') {
                    Some(p) => (&b[..p], Some(&b[p + 1..])),
                    None => (b, None),
                };

                while token.first().is_some_and(|&c| is_space(c)) {
                    token = &token[1..];
                }
                if token.is_empty() {
                    begin = end;
                    continue;
                }

                let mut indices = [0i32; 3];
                parse_ints(&mut indices, 3, &mut token);

                value_counts[mi] = [positions.len(), normals.len(), uvs.len()];
                if indices[0] > 0 {
                    m.vertex_position.exists = true;
                }
                if indices[1] > 0 {
                    m.vertex_uv.exists = true;
                }
                if indices[2] > 0 {
                    m.vertex_normal.exists = true;
                }

                m.vertex_position.indices.push(indices[0].wrapping_sub(1) as u32);
                m.vertex_normal.indices.push(indices[2].wrapping_sub(1) as u32);
                m.vertex_uv.indices.push(indices[1].wrapping_sub(1) as u32);
                m.num_indices += 1;
                face.num_indices += 1;

                begin = end;
            }

            m.faces.push(face);
            m.num_faces += 1;
        } else if line.starts_with(b"g defau") && !allow_default {
        } else if (line.starts_with(b"g ") && !merge_groups) || line.starts_with(b"o ") {
            obj.meshes.push(ObjMesh::default());
            value_counts.push([0; 3]);
            let mi = obj.meshes.len() - 1;
            mesh = Some(mi);
            let m = &mut obj.meshes[mi];

            let rest = &line[2..];
            m.original_group = Some(String::from_utf8_lossy(rest).into_owned());

            let mut c = 0;
            loop {
                while matches!(at(rest, c), b' ' | b'\t' | b'\r') {
                    c += 1;
                }

                if matches!(at(rest, c), b'\n' | 0) {
                    break;
                }

                let mut group = Vec::new();
                while !matches!(at(rest, c), b' ' | b'\t' | b'\r' | b'\n' | 0) {
                    if rest[c..].starts_with(b"FBXASC") {
                        c += 6;
                        let mut num_len = 0;
                        while num_len < 3 && at(rest, c) != 0 {
                            c += 1;
                            num_len += 1;
                        }
                        group.push(atoi(&rest[c - num_len..c]) as u8);
                    } else {
                        group.push(rest[c]);
                        c += 1;
                    }
                }
                m.groups.push(c_string(&group));
            }
        }

        if line.first() == Some(&b'#') {
            line = &line[1..];
            // C trims against the newline that ends the line, so not on a last line without one
            if has_end {
                while line.first().is_some_and(|&c| c == b' ' || c == b'\t') {
                    line = &line[1..];
                }
                while line.last().is_some_and(|&c| c == b' ' || c == b'\t') {
                    line = &line[..line.len() - 1];
                }
            }
            let flags: [(&[u8], &mut bool); 18] = [
                (b"ufbx:bad_normals", &mut obj.bad_normals),
                (b"ufbx:bad_order", &mut obj.bad_order),
                (b"ufbx:bad_uvs", &mut obj.bad_uvs),
                (b"ufbx:bad_faces", &mut obj.bad_faces),
                (b"ufbx:has_empty", &mut obj.has_empty),
                (b"ufbx:missing_uvs", &mut obj.missing_uvs),
                (b"ufbx:line_faces", &mut obj.line_faces),
                (b"ufbx:point_faces", &mut obj.point_faces),
                (b"ufbx:no_subdivision", &mut obj.no_subdivision),
                (b"ufbx:ignore_duplicates", &mut obj.ignore_duplicates),
                (b"ufbx:root_groups_at_bone", &mut obj.root_groups_at_bone),
                (b"ufbx:remove_namespaces", &mut obj.remove_namespaces),
                (b"ufbx:match_by_order", &mut obj.match_by_order),
                (b"ufbx:negate_xz", &mut obj.negate_xz),
                (b"ufbx:bind_pose", &mut obj.bind_pose),
                (b"ufbx:ignore_unskinned", &mut obj.ignore_unskinned),
                (b"ufbx:transform_skin", &mut obj.transform_skin),
                (b"ufbx:no_bake", &mut obj.no_bake),
            ];
            for (name, flag) in flags {
                if line == name {
                    *flag = true;
                }
            }
            if line == b"www.blender.org" {
                obj.exporter = ObjExporter::Blender;
            }
            if line.starts_with(b"ufbx:animation=") {
                line = &line[15..];
                let len = line.iter().position(|&c| c == b'\r' || c == b'\n').unwrap_or(line.len());
                ufbxt_assert!(len + 1 < animation_name.len());
                animation_name[..len].copy_from_slice(&line[..len]);
            }
            if line.starts_with(b"ufbx:model=") {
                line = &line[11..];
                let len = line.iter().position(|&c| c == b'\r' || c == b'\n').unwrap_or(line.len());
                ufbxt_assert!(len + 1 < model_name.len());
                model_name[..len].copy_from_slice(&line[..len]);
            }
            if let Some(tolerance) = scan_f64(line, b"ufbx:tolerance=") {
                obj.tolerance = tolerance as Real;
            }
            if let Some(uv_tolerance) = scan_f64(line, b"ufbx:uv_tolerance=") {
                obj.uv_tolerance = uv_tolerance as Real;
            }
            if let Some(allow_missing) = scan_int(line, b"ufbx:allow_missing=") {
                obj.allow_missing = allow_missing as u32;
            }
            if let Some(position_scale) = scan_f64(line, b"ufbx:position_scale=") {
                obj.position_scale = position_scale;
            }
            if let Some(position_scale_float) = scan_f64(line, b"ufbx:position_scale_float=") {
                obj.position_scale_float = position_scale_float;
            }
            if let Some(position_scale_bake) = scan_f64(line, b"ufbx:position_scale_bake=") {
                obj.position_scale_bake = position_scale_bake;
            }
            if let Some(position_scale_compensate) = scan_f64(line, b"ufbx:position_scale_compensate=") {
                obj.position_scale_compensate = position_scale_compensate;
            }
            if let Some(frame) = scan_int(line, b"ufbx:frame=") {
                obj.animation_frame = frame;
            }
        }
    }

    for (m, counts) in obj.meshes.iter_mut().zip(&value_counts) {
        m.vertex_position.values = positions[..counts[0]].to_vec();
        m.vertex_normal.values = normals[..counts[1]].to_vec();
        m.vertex_uv.values = uvs[..counts[2]].to_vec();
    }

    obj.meshes.sort_by(cmp_obj_mesh);
    obj.animation_name = c_string(&animation_name);
    obj.model_name = c_string(&model_name);

    obj
}

pub fn check_source_vertices(mesh: &Mesh, src_mesh: &Mesh, p_err: &mut DiffError, mut threshold: Real) {
    if threshold == 0.0 {
        threshold = 0.001;
    }

    let sub = mesh.subdivision_result.as_deref();
    ufbxt_assert!(sub.is_some());
    let sub = sub.unwrap();

    let num_vertices = mesh.num_vertices;
    ufbxt_assert!(sub.source_vertex_ranges.len() == num_vertices);
    for vi in 0..num_vertices {
        let range = sub.source_vertex_ranges[vi];

        let mut sum = Vec3::ZERO;
        for i in 0..range.num_weights as usize {
            let weight = sub.source_vertex_weights[range.weight_begin as usize + i];
            let v = src_mesh.vertices[weight.index as usize];
            sum.x += v.x * weight.weight;
            sum.y += v.y * weight.weight;
            sum.z += v.z * weight.weight;
        }

        let r = mesh.vertices[vi];
        assert_close_vec3_threshold(p_err, r, sum, threshold);
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct MatchVertex {
    pos: Vec3,
    normal: Vec3,
    uv: Vec2,
}

fn cmp_sub_vertex(a: &MatchVertex, b: &MatchVertex) -> Ordering {
    let pairs = [
        (a.pos.x, b.pos.x),
        (a.pos.y, b.pos.y),
        (a.pos.z, b.pos.z),
        (a.normal.x, b.normal.x),
        (a.normal.y, b.normal.y),
        (a.normal.z, b.normal.z),
        (a.uv.x, b.uv.x),
        (a.uv.y, b.uv.y),
    ];
    for (x, y) in pairs {
        if x != y {
            return if x < y { Ordering::Less } else { Ordering::Greater };
        }
    }
    Ordering::Equal
}

fn face_has_duplicate_vertex(mesh: &Mesh, face: Face) -> bool {
    for i in 0..face.num_indices as usize {
        let ix = mesh.vertex_indices[face.index_begin as usize + i];
        for j in 0..i {
            let jx = mesh.vertex_indices[face.index_begin as usize + j];
            if ix == jx {
                return true;
            }
        }
    }
    false
}

fn match_obj_mesh(obj: &ObjFile, fbx_node: &Node, fbx_mesh: &Mesh, obj_mesh: &ObjMesh, p_err: &mut DiffError, scale: Real) {
    let tolerance = obj.tolerance;

    if !obj.bad_faces {
        ufbxt_assert!(fbx_mesh.num_faces == obj_mesh.num_faces);
        ufbxt_assert!(fbx_mesh.num_indices == obj_mesh.num_indices);
    }

    let mut obj_verts = vec![MatchVertex::default(); obj_mesh.num_indices];
    let mut fbx_verts = vec![MatchVertex::default(); fbx_mesh.num_indices];

    let norm_mat = get_compatible_matrix_for_normals(fbx_node);

    for (i, ov) in obj_verts.iter_mut().enumerate() {
        ov.pos = obj_mesh.vertex_position.get(i);
        ov.normal = obj_mesh.vertex_normal.get(i);
        if obj_mesh.vertex_uv.exists {
            ov.uv = obj_mesh.vertex_uv.get(i);
        }

        if obj.fbx_position_scale != 1.0 {
            ov.pos.x *= obj.fbx_position_scale as Real;
            ov.pos.y *= obj.fbx_position_scale as Real;
            ov.pos.z *= obj.fbx_position_scale as Real;
        }
        if obj.fbx_rotation.w != 1.0 {
            ov.pos = quat_rotate_vec3(obj.fbx_rotation, ov.pos);
        }
        if obj.negate_xz {
            ov.pos.x *= -1.0;
            ov.pos.z *= -1.0;
            ov.normal.x *= -1.0;
            ov.normal.z *= -1.0;
        }

        if scale != 1.0 {
            ov.pos.x *= scale;
            ov.pos.y *= scale;
            ov.pos.z *= scale;
        }
    }
    for (i, fv) in fbx_verts.iter_mut().enumerate() {
        let mut fp = fbx_mesh.skinned_position.get(i);
        let mut fnrm = if fbx_mesh.skinned_normal.exists { fbx_mesh.skinned_normal.get(i) } else { Vec3::ZERO };
        if fbx_mesh.skinned_is_local {
            fp = transform_position(&fbx_node.geometry_to_world, fp);
            fnrm = transform_direction(&norm_mat, fnrm);
            fnrm = normalize(fnrm);
        }
        fv.pos = fp;
        fv.normal = fnrm;
        if fbx_mesh.vertex_uv.exists {
            fv.uv = fbx_mesh.vertex_uv.get(i);
        }

        if scale != 1.0 {
            fv.pos.x *= scale;
            fv.pos.y *= scale;
            fv.pos.z *= scale;
        }
    }

    obj_verts.sort_by(cmp_sub_vertex);
    fbx_verts.sort_by(cmp_sub_vertex);

    let mut obj_verts_left = obj_mesh.num_indices as i32;
    for i in (0..fbx_mesh.num_indices as i32).rev() {
        let v = fbx_verts[i as usize];

        let face_ix = find_face_index(fbx_mesh, i as usize);
        let face = fbx_mesh.faces[face_ix as usize];

        if obj.bad_faces
            && ((obj.bad_faces && face_has_duplicate_vertex(fbx_mesh, face))
                || (obj.line_faces && face.num_indices == 2)
                || (obj.point_faces && face.num_indices == 1))
        {
            continue;
        }

        let mut found = false;
        let missing_count: u32 = 0;
        let mut j = obj_verts_left - 1;
        while j >= 0 && obj_verts[j as usize].pos.x >= v.pos.x - tolerance {
            let o = obj_verts[j as usize];
            let dx = o.pos.x - v.pos.x;
            let dy = o.pos.y - v.pos.y;
            let dz = o.pos.z - v.pos.z;
            let mut dnx = o.normal.x - v.normal.x;
            let mut dny = o.normal.y - v.normal.y;
            let mut dnz = o.normal.z - v.normal.z;
            let mut du = o.uv.x - v.uv.x;
            let mut dv = o.uv.y - v.uv.y;

            if obj.bad_normals {
                dnx = 0.0;
                dny = 0.0;
                dnz = 0.0;
            }

            if obj.bad_uvs || !obj_mesh.vertex_uv.exists {
                du = 0.0;
                dv = 0.0;
            }

            let err = (dx * dx + dy * dy + dz * dz + dnx * dnx + dny * dny + dnz * dnz + du * du + dv * dv).sqrt();
            if err < tolerance {
                if err > p_err.max {
                    p_err.max = err;
                }
                p_err.sum += err;
                p_err.num += 1;

                obj_verts[j as usize] = obj_verts[(obj_verts_left - 1) as usize];
                obj_verts_left -= 1;
                found = true;
                break;
            }
            j -= 1;
        }

        if !found {
            ufbxt_assert!(missing_count < obj.allow_missing);
        }
    }
}

#[derive(Debug, Clone)]
struct ObjNode {
    cat_name: String,
    node: usize,
}

fn cmp_obj_node(a: &ObjNode, b: &ObjNode) -> Ordering {
    a.cat_name.cmp(&b.cat_name)
}

/// glibc's `bsearch`: which of several equal keys it lands on decides the node a mesh is matched to
fn bsearch<'a>(key: &str, nodes: &'a [ObjNode]) -> Option<&'a ObjNode> {
    let (mut l, mut u) = (0, nodes.len());
    while l < u {
        let idx = (l + u) / 2;
        match key.cmp(nodes[idx].cat_name.as_str()) {
            Ordering::Less => u = idx,
            Ordering::Greater => l = idx + 1,
            Ordering::Equal => return Some(&nodes[idx]),
        }
    }
    None
}

fn obj_group_key(groups: &mut [&str], remove_namespaces: bool) -> String {
    const CAT_CAP: usize = 4096;
    ufbxt_assert!(!groups.is_empty());
    groups.sort();

    let mut cat = String::new();
    let mut cat_len = 0;
    for &group in groups.iter() {
        let mut group = group;

        if remove_namespaces {
            while let Some(next) = group.find(':') {
                group = &group[next + 1..];
            }
        }

        let space = CAT_CAP - cat_len;
        let res = group.len() + 1;
        ufbxt_assert!(res < space);
        cat.push_str(group);
        cat.push('\n');
        cat_len += res;
    }

    cat.pop();
    cat
}

pub const OBJ_DIFF_FLAG_CHECK_DEFORMED_NORMALS: u32 = 0x1;
pub const OBJ_DIFF_FLAG_IGNORE_NORMALS: u32 = 0x2;
pub const OBJ_DIFF_FLAG_BAKED_ANIM: u32 = 0x4;
pub const OBJ_DIFF_FLAG_INHERIT_COMPENSATE: u32 = 0x4;

fn has_mesh(scene: &Scene, node: Option<usize>) -> bool {
    let Some(node) = node else { return false };
    let node = &scene.nodes[node];
    if node.mesh.is_some() {
        return true;
    }
    if has_mesh(scene, node.scale_helper) {
        return true;
    }
    if has_mesh(scene, node.geometry_transform_helper) {
        return true;
    }
    false
}

fn subdivision_boundary(index: i64) -> SubdivisionBoundary {
    match index {
        0 => SubdivisionBoundary::Default,
        1 => SubdivisionBoundary::Legacy,
        2 => SubdivisionBoundary::SharpCorners,
        3 => SubdivisionBoundary::SharpNone,
        4 => SubdivisionBoundary::SharpBoundary,
        _ => SubdivisionBoundary::SharpInterior,
    }
}

const SUBDIVISION_BOUNDARY_COUNT: i64 = 6;

pub fn diff_to_obj(scene: &Scene, obj: &ObjFile, p_err: &mut DiffError, flags: u32) {
    let mut used_nodes: Vec<usize> = Vec::with_capacity(obj.meshes.len());

    const MAX_GROUPS: usize = 64;

    let mut obj_nodes: Vec<ObjNode> = Vec::with_capacity(scene.nodes.len());

    for (node_i, node) in scene.nodes.iter().enumerate() {
        if node.element.name.is_empty() {
            continue;
        }

        let mut groups: Vec<&str> = Vec::new();
        let mut parent = Some(node_i);
        while let Some(p) = parent {
            let parent_node = &scene.nodes[p];
            if parent_node.is_root {
                break;
            }
            if obj.root_groups_at_bone && parent_node.attrib_type == ElementType::Bone {
                break;
            }

            ufbxt_assert!(groups.len() < MAX_GROUPS);
            groups.push(&parent_node.element.name);
            parent = parent_node.parent;
        }

        if groups.is_empty() {
            continue;
        }

        let cat_name = obj_group_key(&mut groups, obj.remove_namespaces);
        obj_nodes.push(ObjNode { cat_name, node: node_i });
    }

    obj_nodes.sort_by(cmp_obj_node);

    for (mesh_i, obj_mesh) in obj.meshes.iter().enumerate() {
        if obj_mesh.num_indices == 0 {
            continue;
        }

        let mut node: Option<usize> = None;
        let mut groups: Vec<&str> = obj_mesh.groups.iter().map(String::as_str).collect();

        if obj.match_by_order {
            ufbxt_assert!(mesh_i + 1 < scene.nodes.len());
            node = Some(mesh_i + 1);
        }

        if node.is_none() && !groups.is_empty() {
            let cat_name = obj_group_key(&mut groups, obj.remove_namespaces);
            if let Some(found) = bsearch(&cat_name, &obj_nodes)
                && has_mesh(scene, Some(found.node))
            {
                let seen = used_nodes.contains(&found.node);
                if !seen {
                    node = Some(found.node);
                }
            }
        }

        if node.is_none()
            && let Some(original_group) = &obj_mesh.original_group
            && !original_group.is_empty()
        {
            if obj.has_empty && original_group == "ufbx:empty" {
                for (i, empty_node) in scene.nodes.iter().enumerate().skip(1) {
                    if empty_node.element.name.is_empty() && empty_node.mesh.is_some() {
                        node = Some(i);
                        break;
                    }
                }
            } else {
                node = find_node(scene, original_group);
            }
        }

        if node.is_none() {
            for name in &groups {
                node = find_node(scene, name);
                if node.is_none() && obj.exporter == ObjExporter::Blender {
                    // Blender concatenates _Material to names
                    let name_len = name.find('_').unwrap_or(name.len());
                    node = find_node(scene, &name[..name_len]);
                }

                if let Some(n) = node
                    && has_mesh(scene, Some(n))
                {
                    let seen = used_nodes.contains(&n);
                    if !seen {
                        break;
                    }
                }
            }
        }

        if obj.ignore_duplicates {
            if node.is_none() || !has_mesh(scene, node) {
                let is_ncl = obj_mesh.groups.iter().any(|group| group.contains("_ncl1_"));
                if is_ncl {
                    continue;
                }
            }

            if let Some(n) = node
                && let Some(parent) = scene.nodes[n].parent
            {
                let name = &scene.nodes[n].element.name;
                let duplicate_name = scene.nodes[parent].children.iter().any(|&child| child != n && scene.nodes[child].element.name == *name);
                if duplicate_name {
                    continue;
                }
            }
        }

        if node.is_none() {
            ufbxt_assert!(scene.meshes.len() == 1);
            ufbxt_assert!(scene.meshes[0].element.instances.len() == 1);
            node = Some(scene.meshes[0].element.instances[0]);
        }

        ufbxt_assert!(node.is_some());
        let mut node_ix = node.unwrap();
        if let Some(helper) = scene.nodes[node_ix].scale_helper {
            node_ix = helper;
        }
        if let Some(helper) = scene.nodes[node_ix].geometry_transform_helper {
            node_ix = helper;
        }
        let node = &scene.nodes[node_ix];

        let mesh = node.mesh.map(|m| &scene.meshes[m]);

        let mut scale = 1.0f64;

        if obj.normalize_units && scene.settings.unit_meters != 1.0 {
            scale *= scene.settings.unit_meters;
        }

        if obj.position_scale != 1.0 {
            scale *= obj.position_scale;
        }
        if size_of::<Real>() == size_of::<f32>() {
            scale *= obj.position_scale_float;
        }
        if flags & OBJ_DIFF_FLAG_BAKED_ANIM != 0 {
            scale *= obj.position_scale_bake;
        }
        if flags & OBJ_DIFF_FLAG_INHERIT_COMPENSATE != 0 {
            scale *= obj.position_scale_compensate;
        }

        used_nodes.push(node_ix);

        if mesh.is_none() && node.attrib_type == ElementType::NurbsSurface {
            let surface = node.attrib.and_then(|id| scene.as_nurbs_surface(id)).expect("a NURBS surface node has its surface");
            let opts = TessellateSurfaceOpts {
                span_subdivision_u: surface.span_subdivision_u as usize,
                span_subdivision_v: surface.span_subdivision_v as usize,
                ..Default::default()
            };
            let tess_mesh = tessellate_nurbs_surface(surface, &opts);
            ufbxt_assert!(tess_mesh.is_ok());
            let tess_mesh = tess_mesh.unwrap();

            check_mesh(scene, &tess_mesh);
            match_obj_mesh(obj, node, &tess_mesh, obj_mesh, p_err, scale as Real);

            continue;
        }

        ufbxt_assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        if mesh.skin_deformers.len() > 1 {
            continue;
        }

        let mut mat = node.geometry_to_world;
        let mut norm_mat = get_compatible_matrix_for_normals(node);

        if obj.transform_skin && !mesh.skin_deformers.is_empty() {
            let mut node_bone = node.bind_pose.and_then(|pose| get_bone_pose(&scene.poses[pose], node_ix));

            if node_bone.is_none() {
                for &inst in &mesh.element.instances {
                    node_bone = scene.nodes[inst].bind_pose.and_then(|pose| get_bone_pose(&scene.poses[pose], inst));
                    if node_bone.is_some() {
                        break;
                    }
                }
            }

            ufbxt_assert!(node_bone.is_some());
            let inv_bind = matrix_invert(&node_bone.unwrap().bone_to_world);
            mat = matrix_mul(&mat, &inv_bind);
            norm_mat = matrix_for_normals(&mat);
        }

        if !obj.no_subdivision && matches!(mesh.subdivision_display_mode, SubdivisionDisplayMode::Smooth | SubdivisionDisplayMode::HullAndSmooth) {
            let mut opts = SubdivideOpts { evaluate_source_vertices: true, evaluate_skin_weights: true, ..Default::default() };

            let uv_boundary = node.element.props.find_int("ufbx:UVBoundary", -1);

            if (0..SUBDIVISION_BOUNDARY_COUNT).contains(&uv_boundary) {
                opts.uv_boundary = subdivision_boundary(uv_boundary);
            } else {
                opts.uv_boundary = SubdivisionBoundary::SharpBoundary;
            }

            let sub_mesh = load3d::subdivide_mesh(scene, mesh, mesh.subdivision_preview_levels as usize, &opts);
            ufbxt_assert!(sub_mesh.is_ok());
            let sub_mesh = sub_mesh.unwrap();

            check_mesh(scene, mesh);

            check_source_vertices(&sub_mesh, mesh, p_err, 0.0);

            check_mesh(scene, &sub_mesh);
            match_obj_mesh(obj, node, &sub_mesh, obj_mesh, p_err, scale as Real);

            continue;
        }

        if !(obj.bad_faces || obj.line_faces || obj.point_faces) {
            ufbxt_assert!(obj_mesh.num_faces == mesh.num_faces);
            ufbxt_assert!(obj_mesh.num_indices == mesh.num_indices);
        }

        let mut check_normals = true;
        if obj.bad_normals {
            check_normals = false;
        }
        if flags & OBJ_DIFF_FLAG_CHECK_DEFORMED_NORMALS == 0 && !mesh.all_deformers.is_empty() {
            check_normals = false;
        }
        if flags & OBJ_DIFF_FLAG_IGNORE_NORMALS != 0 {
            check_normals = false;
        }

        let uv_set_name = node.element.props.find_string("currentUVSet", "");
        let mut vertex_uv = &mesh.vertex_uv;
        if !uv_set_name.is_empty() {
            for uv_set in &mesh.uv_sets {
                if uv_set.name == uv_set_name {
                    vertex_uv = &uv_set.vertex_uv;
                }
            }
        }

        if obj.bad_order {
            match_obj_mesh(obj, node, mesh, obj_mesh, p_err, scale as Real);
        } else {
            let mut obj_face_ix = 0;

            // Assume that the indices are in the same order!
            for face_ix in 0..mesh.num_faces {
                // ufbx reads past the mesh's faces here where faces it skips are left at the end
                let obj_face = obj_mesh.faces.get(obj_face_ix).copied().unwrap_or_default();
                let face = mesh.faces[face_ix];

                if (obj.bad_faces && face_has_duplicate_vertex(mesh, face))
                    || (obj.line_faces && face.num_indices == 2)
                    || (obj.point_faces && face.num_indices == 1)
                {
                    continue;
                } else if !obj.bad_faces {
                    ufbxt_assert!(obj_face.index_begin == face.index_begin);
                }

                ufbxt_assert!(obj_face.num_indices == face.num_indices);
                obj_face_ix += 1;

                for i in 0..face.num_indices as usize {
                    let oix = obj_face.index_begin as usize + i;
                    let fix = face.index_begin as usize + i;

                    let mut op = obj_mesh.vertex_position.get(oix);
                    let mut fp = mesh.skinned_position.get(fix);
                    let mut on = if check_normals { obj_mesh.vertex_normal.get(oix) } else { Vec3::ZERO };
                    let mut fnrm = if check_normals { mesh.skinned_normal.get(fix) } else { Vec3::ZERO };

                    if mesh.skinned_is_local || obj.transform_skin {
                        fp = transform_position(&mat, fp);
                        fnrm = transform_direction(&norm_mat, fnrm);

                        let fn_len = (fnrm.x * fnrm.x + fnrm.y * fnrm.y + fnrm.z * fnrm.z).sqrt();
                        if fn_len > 0.00001f32 as Real {
                            fnrm.x /= fn_len;
                            fnrm.y /= fn_len;
                            fnrm.z /= fn_len;
                        }

                        let on_len = (on.x * on.x + on.y * on.y + on.z * on.z).sqrt();
                        if on_len > 0.00001f32 as Real {
                            on.x /= on_len;
                            on.y /= on_len;
                            on.z /= on_len;
                        }
                    }

                    if obj.fbx_position_scale != 1.0 {
                        fp.x *= obj.fbx_position_scale as Real;
                        fp.y *= obj.fbx_position_scale as Real;
                        fp.z *= obj.fbx_position_scale as Real;
                    }
                    if obj.fbx_rotation.w != 1.0 {
                        fp = quat_rotate_vec3(obj.fbx_rotation, fp);
                    }

                    if scale != 1.0 {
                        fp.x *= scale as Real;
                        fp.y *= scale as Real;
                        fp.z *= scale as Real;
                        op.x *= scale as Real;
                        op.y *= scale as Real;
                        op.z *= scale as Real;
                    }

                    if obj.negate_xz {
                        op.x *= -1.0;
                        op.z *= -1.0;
                        on.x *= -1.0;
                        on.z *= -1.0;
                    }

                    if obj.ignore_unskinned && !mesh.skin_deformers.is_empty() {
                        let skin = &scene.skin_deformers[mesh.skin_deformers[0]];
                        let vert = skin.vertices[mesh.vertex_indices[fix] as usize];
                        if vert.num_weights == 0 {
                            continue;
                        }
                    }

                    assert_close_vec3(p_err, op, fp);

                    if check_normals && !mesh.generated_normals {
                        let on2 = on.x * on.x + on.y * on.y + on.z * on.z;
                        let fn2 = fnrm.x * fnrm.x + fnrm.y * fnrm.y + fnrm.z * fnrm.z;
                        if on2 > 0.01f32 as Real && fn2 > 0.01f32 as Real {
                            assert_close_vec3(p_err, on, fnrm);
                        }
                    }

                    if obj_mesh.vertex_uv.exists && !obj.bad_uvs {
                        ufbxt_assert!(mesh.vertex_uv.exists);
                        let ovx = obj_mesh.vertex_uv.indices[oix];

                        if ovx == NO_INDEX {
                            if !obj.missing_uvs {
                                ufbxt_assert!(vertex_uv.indices[fix] == NO_INDEX);
                            }
                        } else {
                            let ou = obj_mesh.vertex_uv.get(oix);
                            let fu = vertex_uv.get(fix);
                            assert_close_vec2_threshold(p_err, ou, fu, obj.uv_tolerance);
                            assert_close_vec2_threshold(p_err, ou, fu, obj.uv_tolerance);
                        }
                    }
                }
            }
        }
    }
}

pub fn ufbxt_subdivide_mesh(scene: &Scene, mesh: &Mesh, level: usize, opts: Option<&SubdivideOpts>) -> Result<Mesh, Error> {
    let memory_opts = opts.cloned().unwrap_or_default();
    let result = load3d::subdivide_mesh(scene, mesh, level, &memory_opts)?;

    let memory_result = load3d::subdivide_mesh(scene, mesh, level, &memory_opts);
    ufbxt_assert!(memory_result.is_ok());

    let res = memory_result.unwrap().subdivision_result;
    ufbxt_assert!(res.is_some());

    Ok(result)
}

pub fn check_baked_anim(scene: &Scene, bake: Option<&BakedAnim>) {
    let Some(bake) = bake else { return };

    for i in 0..bake.nodes.len() {
        if i > 0 {
            ufbxt_assert!(bake.nodes[i - 1].typed_id < bake.nodes[i].typed_id);
        }

        let bake_node = &bake.nodes[i];
        let scene_node = &scene.nodes[bake_node.typed_id as usize];
        ufbxt_assert!(scene_node.element.element_id == bake_node.element_id);

        let r = find_baked_node(bake, bake_node.typed_id as usize);
        ufbxt_assert!(r.is_some_and(|r| std::ptr::eq(r, bake_node)));
    }
    for i in 0..bake.elements.len() {
        if i > 0 {
            ufbxt_assert!(bake.elements[i - 1].element_id < bake.elements[i].element_id);
        }

        let bake_element = &bake.elements[i];
        let scene_element = scene.element(bake_element.element_id);

        let r = find_baked_element(bake, scene_element.element_id);
        ufbxt_assert!(r.is_some_and(|r| std::ptr::eq(r, bake_element)));
    }
}

pub fn ufbxt_bake_anim(scene: &Scene, anim: &Anim, opts: Option<&BakeOpts>) -> Result<BakedAnim, Error> {
    let memory_opts = opts.cloned().unwrap_or_default();
    let result = load3d::bake_anim(scene, anim, &memory_opts)?;
    check_baked_anim(scene, Some(&result));

    let memory_result = load3d::bake_anim(scene, anim, &memory_opts);
    ufbxt_assert!(memory_result.is_ok());

    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pixel16 {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Image16 {
    pub pixels: Vec<Pixel16>,
    pub width: u32,
    pub height: u32,
}

pub fn px16(r: u16, g: u16, b: u16, a: u16) -> Pixel16 {
    Pixel16 { r, g, b, a }
}

fn calloc<T: Clone + Default>(count: usize) -> Option<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(count).ok()?;
    v.resize(count, T::default());
    Some(v)
}

/// http://www.libpng.org/pub/png/spec/1.2/PNG-Contents.html
pub fn read_png(data: &[u8], p_error: Option<&mut Option<&'static str>>) -> Image16 {
    let mut image = Image16::default();
    let empty_pixel = Pixel16::default();

    let end = data.len();
    let mut ptr = 0;
    let mut bit_depth: u8 = 0;
    let mut pixel_values: u8 = 0;
    let mut pixel_format: u8 = 0;
    let mut deflate_data: Vec<u8> = Vec::new();
    let mut palette: Vec<Pixel16> = Vec::new();
    let mut palette_size = 0;
    const LACE_NONE: [u8; 8] = [0, 0, 1, 1, 0, 0, 0, 0];
    const LACE_ADAM7: [u8; 32] = [0, 0, 8, 8, 4, 0, 8, 8, 0, 4, 4, 8, 2, 0, 4, 4, 0, 2, 2, 4, 1, 0, 2, 2, 0, 1, 1, 2, 0, 0, 0, 0];
    let mut scale: u32 = 1;
    let mut lace: &[u8] = &LACE_NONE; // Interlacing pattern (x0,y0,dx,dy)
    let mut adam7 = false;
    let mut trns = Pixel16::default();
    let mut error: Option<&'static str> = None;

    if end - ptr < 8 {
        error = Some("file header truncated");
    }
    if error.is_none() && data[ptr..ptr + 8] != *b"\x89PNG\r\n\x1a\n" {
        error = Some("bad_file_header");
    }
    ptr += 8;

    while error.is_none() {
        if end - ptr < 8 {
            error = Some("chunk header truncated");
            break;
        }
        let chunk_len = u32::from_be_bytes([data[ptr], data[ptr + 1], data[ptr + 2], data[ptr + 3]]);
        let tag = &data[ptr + 4..ptr + 8];
        ptr += 8;
        if ((end - ptr) as u32) < chunk_len.wrapping_add(4) {
            error = Some("chunk data truncated");
            break;
        }
        let p = &data[ptr..];
        if tag == b"IHDR" && chunk_len >= 13 {
            image.width = u32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            image.height = u32::from_be_bytes([p[4], p[5], p[6], p[7]]);
            bit_depth = p[8];
            pixel_format = p[9];
            pixel_values = match pixel_format {
                0 => 1,
                2 => 3,
                3 => 1,
                4 => 2,
                6 => 4,
                _ => {
                    error = Some("unknown pixel format");
                    continue;
                }
            };
            let mut i = 0u32;
            while i < 16 && pixel_format != 3 {
                scale |= 1u32 << i;
                i += bit_depth as u32;
            }
            if p[12] != 0 {
                lace = &LACE_ADAM7;
                adam7 = true;
            }
            if p[10] != 0 || p[11] != 0 {
                error = Some("unknown settings");
                break;
            }
        } else if tag == b"IDAT" {
            deflate_data.extend_from_slice(&p[..chunk_len as usize]);
        } else if tag == b"PLTE" {
            palette_size = (chunk_len / 3) as usize;
            palette = (0..palette_size)
                .map(|i| Pixel16 { r: p[i * 3] as u16 * 0x101, g: p[i * 3 + 1] as u16 * 0x101, b: p[i * 3 + 2] as u16 * 0x101, a: 0 })
                .collect();
        } else if tag == b"IEND" {
            break;
        } else if tag == b"tRNS" {
            if pixel_format == 2 && chunk_len >= 6 {
                trns.r = ((p[0] as u32) << 8 | p[1] as u32).wrapping_mul(scale) as u16;
                trns.g = ((p[2] as u32) << 8 | p[3] as u32).wrapping_mul(scale) as u16;
                trns.b = ((p[4] as u32) << 8 | p[5] as u32).wrapping_mul(scale) as u16;
            } else if pixel_format == 0 && chunk_len >= 2 {
                let v = ((p[0] as u32) << 8 | p[1] as u32).wrapping_mul(scale) as u16;
                trns.r = v;
                trns.g = v;
                trns.b = v;
            } else if pixel_format == 3 {
                for (i, &alpha) in p[..chunk_len as usize].iter().enumerate() {
                    if palette_size != 0
                        && let Some(px) = palette.get_mut(i)
                    {
                        px.a = alpha as u16 * 0x101;
                    }
                }
            }
        }
        ptr += chunk_len.wrapping_add(4) as usize;
    }

    let bpp = (pixel_values as usize * bit_depth as usize).div_ceil(8);
    let stride = (image.width.wrapping_mul(pixel_values as u32).wrapping_mul(bit_depth as u32).wrapping_add(7) / 8) as usize;
    if image.width == 0 || image.height == 0 {
        error = Some("bad image size");
    }
    let src_size = (image.height as usize).wrapping_mul(stride).wrapping_add(image.height.wrapping_mul(if adam7 { 14 } else { 1 }) as usize);
    let mut src: Option<Vec<u8>> = None;
    let mut dst: Option<Vec<u8>> = None;
    let mut pixels: Option<Vec<Pixel16>> = None;
    if error.is_none() {
        src = calloc(src_size);
        dst = calloc((image.height.wrapping_add(1) as usize).wrapping_mul(stride + bpp));
        pixels = calloc(image.width.wrapping_mul(image.height) as usize);
    }
    if src.is_none() || dst.is_none() || pixels.is_none() {
        error = Some("allocation failure");
    }

    let mut retain = InflateRetain::new();
    let mut input = InflateInput { total_size: deflate_data.len(), data: &deflate_data, ..Default::default() };

    let mut src = src.unwrap_or_default();
    let mut dst = dst.unwrap_or_default();
    let mut pixels = pixels.unwrap_or_default();

    let res = inflate(&mut src, &mut input, &mut retain);
    if res != src_size as isize {
        error = Some("deflate error");
    }
    let mut sp = 0;
    let sp_end = src.len();

    let mut li = 0;
    while error.is_none() && lace[li + 2] != 0 {
        let (l0, l1, l2, l3) = (lace[li] as i32, lace[li + 1] as i32, lace[li + 2] as i32, lace[li + 3] as i32);
        let width = (image.width as i32 - l0 + l2 - 1) / l2;
        let height = (image.height as i32 - l1 + l3 - 1) / l3;
        if width <= 0 || height <= 0 {
            li += 4;
            continue;
        }
        let lace_stride = (width.wrapping_mul(pixel_values as i32).wrapping_mul(bit_depth as i32).wrapping_add(7) / 8) as usize;
        if sp_end - sp < (height as usize).wrapping_mul(1 + lace_stride) {
            error = Some("data truncated");
            break;
        }

        let (dx, dy) = (bpp, stride + bpp);
        for y in 0..height as usize {
            let dp = (stride + bpp) * (1 + y) + bpp;
            let filter = src[sp];
            sp += 1;
            for x in 0..lace_stride {
                let s = src[sp];
                sp += 1;
                let d = dp + x;
                match filter {
                    0 => dst[d] = s,                                                                // 6.2: No filter
                    1 => dst[d] = dst[d - dx].wrapping_add(s),                                      // 6.3: Sub (predict left)
                    2 => dst[d] = dst[d - dy].wrapping_add(s),                                      // 6.4: Up (predict top)
                    3 => dst[d] = ((dst[d - dx] as i32 + dst[d - dy] as i32) / 2 + s as i32) as u8, // 6.5: Average (top+left)
                    4 => {
                        // 6.6: Paeth (choose closest of 3 to estimate)
                        let (a, b, c) = (dst[d - dx] as i32, dst[d - dy] as i32, dst[d - dx - dy] as i32);
                        let p = a + b - c;
                        let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                        dst[d] = if pa <= pb && pa <= pc {
                            (a + s as i32) as u8
                        } else if pb <= pc {
                            (b + s as i32) as u8
                        } else {
                            (c + s as i32) as u8
                        };
                    }
                    _ => error = Some("unknown filter"),
                }
            }
        }

        for y in 0..height {
            let dr = ((stride + bpp) * (y as usize + 1) + bpp) as isize;
            for x in 0..width {
                let mut v = [0u16; 4];
                for c in 0..pixel_values as u32 {
                    let bit = (x as u32).wrapping_mul(pixel_values as u32).wrapping_add(c).wrapping_add(1).wrapping_mul(bit_depth as u32) as isize;
                    let hi = dst[(dr + ((bit - 9) >> 3)) as usize] as u32;
                    let lo = dst[(dr + ((bit - 1) >> 3)) as usize] as u32;
                    let raw = (hi << 8 | lo) >> ((8 - bit) & 7);
                    v[c as usize] = (raw & 1u32.wrapping_shl(bit_depth as u32).wrapping_sub(1)).wrapping_mul(scale) as u16;
                }

                let mut px = match pixel_format {
                    0 => px16(v[0], v[0], v[0], 0xffff),
                    2 => px16(v[0], v[1], v[2], 0xffff),
                    3 => {
                        if (v[0] as usize) < palette_size {
                            palette[v[0] as usize]
                        } else {
                            empty_pixel
                        }
                    }
                    4 => px16(v[0], v[0], v[0], v[1]),
                    6 => px16(v[0], v[1], v[2], v[3]),
                    _ => empty_pixel,
                };
                if px == trns {
                    px = empty_pixel;
                }
                let row = (l1 + y * l3) as u32;
                let col = (l0 + x * l2) as u32;
                pixels[row.wrapping_mul(image.width).wrapping_add(col) as usize] = px;
            }
        }
        li += 4;
    }

    if let Some(error) = error {
        if let Some(p_error) = p_error {
            *p_error = Some(error);
        }
        image.pixels = Vec::new();
        image.width = 0;
        image.height = 0;
    } else {
        image.pixels = pixels;
    }

    image
}

#[allow(clippy::excessive_precision)] // ufbx's `0.947867298578f`, a float
pub fn srgb_to_linear(a: Real) -> Real {
    let a = clamp(a, 0.0, 1.0);
    if a <= 0.04045 { 0.0773993808 * a } else { (a as f64 * 0.947867298578f32 as f64 + 0.0521327014218).powf(2.4) as Real }
}

pub fn file_size(name: &str) -> usize {
    std::fs::File::open(name).and_then(|mut file| file.seek(SeekFrom::End(0))).map(|size| size as usize).unwrap_or(0)
}

pub fn read_file_ex(name: &str) -> Option<Vec<u8>> {
    if name.ends_with(".gz") {
        let gz_data = read_file(name)?;
        decompress_gzip(&gz_data)
    } else {
        read_file(name)
    }
}

pub fn load_obj_file(file_name: &str, _opts: Option<&LoadObjOpts>) -> ObjFile {
    let buf = data_path(&format!("{file_name}.obj"));

    let obj_file = read_file(&buf).map(|obj_data| load_obj(&obj_data, None));
    ufbxt_assert!(obj_file.is_some());

    obj_file.unwrap()
}

/// `ufbxt_check_frame_flags` in test/test_transform.h
pub const CHECK_FRAME_SCALE_100: u32 = 0x1;
pub const CHECK_FRAME_Z_TO_Y_UP: u32 = 0x2;
pub const CHECK_FRAME_NO_EXTRAPOLATE: u32 = 0x4;

pub fn check_frame_imp(scene: &Scene, err: &mut DiffError, check_normals: bool, file_name: &str, anim_name: Option<&str>, time: f64, flags: u32) {
    ufbxt_hintf!("Frame from '{}' {} time {:.2}s", anim_name.unwrap_or("(implicit animation)"), file_name, time);

    let mut opts = EvaluateOpts { evaluate_skinning: true, evaluate_caches: true, load_external_files: true, ..Default::default() };

    if flags & CHECK_FRAME_NO_EXTRAPOLATE != 0 {
        opts.evaluate_flags |= EVALUATE_FLAG_NO_EXTRAPOLATION;
    }

    let mut obj_file = load_obj_file(file_name, None);

    if flags & CHECK_FRAME_SCALE_100 != 0 {
        obj_file.fbx_position_scale = 100.0;
    }

    if flags & CHECK_FRAME_Z_TO_Y_UP != 0 {
        // -90deg rotation around X
        obj_file.fbx_rotation.w = 0.70710678f32 as Real;
        obj_file.fbx_rotation.x = -0.70710678f32 as Real;
    }

    let mut anim: &Anim = &scene.anim;

    if let Some(anim_name) = anim_name {
        for stack in &scene.anim_stacks {
            if stack.element.name.contains(anim_name) {
                ufbxt_assert!(!stack.layers.is_empty());
                anim = &stack.anim;
                break;
            }
        }
    }

    let eval = evaluate_scene(scene, anim, time, &opts);
    ufbxt_assert!(eval.is_ok());
    let eval = eval.unwrap();

    check_scene(&eval);

    let mut diff_flags = 0;
    if check_normals {
        diff_flags |= OBJ_DIFF_FLAG_CHECK_DEFORMED_NORMALS;
    }
    diff_to_obj(&eval, &obj_file, err, diff_flags);
}

pub fn check_frame(scene: &Scene, err: &mut DiffError, check_normals: bool, file_name: &str, anim_name: Option<&str>, time: f64) {
    check_frame_imp(scene, err, check_normals, file_name, anim_name, time, 0);
}

/// `element_id` `NO_INDEX` for a warning about no element, `count` `usize::MAX` for any count but zero
pub fn check_warning_imp(scene: &Scene, kind: WarningType, element_id: u32, count: usize, substring: Option<&str>) {
    let mut found = false;
    for warning in &scene.metadata.warnings {
        if warning.kind == kind && warning.element_id == element_id {
            if let Some(substring) = substring {
                ufbxt_assert!(warning.description.contains(substring));
            }
            if count == usize::MAX {
                ufbxt_assert!(warning.count > 0);
            } else {
                ufbxt_assert!(warning.count == count);
            }
            found = true;
            break;
        }
    }

    if !found {
        ufbxt_logf!("Warning not found: {}", kind as i32);
    }
    ufbxt_assert!(found);
}

/// `#name` names the node it is an instance of
pub fn check_warning_ix(
    scene: &Scene,
    kind: WarningType,
    element_type: ElementType,
    element_name: Option<&str>,
    element_index: u32,
    count: usize,
    substring: Option<&str>,
) {
    let mut element_id = NO_INDEX;
    if let Some(element_name) = element_name {
        let mut match_index = 0;
        for i in 0..scene.elements.len() {
            let element = scene.element(i as u32);
            if element.element_type != element_type {
                continue;
            }

            let mut name_match = false;
            if let Some(instance_name) = element_name.strip_prefix('#') {
                for &instance in &element.instances {
                    if scene.nodes[instance].element.name == instance_name {
                        name_match = true;
                        break;
                    }
                }
            } else if element.name == element_name {
                name_match = true;
            }

            if name_match {
                if match_index == element_index {
                    element_id = element.element_id;
                    break;
                }
                match_index += 1;
            }
        }
        ufbxt_assert!(element_id != NO_INDEX);
    }

    check_warning_imp(scene, kind, element_id, count, substring);
}

pub fn check_warning(scene: &Scene, kind: WarningType, element_type: ElementType, element_name: Option<&str>, count: usize, substring: Option<&str>) {
    check_warning_ix(scene, kind, element_type, element_name, 0, count, substring);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefTransform {
    pub translation: Vec3,
    pub rotation_euler: Vec3,
    pub scale: Vec3,
}

pub fn check_transform(err: &mut DiffError, name: &str, transform: Transform, ref_transform: RefTransform) {
    let rotation_euler = quat_to_euler(transform.rotation, RotationOrder::Xyz);
    ufbxt_hintf!(
        "{} {{ {{ {:.2}, {:.2}, {:.2} }}, {{ {:.2}, {:.2}, {:.2} }}, {{ {:.2}, {:.2}, {:.2} }} }}",
        name,
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
        rotation_euler.x,
        rotation_euler.y,
        rotation_euler.z,
        transform.scale.x,
        transform.scale.y,
        transform.scale.z
    );

    assert_close_vec3(err, transform.translation, ref_transform.translation);
    assert_close_vec3(err, rotation_euler, ref_transform.rotation_euler);
    assert_close_vec3(err, transform.scale, ref_transform.scale);

    ufbxt_hintf!("");
}

pub const REF_TRANSFORM_IDENTITY: RefTransform = RefTransform { translation: Vec3::ZERO, rotation_euler: Vec3::ZERO, scale: Vec3::ONE };

/// Two NaNs taken alike
pub fn assert_vec3_equal(v: Vec3, x: Real, y: Real, z: Real) {
    ufbxt_assert!(if x.is_nan() { v.x.is_nan() } else { v.x == x });
    ufbxt_assert!(if y.is_nan() { v.y.is_nan() } else { v.y == y });
    ufbxt_assert!(if z.is_nan() { v.z.is_nan() } else { v.z == z });
}

pub const CHECK_TRANSFORM_CONSISTENCY_NO_BAKE: u32 = 0x1;
pub const CHECK_TRANSFORM_CONSISTENCY_NO_EXTRAPOLATION: u32 = 0x2;

pub fn check_transform_consistency(err: &mut DiffError, scene: &Scene, sample_rate: f64, frame_count: u32, flags: u32) {
    let mut eval_flags = 0;
    let mut transform_flags = 0;
    if flags & CHECK_TRANSFORM_CONSISTENCY_NO_EXTRAPOLATION != 0 {
        eval_flags |= EVALUATE_FLAG_NO_EXTRAPOLATION;
        transform_flags |= TRANSFORM_FLAG_NO_EXTRAPOLATION;
    }

    let bake_opts = BakeOpts { resample_rate: sample_rate, evaluate_flags: eval_flags, max_keyframe_segments: 4096, ..Default::default() };

    let bake = load3d::bake_anim(scene, &scene.anim, &bake_opts);
    if let Err(error) = &bake {
        super::runner::log_error(error);
    }
    ufbxt_assert!(bake.is_ok());
    let bake = bake.unwrap();

    for frame in 0..=frame_count {
        let eval_opts = EvaluateOpts { evaluate_flags: eval_flags, ..Default::default() };

        let time = frame as f64 / sample_rate;
        let state = evaluate_scene(scene, &scene.anim, time, &eval_opts);
        if let Err(error) = &state {
            super::runner::log_error(error);
        }
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();

        for bake_node in &bake.nodes {
            let node_ix = bake_node.typed_id as usize;

            let state_transform = state.nodes[node_ix].local_transform;

            let eval_transform = evaluate_transform_flags(scene, &scene.anim, node_ix, time, transform_flags);

            assert_close_vec3(err, state_transform.translation, eval_transform.translation);
            assert_close_quat(err, state_transform.rotation, eval_transform.rotation);
            assert_close_vec3(err, state_transform.scale, eval_transform.scale);

            if flags & CHECK_TRANSFORM_CONSISTENCY_NO_BAKE == 0 {
                let bake_transform = Transform {
                    translation: evaluate_baked_vec3(&bake_node.translation_keys, time),
                    rotation: evaluate_baked_quat(&bake_node.rotation_keys, time),
                    scale: evaluate_baked_vec3(&bake_node.scale_keys, time),
                };

                assert_close_vec3(err, state_transform.translation, bake_transform.translation);
                assert_close_quat(err, state_transform.rotation, bake_transform.rotation);
                assert_close_vec3(err, state_transform.scale, bake_transform.scale);
            }
        }
    }
}

pub fn scale_to_cm_opts() -> LoadOpts {
    LoadOpts { target_unit_meters: 0.01f32 as Real, ..Default::default() }
}

pub fn scale_helper_opts() -> LoadOpts {
    LoadOpts { inherit_mode_handling: InheritModeHandling::HelperNodes, ..Default::default() }
}

pub fn fnv1a(data: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for &b in data {
        hash = (hash ^ b as u32).wrapping_mul(0x01000193);
    }
    hash
}

pub fn retain_dom_opts() -> LoadOpts {
    LoadOpts { retain_dom: true, ..Default::default() }
}

pub fn check_blob_content(blob: &[u8], filename: &str) {
    ufbxt_assert!(!blob.is_empty());

    let buf = data_path(filename);
    let reference = read_file(&buf);
    ufbxt_assert!(reference.is_some());
    let reference = reference.unwrap();

    ufbxt_assert!(reference.len() == blob.len());
    ufbxt_assert!(reference == blob);
}

pub fn check_texture_content(scene: &Scene, texture: Option<usize>, filename: &str) {
    let buf = format!("textures/{filename}");
    ufbxt_assert!(texture.is_some());
    check_blob_content(&scene.textures[texture.unwrap()].content, &buf);
}

pub fn check_material_texture_ex(scene: &Scene, texture: Option<usize>, directory: &str, filename: &str, require_content: bool) {
    let tex = &scene.textures[texture.unwrap()];

    let buf = format!("{directory}{filename}");
    ufbxt_assert!(tex.relative_filename == buf);

    if require_content && (scene.metadata.version >= 7000 || !scene.metadata.ascii) {
        ufbxt_assert!(!tex.content.is_empty());
    }

    if !tex.content.is_empty() {
        check_texture_content(scene, texture, filename);
    }
}

pub fn check_material_texture(scene: &Scene, texture: Option<usize>, filename: &str, require_content: bool) {
    check_material_texture_ex(scene, texture, "textures\\", filename, require_content);
}

pub fn element_dom_node<'a>(scene: &'a Scene, element: &Element) -> Option<&'a DomNode> {
    scene.dom_node(element.dom_node.as_deref()?)
}

pub fn evaluate_baked_matrix(scene: &Scene, bake: &BakedAnim, node: usize, time: f64) -> Matrix {
    let node = &scene.nodes[node];
    let mut transform = node.local_transform;
    for baked_node in &bake.nodes {
        if baked_node.typed_id == node.element.typed_id {
            if !baked_node.translation_keys.is_empty() {
                transform.translation = evaluate_baked_vec3(&baked_node.translation_keys, time);
            }
            if !baked_node.rotation_keys.is_empty() {
                transform.rotation = evaluate_baked_quat(&baked_node.rotation_keys, time);
            }
            if !baked_node.scale_keys.is_empty() {
                transform.scale = evaluate_baked_vec3(&baked_node.scale_keys, time);
            }
        }
    }

    let mut node_to_parent = transform_to_matrix(&transform);
    if let Some(parent) = node.parent {
        let parent_to_world = evaluate_baked_matrix(scene, bake, parent, time);
        node_to_parent = matrix_mul(&parent_to_world, &node_to_parent);
    }
    node_to_parent
}

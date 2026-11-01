//! Kept out of `types.rs`, which the oracle pairs with ufbx's.

use crate::matcher::is_format;
use crate::{FileFormat, Material, Vec3};

/// The format these bytes are taken for by their content alone, in the order a load tries them
pub fn detect_format(data: &[u8]) -> FileFormat {
    const LOOKAHEAD: usize = 0x4000;
    let mut lookahead = 32;
    loop {
        let head = &data[..lookahead.min(data.len())];
        for format in [FileFormat::Fbx, FileFormat::Obj, FileFormat::Mtl] {
            if is_format(head, format) {
                return format;
            }
        }
        if lookahead >= data.len() || lookahead >= LOOKAHEAD {
            break;
        }
        lookahead = (lookahead * 2).min(LOOKAHEAD);
    }
    if cfg!(feature = "format-3ds") && is_format(data, FileFormat::Tds) { FileFormat::Tds } else { FileFormat::Unknown }
}

impl Material {
    pub fn diffuse(&self) -> Option<Vec3> {
        let map = self.fbx.diffuse_color();
        map.has_value.then_some(Vec3::new(map.value_vec4.x, map.value_vec4.y, map.value_vec4.z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_is_told_by_its_content() {
        assert_eq!(detect_format(b"Kaydara FBX Binary  \x00\x1a\x00"), FileFormat::Fbx);
        assert_eq!(detect_format(b"; FBX 7.5.0 project file\n"), FileFormat::Fbx);
        assert_eq!(detect_format(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"), FileFormat::Obj);
        assert_eq!(detect_format(b"newmtl paint\nKd 1 0 0\n"), FileFormat::Mtl);
        assert_eq!(detect_format(b"solid hull\nfacet"), FileFormat::Unknown);
        assert_eq!(detect_format(b""), FileFormat::Unknown);
        assert_eq!(detect_format(b"MM"), FileFormat::Unknown);
        let tds = if cfg!(feature = "format-3ds") { FileFormat::Tds } else { FileFormat::Unknown };
        assert_eq!(detect_format(b"MM\x06\0\0\0"), tds);
    }

    #[test]
    fn a_line_past_the_first_look_counts_and_one_past_the_last_does_not() {
        let at = |n: usize| [vec![b'#'; n], b"\nf 1 2 3\n".to_vec()].concat();
        assert_eq!(detect_format(&at(100)), FileFormat::Obj);
        assert_eq!(detect_format(&at(0x4000)), FileFormat::Unknown);
    }

    #[test]
    #[cfg(feature = "format-obj")]
    fn a_colour_is_read_alike_in_any_format() {
        let opts = crate::LoadOpts { obj_mtl_data: b"newmtl paint\nKd 1 0.5 0\nnewmtl bare\n".to_vec(), ..crate::LoadOpts::default() };
        let scene = crate::load_memory(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl paint\nf 1 2 3\nusemtl bare\nf 1 3 2\n", &opts).unwrap();
        let of = |name: &str| scene.materials.iter().find(|m| m.element.name == name).unwrap().diffuse();
        assert_eq!(of("paint"), Some(Vec3::new(1.0, 0.5, 0.0)));
        assert_eq!(of("bare"), None);
    }
}

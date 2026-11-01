//! Kept out of `types.rs`, which the oracle pairs with ufbx's.

use crate::{Material, Vec3};

impl Material {
    pub fn diffuse(&self) -> Option<Vec3> {
        let map = self.fbx.diffuse_color();
        map.has_value.then_some(Vec3::new(map.value_vec4.x, map.value_vec4.y, map.value_vec4.z))
    }
}

#[cfg(all(test, feature = "format-obj"))]
mod tests {
    use super::*;

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

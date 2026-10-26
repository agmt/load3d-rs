//! Properties sort by the first four bytes of the name read as a big-endian key, then by the name.

use std::cmp::Ordering;

use crate::types::{Prop, Props, Real, Vec3, prop_flags};

/// `ufbxi_get_name_key` in ufbx.c
pub fn name_key(name: &[u8]) -> u32 {
    let mut key = 0u32;
    if name.len() >= 4 {
        key = (name[0] as u32) << 24 | (name[1] as u32) << 16 | (name[2] as u32) << 8 | name[3] as u32;
    } else {
        for i in 0..4 {
            key <<= 8;
            if i < name.len() {
                key |= name[i] as u32;
            }
        }
    }
    key
}

pub(crate) fn prop_less(a: &Prop, b: &Prop) -> Ordering {
    a.internal_key.cmp(&b.internal_key).then_with(|| a.name.as_bytes().cmp(b.name.as_bytes()))
}

fn cmp_prop_less_ref(p: &Prop, name: &str, key: u32) -> Ordering {
    p.internal_key.cmp(&key).then_with(|| p.name.as_bytes().cmp(name.as_bytes()))
}

impl Props {
    /// Part of `ufbx_find_prop_len` in ufbx.c
    pub fn find_own(&self, name: &str) -> Option<&Prop> {
        let key = name_key(name.as_bytes());
        let i = self.props.partition_point(|p| cmp_prop_less_ref(p, name, key) == Ordering::Less);
        self.props.get(i).filter(|p| p.internal_key == key && p.name == name)
    }

    /// Part of `ufbx_find_prop_len` in ufbx.c
    pub fn find_own_mut(&mut self, name: &str) -> Option<&mut Prop> {
        let key = name_key(name.as_bytes());
        let i = self.props.partition_point(|p| cmp_prop_less_ref(p, name, key) == Ordering::Less);
        self.props.get_mut(i).filter(|p| p.internal_key == key && p.name == name)
    }

    /// `ufbx_find_prop` in ufbx.c: In these props, then their defaults
    pub fn find(&self, name: &str) -> Option<&Prop> {
        let mut props = Some(self);
        while let Some(p) = props {
            if let Some(found) = p.find_own(name) {
                return Some(found);
            }
            props = p.defaults.as_deref();
        }
        None
    }

    /// With `ufbxi_find_prop_with_key` in ufbx.c: Passes over properties without a value
    pub(crate) fn find_prop(&self, name: &str) -> Option<&Prop> {
        let key = name_key(name.as_bytes());
        let mut props = Some(self);
        while let Some(p) = props {
            let i = p.props.partition_point(|q| q.internal_key < key);
            for q in &p.props[i..] {
                if q.internal_key > key {
                    break;
                }
                if q.name == name && q.flags & prop_flags::NO_VALUE == 0 {
                    return Some(q);
                }
            }
            props = p.defaults.as_deref();
        }
        None
    }

    pub fn find_real(&self, name: &str, def: Real) -> Real {
        self.find(name).map_or(def, |p| p.value_real())
    }
    pub fn find_vec3(&self, name: &str, def: Vec3) -> Vec3 {
        self.find(name).map_or(def, |p| p.value_vec3())
    }
    pub fn find_int(&self, name: &str, def: i64) -> i64 {
        self.find(name).map_or(def, |p| p.value_int)
    }
    pub fn find_bool(&self, name: &str, def: bool) -> bool {
        self.find(name).map_or(def, |p| p.value_int != 0)
    }
    pub fn find_string<'a>(&'a self, name: &str, def: &'a str) -> &'a str {
        self.find(name).map_or(def, |p| p.value_str.as_str())
    }
    pub fn find_blob<'a>(&'a self, name: &str, def: &'a [u8]) -> &'a [u8] {
        self.find(name).map_or(def, |p| p.value_blob.as_slice())
    }

    /// `ufbx_find_prop_concat` in ufbx.c
    pub fn find_concat(&self, parts: &[&str]) -> Option<&Prop> {
        let name: String = parts.concat();
        self.find(&name)
    }

    /// `ufbxi_find_real` in ufbx.c: Over values only, as are its kin below
    pub(crate) fn real(&self, name: &str, def: Real) -> Real {
        self.find_prop(name).map_or(def, |p| p.value_real())
    }
    /// `ufbxi_find_vec3` in ufbx.c
    pub(crate) fn vec3(&self, name: &str, def: Vec3) -> Vec3 {
        self.find_prop(name).map_or(def, |p| p.value_vec3())
    }
    /// `ufbxi_find_int` in ufbx.c
    pub(crate) fn int(&self, name: &str, def: i64) -> i64 {
        self.find_prop(name).map_or(def, |p| p.value_int)
    }
    pub(crate) fn find_enum(&self, name: &str, def: i64, max: i64) -> i64 {
        match self.find_prop(name) {
            Some(p) if p.value_int >= 0 && p.value_int <= max => p.value_int,
            _ => def,
        }
    }
}

pub(crate) fn sort_properties(props: &mut [Prop]) {
    props.sort_by(prop_less);
}

/// Of equal names the last one stays
pub(crate) fn deduplicate_properties(props: &mut Vec<Prop>) {
    if props.len() < 2 {
        return;
    }
    let mut out: Vec<Prop> = Vec::with_capacity(props.len());
    let n = props.len();
    for i in 0..n {
        if i + 1 < n && props[i].name == props[i + 1].name {
            continue;
        }
        out.push(std::mem::take(&mut props[i]));
    }
    *props = out;
}

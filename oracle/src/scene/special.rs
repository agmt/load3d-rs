//! The structs gen.py leaves to hand (its HANDWRITTEN table, which checks these field lists).

use super::generated::*;
use super::rt::{self, Cx, Kx};
use load3d as f;
use ufbx as u;

pub fn cmp_DomValue(cx: &mut Cx, c: &u::DomValue, p: &f::DomValue) {
    cx.push("type_");
    if !cx.same(eq_DomValueType(c.type_, p.value_type)) {
        cx.diff(format_args!("{:?}", c.type_), format_args!("{:?}", p.value_type));
    }
    cx.pop();
    cx.push("value_str");
    cx.str(rt::cstr(&c.value_str), p.value_str.as_bytes());
    cx.pop();
    cx.push("value_blob");
    match c.type_ {
        u::DomValueType::ArrayBlob => {
            let cs = rt::string_array(&c.value_blob);
            let ps: &[Vec<u8>] = match &p.array {
                Some(f::DomArray::Blob(v)) => v,
                _ => &[],
            };
            cx.len(cs.len(), ps.len());
            for i in 0..cs.len().min(ps.len()) {
                cx.idx(i);
                cx.str(cs[i], &ps[i]);
                cx.pop();
            }
        }
        u::DomValueType::ArrayIgnored => {}
        _ => cx.bytes(rt::blob(&c.value_blob), &p.value_blob),
    }
    cx.pop();
    cx.push("value_int");
    cx.int(c.value_int as i128, p.value_int as i128);
    cx.pop();
    cx.push("value_float");
    cx.f64(c.value_float, p.value_float);
    cx.pop();
}

pub fn conv_DomValue(_k: &Kx, c: &u::DomValue) -> f::DomValue {
    let (value_blob, array) = match c.type_ {
        u::DomValueType::ArrayBlob => (Vec::new(), Some(f::DomArray::Blob(rt::string_array(&c.value_blob).iter().map(|s| s.to_vec()).collect()))),
        u::DomValueType::ArrayIgnored => (Vec::new(), None),
        _ => (rt::blob(&c.value_blob).to_vec(), None),
    };
    f::DomValue {
        value_type: conv_enum_DomValueType(c.type_),
        value_str: rt::to_string(rt::cstr(&c.value_str)),
        value_blob,
        value_int: c.value_int,
        value_float: c.value_float,
        array,
    }
}

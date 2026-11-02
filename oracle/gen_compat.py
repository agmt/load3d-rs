#!/usr/bin/env python3
"""Writes src/compat/generated.rs from ufbx-rust's generated.rs and load3d's types, on gen.py's pairing of their fields.

The file is kept in the repository, as the crate's users have neither ufbx-rust's source nor Python; `--check` fails if
it is out of date. A struct whose fields are the port's, name for name and type for type, is the port's own; the others
are ufbx-rust's fields with a converter from the port's value."""

import argparse
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen  # noqa: E402
from gen import P, tstr  # noqa: E402

SEEDS = ['Scene', 'BakedAnim', 'CurvePoint', 'SurfacePoint', 'TopoEdge', 'Progress']
OPTS = ['AllocatorOpts', 'OpenFileOpts', 'OpenMemoryOpts', 'ThreadOpts', 'LoadOpts', 'EvaluateOpts', 'PropOverrideDesc',
        'AnimOpts', 'BakeOpts', 'TessellateCurveOpts', 'TessellateSurfaceOpts', 'SubdivideOpts', 'GeometryCacheOpts',
        'GeometryCacheDataOpts']
# written by hand in src/compat/mod.rs
HANDWRITTEN = {'DomValue'}
# fields beyond ufbx-rust's: (struct, field, type, value)
EXTRA = {
    'Scene': [('bases', 'Bases', 'k.b'), ('owner', 'Weak<SceneData>', 'k.owner.clone()')],
    'DomNode': [('blobs', 'List<Blob>', 'k.dom_blobs(p)')],
    'Prop': [('own', 'Option<Arc<f::Prop>>', 'None')],
    'Props': [('own', 'Option<Arc<OwnedProps>>', 'None')],
}


def snake(name):
    return re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()


class CompatGen(gen.Gen):
    def __init__(self, c, p, text):
        super().__init__(c, p)
        self.text = text
        self.planned = {}
        self.plain_memo = {}
        self.lists = {}  # element type -> the Scene field that lists it

    def run(self):
        for s in SEEDS:
            self.want(s, P(s))
        while self.queue:
            name = self.queue.pop(0)
            self.done.add(name)
            self.planned[name] = self.plan_struct(name)
        if self.missing or self.mismatch or self.missing_structs:
            sys.exit(f'gen_compat.py: unpaired: {self.missing} {self.mismatch} {self.missing_structs}')
        for f, t, _ in self.c.fields('Scene'):
            if t[0] == 'path' and t[1] == 'RefList':
                tgt = t[2][0][1]
                if tgt != 'Element' and self.c.is_elem(tgt):
                    self.lists[tgt] = f

    def enum_reused(self, name):
        pv = self.p.enums.get(name)
        return bool(pv) and pv == [v for v, _ in self.c.enums[name]] and all(i == val for i, (_, val) in enumerate(self.c.enums[name]))

    def same(self, rule, ct, pt):
        k = rule[0]
        if k in ('f64', 'f32', 'bool'):
            return True
        if k == 'int':
            return tstr(ct) == tstr(pt)
        if k == 'enum':
            return self.enum_reused(rule[1])
        if k == 'struct':
            return self.plain(rule[1])
        if k == 'list' and ct[0] == 'array' and pt[0] == 'array':
            return self.same(rule[1], ct[1], pt[1])
        return False

    def plain(self, name):
        if name in self.plain_memo:
            return self.plain_memo[name]
        self.plain_memo[name] = False
        plan = self.planned.get(name)
        ok = False
        if name not in HANDWRITTEN and name not in EXTRA and plan is not None:
            cfields, pt, pf, fields, maps, feat = plan
            if not maps and not feat and {f for f, _, _ in cfields} == set(pf):
                ok = all(kind == 'rule' and f == pfield and pub and self.same(rule, t, pf[pfield][0])
                         for (f, t, kind, rule, pfield), (_, _, pub) in zip(fields, cfields))
        self.plain_memo[name] = ok
        return ok

    # `e` is an expression of a reference to the port's value

    def ptr(self, kind, tgt, i):
        return f'k.element_id({i} as usize)' if kind == 'eid' else f'k.b.{self.lists[tgt]}.wrapping_add({i} as usize)'

    def conv(self, rule, e, ct, pt):
        k = rule[0]
        if k in ('f64', 'f32', 'bool'):
            return f'*{e}'
        if k == 'int':
            return f'*{e} as {tstr(ct)}'
        if k == 'str':
            return f'String::view({e})' if rule[1] == 'S' else f'String::view_bytes({e})'
        if k == 'blob':
            return f'Blob::view({e})' if rule[1] == 'B' else f'Blob::view({e}.as_bytes())'
        if k == 'enum':
            return f'*{e}' if self.enum_reused(rule[1]) else f'from_enum_{rule[1]}(*{e})'
        if k == 'flags':
            return f'{ct[1]}(*{e} as u32)'
        if k == 'struct':
            return f'{e}.clone()' if self.plain(rule[1]) else f'from_{rule[1]}(k, {e})'
        if k == 'list':
            if ct[0] == 'array':
                return f'std::array::from_fn(|i| {self.conv(rule[1], "(&" + e + "[i])", ct[1], self.elem_of(pt))})'
            ce, pe = ct[2][0], self.elem_of(pt)
            if self.same(rule[1], ce, pe):
                return f'List::view(&{e}[..])'
            return f'k.list({e}.iter().map(|x| {self.conv(rule[1], "x", ce, pe)}).collect())'
        if k == 'ids':
            tgt, x = ct[2][0][1], rule[2]
            if self.opt_inner(x) is not None:
                return f'k.reflist({e}.iter().map(|x| Ref::opt(x.map(|i| {self.ptr(rule[1], tgt, "i")}))).collect())'
            return f'k.reflist({e}.iter().map(|&i| Ref::new({self.ptr(rule[1], tgt, "i")})).collect())'
        if k == 'id':
            c_opt = ct[1] == 'Option'
            tgt = (ct[2][0] if c_opt else ct)[2][0][1]
            p_opt = self.opt_inner(pt) is not None
            if p_opt:
                base = f'{e}.map(|i| {self.ptr(rule[1], tgt, "i")})'
                return f'{base}.map(Ref::new)' if c_opt else f'Ref::opt({base})'
            r = f'Ref::new({self.ptr(rule[1], tgt, "*" + e)})'
            return f'Some({r})' if c_opt else r
        if k == 'elements_ref':
            return f'k.reflist({e}.iter().map(|r| Ref::new(k.b.element_at(*r))).collect())'
        if k == 'reflist_deep':
            tgt = rule[1]
            if tgt in self.lists:
                return f'k.arena_reflist(k.b.{self.lists[tgt]}, {e}.len())'
            return f'k.reflist({e}.iter().map(|x| k.boxed(from_{tgt}(k, x))).collect())'
        if k == 'deep':
            tgt, wrap = rule[1], rule[2]
            c_opt = ct[1] == 'Option'
            mk = (lambda x: f'{x}.clone()') if self.plain(tgt) else (lambda x: f'from_{tgt}(k, {x})')
            if wrap in ('arc', 'opt_arc'):
                one = lambda x: f'k.cached(std::sync::Arc::as_ptr({x}), || {mk("&**" + x)})'
            else:
                one = lambda x: f'k.boxed({mk(x)})'
            if wrap in ('plain', 'box', 'arc'):
                val = one(e if wrap != 'box' else f'(&**{e})')
                return f'Some({val})' if c_opt else val
            inner = f'{e}.as_ref().map(|x| {one("x")})' if wrap != 'opt_box' else f'{e}.as_deref().map(|x| {one("x")})'
            return inner if c_opt else f'Ref::from_option({inner})'
        if k == 'dom_ref':
            return f'k.dom_ref({e})'
        if k == 'vec4_arr':
            return f'Vec4 {{ x: {e}[0], y: {e}[1], z: {e}[2], w: {e}[3] }}'
        if k == 'prop_name':
            c_opt = ct[1] == 'Option'
            if rule[1]:
                inner = f'{e}.as_ref().map(|s| k.prop_named(s))'
                return inner if c_opt else f'Ref::from_option({inner})'
            return f'Some(k.prop_named({e}))' if c_opt else f'k.prop_named({e})'
        sys.exit(f'gen_compat.py: no conversion for {rule}')

    def elem_of(self, pt):
        return pt[1] if pt[0] == 'array' else pt[2][0]

    def enums(self):
        o = []
        reused = []
        for name, variants in sorted(self.c.enums.items()):
            pv = self.p.enums.get(name)
            if self.enum_reused(name):
                reused.append(name)
                continue
            o.append('#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]')
            o.append('#[repr(u32)]')
            o.append(f'pub enum {name} {{')
            o += [f'    {v} = {val},' for v, val in variants]
            o.append('}')
            o.append(f'impl Default for {name} {{ fn default() -> Self {{ Self::{variants[0][0]} }} }}')
            if not pv:
                sys.exit(f'gen_compat.py: no port enum {name}')
            keys = {}
            exact = {v: v for v, _ in variants if v in pv}
            for v in pv:
                if v not in exact.values():
                    keys.setdefault(gen.norm_variant(v), []).append(v)
            pairs = {v: exact.get(v) or (keys.get(gen.norm_variant(v)) or [None])[0] for v, _ in variants}
            if not all(pairs.values()):
                sys.exit(f'gen_compat.py: {name}: {pairs}')
            o.append(f'pub(crate) fn from_enum_{name}(p: f::{name}) -> {name} {{')
            o.append('    #[allow(unreachable_patterns)]')
            o.append('    match p {')
            o += [f'        f::{name}::{pairs[v]} => {name}::{v},' for v, _ in variants]
            o.append(f'        _ => {name}::default(),')
            o.append('    }')
            o.append('}')
            o.append(f'pub(crate) fn to_enum_{name}(c: {name}) -> f::{name} {{')
            o.append('    match c {')
            o += [f'        {name}::{v} => f::{name}::{pairs[v]},' for v, _ in variants]
            o.append('    }')
            o.append('}')
            o.append('')
        head = 'pub use crate::{' + ', '.join(reused) + '};'
        return [head, ''] + o

    def flags(self):
        o = []
        for name in sorted(self.c.flags):
            m = re.search(rf'impl {name} {{\n((?:    pub const [^\n]*\n)+)}}', self.text)
            o.append('#[derive(Clone, Copy, PartialEq, Eq, Hash)]')
            o.append('#[repr(transparent)]')
            o.append(f'pub struct {name}(pub(crate) u32);')
            o.append(f'impl {name} {{')
            o += m[1].rstrip('\n').split('\n')
            names = re.findall(r'pub const (\w+):', m[1])
            o.append('}')
            pairs = ', '.join(f'("{n}", {name}::{n}.0)' for n in names)
            o.append(f'flags_impl!({name}, [{pairs}]);')
            o.append('')
        return o

    def structs(self):
        o = []
        plain = []
        for name in sorted(self.planned):
            if name in HANDWRITTEN:
                continue
            if self.plain(name):
                plain.append(name)
                continue
            cfields, pt, pf, fields, maps, feat = self.planned[name]
            texts = {f: t for f, t, _ in self.c.structs[name]}
            pty = self.rust_p_type(pt)
            o.append(f'pub struct {name} {{')
            for (f, _, pub) in cfields:
                o.append(f'    {"pub" if pub else "pub(crate)"} {f}: {texts[f]},')
            for f, t, _ in EXTRA.get(name, []):
                o.append(f'    pub(crate) {f}: {t},')
            o.append(f'    pub(crate) port: Port<{pty}>,')
            o.append('}')
            o.append(f'pub(crate) fn from_{name}(k: &Kx, p: &{pty}) -> {name} {{')
            o.append('    let _ = &k;')
            if name == 'Texture':
                o.append('    k.set_current(&p.element);')
            o.append(f'    {name} {{')
            for f, t, kind, rule, pfield in fields:
                if kind == 'skip':
                    o.append(f'        {f}: ' + ('k.scene_ref(),' if (name, f) == ('Element', 'scene') else 'Default::default(),'))
                    continue
                if (name, f) == ('Scene', 'dom_root'):
                    o.append(f'        {f}: k.dom_root(),')
                    continue
                if rule[0] == 'channel_index':
                    o.append(f'        {f}: k.channel(&p.external_cache, p.external_channel),')
                    continue
                e = pf[pfield][1].format(b='p')
                if maps:
                    e = f'{e}'
                o.append(f'        {f}: {self.conv(rule, e, t, pf[pfield][0])},')
            for f, _, v in EXTRA.get(name, []):
                o.append(f'        {f}: {v},')
            o.append('        port: Port::new(p),')
            o.append('    }')
            o.append('}')
            o.append('')
        return ['pub use crate::{' + ', '.join(plain) + '};', ''] + o

    def opts(self):
        o = []
        unmapped = []
        for name in OPTS:
            m = re.search(rf"#\[derive\(Default\)\]\n#\[repr\(C\)\]\npub struct {name}(<'a>)? {{\n(.*?)\n}}", self.text, re.S)
            if not m:
                m = re.search(rf"#\[derive\(Default\)\]\npub struct {name}(<'a>)? {{\n(.*?)\n}}", self.text, re.S)
            if not m:
                sys.exit(f'gen_compat.py: no options struct {name}')
            lt = m[1] or ''
            o.append('#[derive(Default)]')
            o.append(f'pub struct {name}{lt} {{')
            o.append(m[2])
            o.append('}')
            port = self.p.structs.get(name)
            if not port:
                o.append('')
                continue
            pfields = {f: gen.parse_type(t) for f, t, _ in port[1]}
            fields = re.findall(r'^\s*pub (\w+): (.+),$', m[2], re.M)
            o.append(f'impl{lt} {name}{lt} {{')
            o.append(f'    pub(crate) fn to_port(&mut self) -> f::{name} {{')
            o.append(f'        let mut o = f::{name}::default();')
            for f, t in fields:
                if f not in pfields:
                    unmapped.append(f'{name}.{f}')
                    continue
                o.append(f'        o.{f} = {self.to_port(f, t)};')
            o.append('        o')
            o.append('    }')
            o.append('}')
            o.append('')
        return o, unmapped

    def to_port(self, f, t):
        t = t.replace('<>', '')
        if t.startswith('Unsafe<'):
            return f'self.{f}.0'
        if re.match(r"(StringOpt|BlobOpt|ProgressCb|OpenFileCb)<", t):
            return f'self.{f}.to_port()'
        m = re.match(r"ListOpt<'a, (.+)>$", t)
        if m:
            if m[1].startswith('PropOverrideDesc'):
                return f'self.{f}.as_slice().iter().map(prop_override_to_port).collect()'
            return f'self.{f}.as_slice().to_vec()'
        if t == 'AllocatorOpts':
            return f'self.{f}.to_port()'
        if t in self.c.enums and not self.enum_reused(t):
            return f'to_enum_{t}(self.{f})'
        return f'self.{f}.clone()' if t in self.c.structs else f'self.{f}'

    def arenas(self):
        items = list(self.lists.items())
        o = ['pub(crate) struct Arenas {']
        o += [f'    pub(crate) {f}: Vec<{t}>,' for t, f in items]
        o += ['}', '', '#[derive(Clone, Copy)]', 'pub(crate) struct Bases {']
        o += [f'    pub(crate) {f}: *const {t},' for t, f in items]
        o += ['}', '', 'impl Bases {', '    pub(crate) fn none() -> Bases {', '        Bases {']
        o += [f'            {f}: std::ptr::null(),' for _, f in items]
        o += ['        }', '    }', '    pub(crate) fn element_at(&self, r: f::ElementRef) -> *const Element {',
              '        match r.element_type {']
        for t, f in items:
            o.append(f'            ElementType::{t} => field(self.{f}, r.index, std::mem::offset_of!({t}, element)),')
        o += ['            #[allow(unreachable_patterns)]', '            _ => std::ptr::null(),', '        }', '    }', '}', '']
        o += ['pub(crate) fn make_arenas(p: &f::Scene) -> (Arenas, Bases) {', '    let a = Arenas {']
        o += [f'        {f}: Vec::with_capacity(p.{f}.len()),' for _, f in items]
        o += ['    };', '    let b = Bases {']
        o += [f'        {f}: a.{f}.as_ptr(),' for _, f in items]
        o += ['    };', '    (a, b)', '}', '', 'pub(crate) fn fill_arenas(k: &Kx, p: &f::Scene, a: &mut Arenas) {']
        o += [f'    a.{f}.extend(p.{f}.iter().map(|x| from_{t}(k, x)));' for t, f in items]
        o += ['}', '']
        for t, f in items:
            o += [f"pub fn as_{snake(t)}<'a>(element: &'a Element) -> Option<&'a {t}> {{",
                  f'    if element.type_ == ElementType::{t} {{ element.scene.{f}.get(element.typed_id as usize).map(|r| &**r) }} else {{ None }}',
                  '}', '']
        o += ["pub enum ElementData<'a> {"]
        o += [f"    {t}(&'a {t})," for t, _ in items]
        o += ['}', '', 'impl Element {', "    pub fn as_data(&self) -> ElementData<'_> {", '        let s = &self.scene;',
              '        let i = self.typed_id as usize;', '        match self.type_ {']
        o += [f'            ElementType::{t} => ElementData::{t}(&s.{f}[i]),' for t, f in items]
        o += ['            #[allow(unreachable_patterns)]', '            _ => panic!("no such element"),', '        }', '    }', '}', '']
        return o

    def api_check(self):
        """a test that holds every public function, method and field of ufbx-rust to its signature here"""
        o = ['// Written by oracle/gen_compat.py from ufbx-rust\'s generated.rs; do not edit.',
             '#![allow(dead_code, unused_imports, unused_variables, non_snake_case, clippy::all)]',
             'use super::*;', 'use std::ffi::c_void;', '']
        for name in sorted(self.c.structs):
            if name.startswith('Raw') or name in self.c.flags or '<' in name:
                continue
            if name in ('VoidList', 'VertexAttrib', 'ThreadPoolInfo', 'InflateInput', 'InflateRetain', 'SceneRoot', 'MeshRoot',
                        'LineCurveRoot', 'GeometryCacheRoot', 'AnimRoot', 'BakedAnimRoot', 'Error', 'ErrorFrame', 'Panic',
                        'OpenFileInfo') or name in OPTS:
                continue
            rows = [f'    let _: &{t.replace("<>", "")} = &x.{f};' for f, t, pub in self.c.structs[name] if pub]
            o += [f'fn fields_{name}(x: &{name}) {{'] + rows + ['}', '']
        n = 0
        names = []
        for m in re.finditer(r"^pub fn (\w+)(<[^>(]*>)?\((.*)\)( -> ([^{]+?))? *\{", self.text, re.M):
            fn, gens, args, _, ret = m.groups()
            if fn.endswith('_raw'):
                continue
            n += 1
            lts = re.findall(r"'[a-z]+", gens or '')
            pars = ', '.join(a.split(': ', 1)[1] for a in split_args(args)) if args.strip() else ''
            # a fn pointer type cannot say `where 'a: 'b`: one lifetime stands for both
            if ret and ' where ' in f' {ret}':
                ret = ret[:ret.index('where')].strip()
                pars, ret = pars.replace("'b", "'a"), ret.replace("'b", "'a")
                lts = ["'a"]
            # a lifetime no argument names is 'static's to take
            for lt in [x for x in lts if x not in pars]:
                ret = ret.replace(lt, "'static")
            lts = [x for x in lts if x in pars]
            hr = f"for<{', '.join(lts)}> " if lts else ''
            names.append(fn)
            r = f' -> {ret.strip()}' if ret else ''
            # an argument's elided lifetime is the pointer type's own to fill in
            o.append(f'const _: {hr}fn({pars}){r} = {fn};')
        o.append('')
        for m in re.finditer(r"^impl (\w+) \{\n(.*?)^\}", self.text, re.M | re.S):
            owner, body = m.groups()
            if owner.startswith('Raw') or owner in self.c.flags:
                continue
            for b in re.finditer(r"^    pub fn (\w+)(<[^>(]*>)?\((.*)\)( -> ([^{]+?))? *\{", body, re.M):
                fn, gens, args, _, ret = b.groups()
                n += 1
                lts = re.findall(r"'[a-z]+", gens or '')
                parts = split_args(args) if args.strip() else []
                pars = []
                for a in parts:
                    if a == '&self':
                        pars.append(f'&{owner}')
                    elif a == '&mut self':
                        pars.append(f'&mut {owner}')
                    elif a.endswith(' self') and a.startswith('&'):
                        pars.append(a[:-len('self')] + owner)
                    elif a == 'self':
                        pars.append(owner)
                    else:
                        pars.append(a.split(': ', 1)[1])
                pars = ', '.join(pars)
                hr = f"for<{', '.join(lts)}> " if lts else ''
                r = f' -> {ret.strip()}' if ret else ''
                o.append(f'const _: {hr}fn({pars}){r} = {owner}::{fn};')
        o.append('')
        return o, n


class CheckGen(CompatGen):
    """oracle's comparison of ufbx's C scene with the same scene through the layer, field by field"""

    def chk(self, rule, a, b, ct, ind):
        k = rule[0]
        s = ' ' * ind
        if k == 'f64':
            return [f'{s}cx.f64(*{a} as f64, *{b} as f64);']
        if k == 'f32':
            return [f'{s}cx.f32(*{a}, *{b});']
        if k == 'int':
            return [f'{s}cx.int(*{a} as i128, *{b} as i128);']
        if k == 'bool':
            return [f'{s}cx.bool(*{a}, *{b});']
        if k == 'str':
            return [f'{s}cx.str(rt::cstr({a}), {b}.as_bytes());']
        if k == 'blob':
            return [f'{s}cx.bytes(rt::blob({a}), &{b}[..]);']
        if k == 'enum':
            return [f'{s}same_enum(cx, {a}, {b});']
        if k == 'flags':
            return [f'{s}cx.flags(rt::flag_bits(*{a}), {b}.bits());']
        if k == 'struct':
            return [f'{s}chk_{rule[1]}(cx, {a}, {b});']
        if k == 'list':
            n = self.fresh()
            ce = ct[1] if ct[0] == 'array' else ct[2][0]
            view = f'&{b}[..]' if ct[0] == 'array' else f'{b}.as_ref()'
            lines = [f'{s}{{', f'{s}    let (ca{n}, pa{n}): (_, &[_]) = (rt::slice_of({a}), {view});',
                     f'{s}    cx.len(ca{n}.len(), pa{n}.len());', f'{s}    for i{n} in 0..ca{n}.len().min(pa{n}.len()) {{',
                     f'{s}        cx.idx(i{n});']
            lines += self.chk(rule[1], f'(&ca{n}[i{n}])', f'(&pa{n}[i{n}])', ce, ind + 8)
            return lines + [f'{s}        cx.pop();', f'{s}    }}', f'{s}}}']
        if k == 'ids':
            n = self.fresh()
            read, c_read = ('x.element_id', 'rt::c_eid') if rule[1] == 'eid' else ('x.element.typed_id', 'rt::c_typed')
            return [f'{s}{{', f'{s}    let (ca{n}, pa{n}): (_, &[_]) = (rt::reflist({a}), {b}.as_ref());',
                    f'{s}    cx.len(ca{n}.len(), pa{n}.len());', f'{s}    for i{n} in 0..ca{n}.len().min(pa{n}.len()) {{',
                    f'{s}        cx.idx(i{n});',
                    f'{s}        cx.id({c_read}(ca{n}[i{n}] as *const u8), CRef::get(&pa{n}[i{n}]).map(|x| {read} as u64));',
                    f'{s}        cx.pop();', f'{s}    }}', f'{s}}}']
        if k == 'id':
            read, c_read = ('x.element_id', 'rt::c_eid') if rule[1] == 'eid' else ('x.element.typed_id', 'rt::c_typed')
            return [f'{s}cx.id({c_read}(rt::raw_ptr({a})), CRef::get({b}).map(|x| {read} as u64));']
        if k == 'elements_ref':
            n = self.fresh()
            return [f'{s}{{', f'{s}    let (ca{n}, pa{n}): (_, &[_]) = (rt::reflist({a}), {b}.as_ref());',
                    f'{s}    cx.len(ca{n}.len(), pa{n}.len());', f'{s}    for i{n} in 0..ca{n}.len().min(pa{n}.len()) {{',
                    f'{s}        cx.idx(i{n});', f'{s}        let e = unsafe {{ rt::elem(ca{n}[i{n}] as *const u8) }};',
                    f'{s}        let x: &x::Element = &pa{n}[i{n}];',
                    f'{s}        same_enum(cx, &e.type_, &x.type_);', f'{s}        cx.int(e.typed_id as i128, x.typed_id as i128);',
                    f'{s}        cx.pop();', f'{s}    }}', f'{s}}}']
        if k == 'reflist_deep':
            n = self.fresh()
            return [f'{s}{{', f'{s}    let (ca{n}, pa{n}): (_, &[_]) = (rt::reflist({a}), {b}.as_ref());',
                    f'{s}    cx.len(ca{n}.len(), pa{n}.len());', f'{s}    for i{n} in 0..ca{n}.len().min(pa{n}.len()) {{',
                    f'{s}        cx.idx(i{n});',
                    f'{s}        chk_{rule[1]}(cx, unsafe {{ &*(ca{n}[i{n}] as *const u::{rule[1]}) }}, &pa{n}[i{n}]);',
                    f'{s}        cx.pop();', f'{s}    }}', f'{s}}}']
        if k == 'deep':
            n = self.fresh()
            return [f'{s}{{', f'{s}    let (c{n}, b{n}) = (rt::raw_ptr({a}) as *const u::{rule[1]}, CRef::get({b}));',
                    f'{s}    if !cx.same(c{n}.is_null() == b{n}.is_none()) {{ cx.presence(!c{n}.is_null(), b{n}.is_some()); }}',
                    f'{s}    if let (false, Some(b{n})) = (c{n}.is_null(), b{n}) {{ chk_{rule[1]}(cx, unsafe {{ &*c{n} }}, b{n}); }}', f'{s}}}']
        if k == 'dom_ref':
            n = self.fresh()
            return [f'{s}{{', f'{s}    let (c{n}, b{n}) = (rt::raw_ptr({a}) as *const u::DomNode, CRef::get({b}));',
                    f'{s}    if !cx.same(c{n}.is_null() == b{n}.is_none()) {{ cx.presence(!c{n}.is_null(), b{n}.is_some()); }}',
                    f'{s}    if let (false, Some(b{n})) = (c{n}.is_null(), b{n}) {{ cx.str(rt::cstr(unsafe {{ &(*c{n}).name }}), b{n}.name.as_bytes()); }}', f'{s}}}']
        if k == 'vec4_arr':
            return [f'{s}chk_Vec4(cx, {a}, {b});']
        if k == 'prop_name':
            return [f'{s}cx.opt_str(rt::prop_name(rt::raw_ptr({a})), CRef::get({b}).map(|x| x.name.as_bytes()));']
        sys.exit(f'gen_compat.py: no comparison for {rule}')

    def file(self):
        o = ['// Written by gen_compat.py; do not edit.']
        for name in sorted(self.planned):
            plan = self.planned[name]
            if plan is None:
                continue
            cfields, pt, pf, fields, maps, feat = plan
            o.append(f'pub fn chk_{name}(cx: &mut Cx, c: &u::{name}, b: &x::{name}) {{')
            for (f, t, kind, rule, pfield), (_, _, pub) in zip(fields, cfields):
                # C's allocator statistics, as gen.py's SPECIAL leaves out of a scene's metadata
                if kind == 'skip' or not pub or (name == 'BakedAnimMetadata' and f.endswith(('_memory_used', '_allocs'))):
                    continue
                if rule[0] == 'channel_index':
                    o.append('    cx.opt_str(c.external_channel.as_ref().map(|x| rt::cstr(&x.name)), CRef::get(&b.external_channel).map(|x| x.name.as_bytes()));')
                    continue
                o.append(f'    cx.push("{f}");')
                o += self.chk(rule, f'(&c.{f})', f'(&b.{f})', t, 4)
                o.append('    cx.pop();')
            if name == 'DomNode':
                o.append('    dom_arrays(cx, c, b);')
            o.append('}')
            o.append('')
        return o


def split_args(s):
    out, depth, cur = [], 0, ''
    for ch in s:
        if ch in '<([':
            depth += 1
        elif ch in '>)]':
            depth -= 1
        if ch == ',' and depth == 0:
            out.append(cur.strip())
            cur = ''
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def formatted(body, root):
    # as `cargo fmt` lays out the crate, so that test.sh's `cargo fmt --check` holds for these files too
    run = subprocess.run(['rustfmt', '--edition', '2024', '--config-path', os.path.join(root, 'rustfmt.toml'), '--emit', 'stdout'],
                         input=body, capture_output=True, text=True, check=True)
    return run.stdout


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    ap = argparse.ArgumentParser()
    ap.add_argument('--src', default=os.path.join(here, '../src'))
    ap.add_argument('--cargo', default=os.environ.get('CARGO', 'cargo'))
    ap.add_argument('--manifest', default=os.path.join(here, 'Cargo.toml'))
    ap.add_argument('--check', action='store_true', help='fail if the file in the repository differs')
    ap.add_argument('--report', action='store_true')
    ap.add_argument('--cmp-out', help='write the oracle\'s comparison of the layer with C there, and nothing else')
    args = ap.parse_args()
    text = open(gen.find_ufbx(args.cargo, args.manifest)).read()
    c = gen.CSide(text)
    p = gen.PortSide([open(os.path.join(args.src, f)).read() for f in ('types.rs', 'math.rs', 'error.rs')])
    if args.cmp_out:
        g = CheckGen(c, p, text)
        g.run()
        with open(args.cmp_out, 'w') as fh:
            fh.write('\n'.join(g.file()) + '\n')
        return
    g = CompatGen(c, p, text)
    g.run()
    opts, unmapped = g.opts()
    head = ['// Written by oracle/gen_compat.py from ufbx-rust\'s generated.rs and load3d\'s types.rs; do not edit.',
            '#![allow(non_snake_case, dead_code, unused_parens, clippy::all)]',
            'use super::*;', 'use crate as f;', '']
    out = head + g.enums() + g.flags() + g.structs() + opts + g.arenas()
    api, n = g.api_check()
    root = os.path.dirname(os.path.abspath(args.src))
    files = {os.path.join(args.src, 'compat/generated.rs'): formatted('\n'.join(out) + '\n', root),
             os.path.join(args.src, 'compat/api_check.rs'): formatted('\n'.join(api) + '\n', root)}
    if args.report:
        print(f'{len(g.planned)} structs, of which the port\'s own: {sorted(x for x in g.planned if g.plain(x))}')
        print(f'enums generated: {sorted(x for x in c.enums if not g.enum_reused(x))}')
        print(f'option fields with no port field: {unmapped}')
        print(f'functions checked by signature: {n}')
    for path, body in files.items():
        if args.check:
            if open(path).read() != body:
                sys.exit(f'gen_compat.py: {path} is out of date: run oracle/gen_compat.py')
        else:
            with open(path, 'w') as fh:
                fh.write(body)


if __name__ == '__main__':
    main()

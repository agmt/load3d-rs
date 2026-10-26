#!/usr/bin/env python3
"""Writes the oracle's comparator and converter from ufbx-rust's generated.rs and load3d's types.rs.

A ufbx field that pairs with no port field, or whose types do not fit, is compiled into the MISSING / MISMATCH lists
the oracle prints, so none is skipped without a line saying so."""

import argparse
import json
import os
import re
import subprocess
import sys

INT = {'u8', 'u16', 'u32', 'u64', 'usize', 'i8', 'i16', 'i32', 'i64', 'isize'}
FLOAT = {'f32', 'f64', 'Real'}
PRIM = INT | FLOAT | {'bool'}

# (ufbx struct, ufbx field) -> (kind, port field, why). Kinds: 'skip' (not compared), or a handler of emit_rule.
SPECIAL = {
    ('Element', 'scene'): ('skip', None, 'back pointer to the owning ufbx_scene; the port passes the scene alongside'),
    ('Element', 'dom_node'): ('dom_ref', 'dom_node', 'pointer into the retained DOM vs the path of child indices from Scene::dom_root'),
    ('Prop', 'value_vec4'): ('vec4_arr', 'value_real_arr', 'the union value_real/value_vec2/value_vec3/value_vec4 held as one [Real; 4]'),
    ('CacheDeformer', 'external_channel'): ('channel_index', 'external_channel', 'pointer into external_cache.channels vs its index there'),
    ('Metadata', 'result_memory_used'): ('skip', None, "the C allocator's statistics; allocation is not ported"),
    ('Metadata', 'temp_memory_used'): ('skip', None, "the C allocator's statistics; allocation is not ported"),
    ('Metadata', 'result_allocs'): ('skip', None, "the C allocator's statistics; allocation is not ported"),
    ('Metadata', 'temp_allocs'): ('skip', None, "the C allocator's statistics; allocation is not ported"),
    ('Metadata', 'element_buffer_size'): ('skip', None, 'bytes of the C element structs (sizeof-dependent), no meaning for Rust structs'),
    ('SubdivisionResult', 'result_memory_used'): ('skip', None, "the C allocator's statistics"),
    ('SubdivisionResult', 'temp_memory_used'): ('skip', None, "the C allocator's statistics"),
    ('SubdivisionResult', 'result_allocs'): ('skip', None, "the C allocator's statistics"),
    ('SubdivisionResult', 'temp_allocs'): ('skip', None, "the C allocator's statistics"),
}

# Structs compared by hand in src/scene/special.rs: the ufbx fields and the port fields each one covers, checked here so a
# field added on either side stops the generator.
HANDWRITTEN = {
    'DomValue': (['type_', 'value_str', 'value_blob', 'value_int', 'value_float'],
                 ['value_type', 'value_str', 'value_blob', 'value_int', 'value_float', 'array'],
                 'value_blob of an ARRAY_BLOB value holds ufbx_string structs in C: compared as the list of strings '
                 'against DomArray::Blob; ARRAY_IGNORED has no data in C: its blob is not compared'),
}

# ufbx structs whose named fields the port holds as an array indexed by an enum
FEATURE_ARRAYS = {'MaterialFeatures': ('features', 'MaterialFeature')}

# enum variants whose names cannot be paired by the normalisation in pair_variants: (enum, ufbx variant) -> port variant
VARIANT_ALIASES = {}

EXTRA_ENUMS = ['ErrorType', 'DomValueType']


def tokenize(s):
    return re.findall(r"'[A-Za-z_]+|[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*|\d+|[<>\[\];,()&]", s)


def parse_type(s):
    toks = tokenize(s)
    t, i = _pt(toks, 0)
    if i != len(toks):
        raise ValueError(f'cannot parse type {s!r}')
    return t


def _pt(toks, i):
    if toks[i] == '[':
        e, i = _pt(toks, i + 1)
        assert toks[i] == ';', toks
        n = toks[i + 1]
        assert toks[i + 2] == ']', toks
        return ('array', e, n), i + 3
    if toks[i] in ('&', '(', ')'):
        raise ValueError(f'unsupported type tokens {toks}')
    name = toks[i].split('::')[-1]
    i += 1
    args = []
    if i < len(toks) and toks[i] == '<':
        i += 1
        while toks[i] != '>':
            if toks[i].startswith("'"):
                i += 1
            else:
                a, i = _pt(toks, i)
                args.append(a)
            if toks[i] == ',':
                i += 1
        i += 1
    return ('path', name, tuple(args)), i


def tstr(t):
    if t[0] == 'array':
        return f'[{tstr(t[1])}; {t[2]}]'
    if t[2]:
        return f"{t[1]}<{', '.join(tstr(a) for a in t[2])}>"
    return t[1]


def P(name, *args):
    return ('path', name, tuple(args))


def subst(t, m):
    if t[0] == 'array':
        return ('array', subst(t[1], m), t[2])
    if not t[2] and t[1] in m:
        return m[t[1]]
    return ('path', t[1], tuple(subst(a, m) for a in t[2]))


class CSide:
    """ufbx-rust's generated.rs"""

    def __init__(self, text):
        self.structs = {}
        self.enums = {}
        self.flags = set()
        lines = text.split('\n')
        i = 0
        while i < len(lines):
            line = lines[i]
            m = re.match(r'^pub struct (\w+)\(u32\);', line)
            if m:
                self.flags.add(m[1])
            m = re.match(r'^pub struct (\w+)(<[^{]*>)?\s*\{\s*$', line)
            if m:
                fields = []
                i += 1
                while lines[i] != '}':
                    fl = lines[i].strip()
                    if fl and not fl.startswith('//') and not fl.startswith('#'):
                        fm = re.match(r'^(pub\s+)?(\w+)\s*:\s*(.+),$', fl)
                        if not fm:
                            sys.exit(f'gen.py: generated.rs: cannot read the field line {lines[i]!r} of {m[1]}')
                        fields.append((fm[2], fm[3], bool(fm[1])))
                    i += 1
                self.structs[m[1]] = fields
            m = re.match(r'^pub enum (\w+)\s*\{\s*$', line)
            if m:
                variants = []
                i += 1
                while lines[i] != '}':
                    vm = re.match(r'^\s+(\w+)\s*=\s*(0x[0-9a-fA-F]+|-?\d+),', lines[i])
                    if not vm:
                        sys.exit(f'gen.py: generated.rs: cannot read the variant line {lines[i]!r} of {m[1]}')
                    variants.append((vm[1], int(vm[2], 0)))
                    i += 1
                self.enums[m[1]] = variants
            i += 1

    def fields(self, name):
        return [(f, parse_type(t), pub) for f, t, pub in self.structs[name]]

    def is_elem(self, name):
        if name == 'Element':
            return True
        f = self.structs.get(name)
        return bool(f) and f[0][0] == 'element' and f[0][1] == 'Element'


class PortSide:
    """load3d's types.rs, math.rs and error.rs"""

    def __init__(self, texts):
        self.structs = {}   # name -> (generic params, [(field, type text, public)])
        self.enums = {}     # name -> [variants] (None for an enum carrying data)
        self.aliases = {}   # name -> type text
        self.maps = {}      # maps struct -> (enum, [(variant, accessor)])
        for text in texts:
            self._parse(text)

    def _parse(self, text):
        lines = text.split('\n')
        i = 0
        while i < len(lines):
            line = lines[i]
            m = re.match(r'^pub struct (\w+)(?:<([^>]*)>)?\s*\{\s*$', line)
            if m:
                gens = [g.strip() for g in (m[2] or '').split(',') if g.strip() and not g.strip().startswith("'")]
                fields = []
                i += 1
                while lines[i] != '}':
                    fl = lines[i].strip()
                    if fl and not fl.startswith('//') and not fl.startswith('#'):
                        fm = re.match(r'^(pub(?:\([^)]*\))?\s+)?(\w+)\s*:\s*(.+),$', fl)
                        if not fm:
                            sys.exit(f'gen.py: types.rs: cannot read the field line {lines[i]!r} of {m[1]}')
                        fields.append((fm[2], fm[3], fm[1] is not None and fm[1].strip() == 'pub'))
                    i += 1
                self.structs[m[1]] = (gens, fields)
            m = re.match(r'^pub enum (\w+)\s*\{\s*$', line)
            if m:
                variants = []
                data = False
                i += 1
                while lines[i] != '}':
                    vl = lines[i].strip()
                    if vl and not vl.startswith('//') and not vl.startswith('#'):
                        vm = re.match(r'^(\w+)(\(.*\))?(\s*=\s*\w+)?,?$', vl)
                        if not vm:
                            sys.exit(f'gen.py: types.rs: cannot read the variant line {lines[i]!r} of {m[1]}')
                        if vm[2]:
                            data = True
                        variants.append(vm[1])
                    i += 1
                self.enums[m[1]] = None if data else variants
            m = re.match(r'^pub type (\w+)\s*=\s*(.+);', line)
            if m and 'dyn ' not in m[2]:
                self.aliases[m[1]] = m[2]
            i += 1
        for m in re.finditer(r'map_enum!\(\s*(?:///[^\n]*\n\s*)*(\w+),\s*(\w+),\s*\[(.*?)\]\);', text, re.S):
            pairs = re.findall(r'(\w+)\s*=>\s*(\w+)', m[3])
            self.enums[m[1]] = [v for v, _ in pairs]
            self.maps[m[2]] = (m[1], pairs)

    def resolve(self, t):
        """aliases followed, generic structs kept as written"""
        seen = 0
        while t[0] == 'path' and not t[2] and t[1] in self.aliases and seen < 8:
            t = parse_type(self.aliases[t[1]])
            seen += 1
        if t[0] == 'path' and t[1] == 'Real':
            return P('f64')
        if t[0] == 'array':
            return ('array', self.resolve(t[1]), t[2])
        return ('path', t[1], tuple(self.resolve(a) for a in t[2]))

    def struct_fields(self, t):
        """the fields of a port struct type, its generic parameters substituted"""
        t = self.resolve(t)
        name = t[1]
        if name not in self.structs:
            return None
        gens, fields = self.structs[name]
        m = dict(zip(gens, t[2]))
        return [(f, self.resolve(subst(parse_type(ty), m)), pub) for f, ty, pub in fields]


def norm_variant(s):
    s = s.lower()
    if re.match(r'^e\d', s):
        s = s[1:]
    return ''.join(sorted(ch for ch in s if ch.isalnum()))


class Gen:
    def __init__(self, c, p):
        self.c = c
        self.p = p
        self.out = []
        self.done = set()
        self.queue = []
        self.port_type_of = {}   # ufbx struct -> port type it pairs with
        self.enums_used = set(EXTRA_ENUMS)
        self.missing = []        # (struct, field, ufbx type)
        self.mismatch = []       # (struct, field, ufbx type, port type)
        self.skipped = []        # (struct, field, why)
        self.mappings = []       # (struct.field, port field, meaning)
        self.port_only = []      # (struct, port field)
        self.enum_notes = []     # text
        self.repr_notes = []     # (struct.field, text): widths and nullability that differ but compare
        self.missing_structs = []
        self.uid = 0

    def fresh(self):
        self.uid += 1
        return self.uid

    def is_int(self, t):
        return t[0] == 'path' and t[1] in INT and not t[2]

    def opt_inner(self, t):
        if t[0] == 'path' and t[1] == 'Option' and len(t[2]) == 1:
            return t[2][0]
        return None

    def classify(self, ct, pt):
        """how a ufbx value of type ct is compared with a port value of type pt (aliases resolved)"""
        c = self.c
        if ct[0] == 'array':
            if pt[0] == 'array' or (pt[1] == 'Vec' and pt[2]):
                sub = self.classify(ct[1], pt[1] if pt[0] == 'array' else pt[2][0])
                return ('list', sub) if sub[0] != 'mismatch' else sub
            return ('mismatch',)
        name, args = ct[1], ct[2]
        pn = pt[1] if pt[0] == 'path' else None
        if name in PRIM:
            cf = 'f64' if name == 'Real' else name
            if cf in FLOAT and pn in FLOAT:
                return ('f32',) if cf == 'f32' and pn == 'f32' else ('f64',) if cf != 'f32' and pn != 'f32' else ('mismatch',)
            if cf in INT and pn in INT:
                return ('int', pn)
            if cf == 'bool' and pn == 'bool':
                return ('bool',)
            return ('mismatch',)
        if name == 'String':
            if pn == 'String':
                return ('str', 'S')
            if pn == 'Vec' and pt[2] == (P('u8'),):
                return ('str', 'B')
            return ('mismatch',)
        if name == 'Blob':
            if pn == 'Vec' and pt[2] == (P('u8'),):
                return ('blob', 'B')
            if pn == 'String':
                return ('blob', 'S')
            return ('mismatch',)
        if name in c.enums:
            if pn == name and self.p.enums.get(name):
                self.enums_used.add(name)
                return ('enum', name)
            return ('mismatch',)
        if name in c.flags:
            return ('flags',) if pn in INT else ('mismatch',)
        if name == 'List':
            if pn == 'Vec' or pt[0] == 'array':
                sub = self.classify(args[0], pt[2][0] if pn == 'Vec' else pt[1])
                return ('list', sub) if sub[0] != 'mismatch' else sub
            return ('mismatch',)
        if name == 'RefList':
            tgt = args[0][1]
            if pn != 'Vec':
                return ('mismatch',)
            x = pt[2][0]
            xo = self.opt_inner(x)
            if tgt == 'Element' and x[1] == 'ElementRef':
                self.enums_used.add('ElementType')
                return ('elements_ref',)
            if c.is_elem(tgt) and (self.is_int(x) or (xo and self.is_int(xo))):
                return ('ids', 'eid' if tgt == 'Element' else 'typed', x)
            if self.same_struct(tgt, x):
                self.want(tgt, x)
                return ('reflist_deep', tgt)
            return ('mismatch',)
        if name in ('Ref', 'Option'):
            c_opt = name == 'Option'
            inner = args[0]
            if c_opt:
                if inner[1] != 'Ref':
                    return ('mismatch',)
                inner = inner[2][0]
            tgt = inner[1]
            po = self.opt_inner(pt)
            if c.is_elem(tgt):
                if self.is_int(pt) or (po and self.is_int(po)):
                    return ('id', 'eid' if tgt == 'Element' else 'typed', pt)
                return ('mismatch',)
            if tgt == 'Prop':
                if pn == 'String':
                    return ('prop_name', False)
                if po and po[1] == 'String':
                    return ('prop_name', True)
                return ('mismatch',)
            # a pointer to a struct the port holds by value, in a Box or in an Arc
            wrap, s = None, None
            if po is not None:
                if po[1] in ('Box', 'Arc'):
                    wrap, s = 'opt_' + po[1].lower(), po[2][0]
                else:
                    wrap, s = 'opt', po
            elif pn in ('Box', 'Arc'):
                wrap, s = pn.lower(), pt[2][0]
            else:
                wrap, s = 'plain', pt
            if tgt in c.structs and self.same_struct(tgt, s):
                self.want(tgt, s)
                return ('deep', tgt, wrap)
            return ('mismatch',)
        if name in c.structs:
            if self.same_struct(name, pt):
                self.want(name, pt)
                return ('struct', name)
            return ('mismatch',)
        return ('mismatch',)

    def same_struct(self, cname, pt):
        """pt is the port's struct of the ufbx struct's name, directly or through an alias (VertexVec3)"""
        if pt[0] != 'path':
            return False
        if pt[1] == cname and (cname in self.p.structs or cname in self.p.maps):
            return True
        return cname in self.p.aliases and self.p.resolve(P(cname)) == pt

    def want(self, cname, pt):
        prev = self.port_type_of.get(cname)
        if prev is not None and prev != pt:
            sys.exit(f'gen.py: ufbx {cname} pairs with two port types: {tstr(prev)} and {tstr(pt)}')
        self.port_type_of[cname] = pt
        if cname not in self.done and cname not in self.queue:
            self.queue.append(cname)

    def note_repr(self, where, ct, pt):
        """integer widths and nullability where they differ between ufbx and the port"""
        if ct[0] == 'array' or pt[0] == 'array':
            if ct[0] == 'array' and pt[0] == 'array':
                self.note_repr(where + '[]', ct[1], pt[1])
            return
        cn, pn = ct[1], pt[1]
        if cn in INT and pn in INT and cn != pn:
            self.repr_notes.append((where, f'{cn} in ufbx, {pn} in the port'))
        elif cn == 'List' and pn == 'Vec':
            self.note_repr(where + '[]', ct[2][0], pt[2][0])
        elif cn == 'RefList' and pn == 'Vec' and self.opt_inner(pt[2][0]) is not None:
            self.repr_notes.append((where, 'non-null pointers in ufbx, Option indices in the port'))
        elif cn == 'Ref' and pn == 'Option':
            self.repr_notes.append((where, 'a non-null pointer in ufbx (ufbx.h), Option in the port'))
        elif cn == 'Option' and pn != 'Option' and ct[2] and ct[2][0][1] == 'Ref' and self.c.is_elem(ct[2][0][2][0][1]):
            self.repr_notes.append((where, 'nullable in ufbx, a plain index in the port'))

    def trivial(self, rule):
        return rule[0] in ('f64', 'f32', 'int', 'bool', 'str', 'blob', 'enum', 'struct') or \
            (rule[0] == 'list' and self.trivial(rule[1]))

    def describe(self, rule):
        k = rule[0]
        if k == 'list':
            return f'list of: {self.describe(rule[1])}'
        if k == 'ids':
            return f"pointer list vs indices ({'element_id' if rule[1] == 'eid' else 'typed_id'} of each)"
        if k == 'id':
            return f"pointer vs index ({'element_id' if rule[1] == 'eid' else 'typed_id'}; null = None)"
        if k == 'deep':
            return f'pointer to {rule[1]} vs {rule[2]} value, compared field by field'
        if k == 'reflist_deep':
            return f'pointer list of {rule[1]} vs values, compared field by field'
        if k == 'elements_ref':
            return 'element pointers vs ElementRef (element type and typed_id)'
        if k == 'prop_name':
            return "pointer to a prop vs its name"
        if k == 'flags':
            return 'flag bits'
        return k

    def cmp_code(self, rule, a, b, ind):
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
            pb = f'{b}.as_bytes()' if rule[1] == 'S' else f'&{b}[..]'
            return [f'{s}cx.str(rt::cstr({a}), {pb});']
        if k == 'blob':
            pb = f'{b}.as_bytes()' if rule[1] == 'S' else f'&{b}[..]'
            return [f'{s}cx.bytes(rt::blob({a}), {pb});']
        if k == 'enum':
            return [f'{s}if !cx.same(eq_{rule[1]}(*{a}, *{b})) {{ cx.diff(format_args!("{{:?}}", {a}), format_args!("{{:?}}", {b})); }}']
        if k == 'flags':
            return [f'{s}cx.flags(rt::flag_bits(*{a}), *{b} as u32);']
        if k == 'struct':
            return [f'{s}cmp_{rule[1]}(cx, {a}, {b});']
        if k == 'list':
            n = self.fresh()
            ca, pa, i = f'ca{n}', f'pa{n}', f'i{n}'
            lines = [f'{s}{{',
                     f'{s}    let ({ca}, {pa}) = (rt::slice_of({a}), &{b}[..]);',
                     f'{s}    cx.len({ca}.len(), {pa}.len());',
                     f'{s}    for {i} in 0..{ca}.len().min({pa}.len()) {{',
                     f'{s}        cx.idx({i});']
            lines += self.cmp_code(rule[1], f'(&{ca}[{i}])', f'(&{pa}[{i}])', ind + 8)
            lines += [f'{s}        cx.pop();', f'{s}    }}', f'{s}}}']
            return lines
        if k == 'ids':
            f = 'c_eid' if rule[1] == 'eid' else 'c_typed'
            return [f'{s}cx.ids(rt::reflist({a}), &{b}[..], rt::{f});']
        if k == 'id':
            f = 'c_eid' if rule[1] == 'eid' else 'c_typed'
            return [f'{s}cx.id(rt::{f}(rt::raw_ptr({a})), rt::PortId::port_id({b}));']
        if k == 'elements_ref':
            n = self.fresh()
            return [f'{s}{{',
                    f'{s}    let (ca{n}, pa{n}) = (rt::reflist({a}), &{b}[..]);',
                    f'{s}    cx.len(ca{n}.len(), pa{n}.len());',
                    f'{s}    for i{n} in 0..ca{n}.len().min(pa{n}.len()) {{',
                    f'{s}        cx.idx(i{n});',
                    f'{s}        let e = unsafe {{ rt::elem(ca{n}[i{n}] as *const u8) }};',
                    f'{s}        cx.push("type");',
                    f'{s}        if !cx.same(eq_ElementType(e.type_, pa{n}[i{n}].element_type)) {{ cx.diff(format_args!("{{:?}}", e.type_), format_args!("{{:?}}", pa{n}[i{n}].element_type)); }}',
                    f'{s}        cx.pop();',
                    f'{s}        cx.push("typed_id");',
                    f'{s}        cx.int(e.typed_id as i128, pa{n}[i{n}].index as i128);',
                    f'{s}        cx.pop();',
                    f'{s}        cx.pop();',
                    f'{s}    }}',
                    f'{s}}}']
        if k == 'reflist_deep':
            n = self.fresh()
            return [f'{s}{{',
                    f'{s}    let (ca{n}, pa{n}) = (rt::reflist({a}), &{b}[..]);',
                    f'{s}    cx.len(ca{n}.len(), pa{n}.len());',
                    f'{s}    for i{n} in 0..ca{n}.len().min(pa{n}.len()) {{',
                    f'{s}        cx.idx(i{n});',
                    f'{s}        if ca{n}[i{n}].is_null() {{ cx.diff("null", "value"); }} else {{ cmp_{rule[1]}(cx, unsafe {{ &*(ca{n}[i{n}] as *const u::{rule[1]}) }}, &pa{n}[i{n}]); }}',
                    f'{s}        cx.pop();',
                    f'{s}    }}',
                    f'{s}}}']
        if k == 'deep':
            n = self.fresh()
            wrap = rule[2]
            pexpr = {'opt_box': f'{b}.as_deref()', 'opt_arc': f'{b}.as_deref()', 'opt': f'{b}.as_ref()',
                     'box': f'Some(&**{b})', 'arc': f'Some(&**{b})', 'plain': f'Some({b})'}[wrap]
            return [f'{s}{{',
                    f'{s}    let cp{n} = rt::raw_ptr({a}) as *const u::{rule[1]};',
                    f'{s}    match (cp{n}.is_null(), {pexpr}) {{',
                    f'{s}        (false, Some(q)) => cmp_{rule[1]}(cx, unsafe {{ &*cp{n} }}, q),',
                    f'{s}        (true, None) => {{}}',
                    f'{s}        (cn, q) => cx.presence(!cn, q.is_some()),',
                    f'{s}    }}',
                    f'{s}}}']
        if k == 'prop_name':
            pexpr = f'{b}.as_deref().map(str::as_bytes)' if rule[1] else f'Some({b}.as_bytes())'
            return [f'{s}cx.opt_str(rt::prop_name(rt::raw_ptr({a})), {pexpr});']
        if k == 'vec4_arr':
            return [f'{s}cx.vec4_arr({a}, {b});']
        if k == 'dom_ref':
            return [f'{s}cx.dom_ref(rt::raw_ptr({a}), {b}.as_deref());']
        raise ValueError(rule)

    def conv_code(self, rule, a, pt):
        k = rule[0]
        if k in ('f64', 'f32'):
            return f'(*{a} as {pt[1]})' if pt[1] != 'Real' else f'*{a}'
        if k == 'int':
            return f'(*{a} as {pt[1]})'
        if k == 'bool':
            return f'*{a}'
        if k == 'str':
            return f'rt::to_string(rt::cstr({a}))' if rule[1] == 'S' else f'rt::cstr({a}).to_vec()'
        if k == 'blob':
            return f'rt::blob({a}).to_vec()' if rule[1] == 'B' else f'rt::to_string(rt::blob({a}))'
        if k == 'enum':
            return f'conv_enum_{rule[1]}(*{a})'
        if k == 'flags':
            return f'(rt::flag_bits(*{a}) as {pt[1]})'
        if k == 'struct':
            return f'conv_{rule[1]}(k, {a})'
        if k == 'list':
            sub_pt = pt[2][0] if pt[0] == 'path' else pt[1]
            n = self.fresh()
            inner = self.conv_code(rule[1], f'x{n}', sub_pt)
            if pt[0] == 'array':
                return f'{{ let s{n} = rt::slice_of({a}); std::array::from_fn(|j{n}| {{ let x{n} = &s{n}[j{n}]; {inner} }}) }}'
            return f'rt::slice_of({a}).iter().map(|x{n}| {inner}).collect()'
        if k == 'ids':
            f = 'c_eid' if rule[1] == 'eid' else 'c_typed'
            return f'rt::reflist({a}).iter().map(|&x| rt::FromId::from_id(rt::{f}(x as *const u8))).collect()'
        if k == 'id':
            f = 'c_eid' if rule[1] == 'eid' else 'c_typed'
            return f'rt::FromId::from_id(rt::{f}(rt::raw_ptr({a})))'
        if k == 'elements_ref':
            return (f'rt::reflist({a}).iter().map(|&x| {{ let e = unsafe {{ rt::elem(x as *const u8) }}; '
                    f'f::ElementRef {{ element_type: conv_enum_ElementType(e.type_), index: e.typed_id as usize }} }}).collect()')
        if k == 'reflist_deep':
            return f'rt::reflist({a}).iter().map(|&x| conv_{rule[1]}(k, unsafe {{ &*(x as *const u::{rule[1]}) }})).collect()'
        if k == 'deep':
            n = self.fresh()
            body = f'conv_{rule[1]}(k, unsafe {{ &*cp{n} }})'
            wrap = rule[2]
            make = {'opt_box': f'(!cp{n}.is_null()).then(|| Box::new({body}))',
                    'opt_arc': f'(!cp{n}.is_null()).then(|| std::sync::Arc::new({body}))',
                    'opt': f'(!cp{n}.is_null()).then(|| {body})',
                    'box': f'Box::new(if cp{n}.is_null() {{ Default::default() }} else {{ {body} }})',
                    'arc': f'std::sync::Arc::new(if cp{n}.is_null() {{ Default::default() }} else {{ {body} }})',
                    'plain': f'if cp{n}.is_null() {{ Default::default() }} else {{ {body} }}'}[wrap]
            return f'{{ let cp{n} = rt::raw_ptr({a}) as *const u::{rule[1]}; {make} }}'
        if k == 'prop_name':
            if rule[1]:
                return f'rt::prop_name(rt::raw_ptr({a})).map(rt::to_string)'
            return f'rt::prop_name(rt::raw_ptr({a})).map(rt::to_string).unwrap_or_default()'
        if k == 'vec4_arr':
            return f'[{a}.x, {a}.y, {a}.z, {a}.w]'
        if k == 'dom_ref':
            return f'k.dom_path(rt::raw_ptr({a}))'
        raise ValueError(rule)

    def gen_struct(self, cname):
        c, p = self.c, self.p
        pt = self.port_type_of[cname]
        cfields = c.fields(cname)
        if cname in HANDWRITTEN:
            cf, pf, why = HANDWRITTEN[cname]
            pfields = [f for f, _, _ in p.struct_fields(pt)]
            if [f for f, _, _ in cfields] != cf or pfields != pf:
                sys.exit(f'gen.py: {cname} changed: ufbx {[f for f, _, _ in cfields]} port {pfields}; update HANDWRITTEN and src/scene/special.rs')
            self.mappings.append((cname, '(all fields)', f'compared by hand in src/scene/special.rs: {why}'))
            return
        maps = p.maps.get(pt[1])
        feat = FEATURE_ARRAYS.get(cname)
        if maps:
            enum, pairs = maps
            pf = {acc: (P('MaterialMap'), f'{{b}}.{acc}()', f'm[f::{enum}::{v}]') for v, acc in pairs}
            self.mappings.append((cname, 'maps', f'named fields vs the port\'s [MaterialMap; {enum}::COUNT] read through its accessors'))
        elif feat:
            arr, enum = feat
            variants = p.enums[enum]
            pf = {}
            for v in variants:
                snake = re.sub(r'(?<!^)(?=[A-Z])', '_', v).lower()
                pf[snake] = (P('MaterialFeatureInfo'), f'(&{{b}}[f::{enum}::{v}])', f'm[f::{enum}::{v}]')
            self.mappings.append((cname, arr, f'named fields vs the port\'s array indexed by {enum}'))
        else:
            sf = p.struct_fields(pt)
            if sf is None:
                self.missing_structs.append(cname)
                return
            pf = {}
            for f, t, pub in sf:
                if pub:
                    pf[f] = (t, f'(&{{b}}.{f})', None)
        used = set()
        plan = []   # (cfield, kind, rule, port field)
        pending = []
        for f, t, pub in cfields:
            sp = SPECIAL.get((cname, f))
            if sp:
                kind, pfield, why = sp
                if kind == 'skip':
                    self.skipped.append((cname, f, why))
                    if f in pf:
                        used.add(f)
                    plan.append((f, t, 'skip', None, None))
                else:
                    if pfield not in pf:
                        self.missing.append((cname, f, tstr(t)))
                        plan.append((f, t, 'missing', None, None))
                    else:
                        used.add(pfield)
                        self.mappings.append((f'{cname}.{f}', pfield, why))
                        plan.append((f, t, 'rule', (kind,), pfield))
                continue
            if not pub:
                pfield = f.lstrip('_')
                if pfield in pf:
                    used.add(pfield)
                    rule = self.classify(t, pf[pfield][0])
                    if rule[0] == 'mismatch':
                        self.mismatch.append((cname, f, tstr(t), tstr(pf[pfield][0])))
                        plan.append((f, t, 'mismatch', None, pfield))
                    else:
                        self.mappings.append((f'{cname}.{f}', pfield, 'private in ufbx-rust: read through its offset in the repr(C) struct'))
                        plan.append((f, t, 'private', rule, pfield))
                else:
                    self.missing.append((cname, f, tstr(t)))
                    plan.append((f, t, 'missing', None, None))
                continue
            if f in pf:
                used.add(f)
                rule = self.classify(t, pf[f][0])
                self.note_repr(f'{cname}.{f}', t, pf[f][0])
                if rule[0] == 'mismatch':
                    self.mismatch.append((cname, f, tstr(t), tstr(pf[f][0])))
                    plan.append((f, t, 'mismatch', None, f))
                else:
                    if not self.trivial(rule):
                        self.mappings.append((f'{cname}.{f}', f, self.describe(rule)))
                    plan.append((f, t, 'rule', rule, f))
                continue
            pending.append(len(plan))
            plan.append((f, t, 'pending', None, None))
        # by type where only the name differs
        for ix in pending:
            f, t = plan[ix][0], plan[ix][1]
            cands = []
            for pfield, (ptt, _, _) in pf.items():
                if pfield in used:
                    continue
                if t[0] == 'path' and ptt[0] == 'path' and t[1] == ptt[1] and (t[1] in c.enums or t[1] in c.structs):
                    cands.append(pfield)
            if len(cands) == 1:
                pfield = cands[0]
                used.add(pfield)
                rule = self.classify(t, pf[pfield][0])
                self.mappings.append((f'{cname}.{f}', pfield, 'renamed (paired by type)'))
                plan[ix] = (f, t, 'rule', rule, pfield)
            else:
                self.missing.append((cname, f, tstr(t)))
                plan[ix] = (f, t, 'missing', None, None)
        for pfield in pf:
            if pfield not in used:
                self.port_only.append((cname, pfield))

        cty = f'u::{cname}'
        pty = self.rust_p_type(pt)
        out = self.out
        out.append(f'pub fn cmp_{cname}(cx: &mut Cx, c: &{cty}, p: &{pty}) {{')
        for f, t, kind, rule, pfield in plan:
            if kind in ('skip', 'missing', 'mismatch'):
                out.append(f'    // {f}: {kind}')
                continue
            bexpr = pf[pfield][1].format(b='p')
            out.append(f'    cx.push("{f}");')
            if kind == 'private':
                off = self.private_offset(cname, cfields, f)
                ctt = self.rust_c_type(t)
                out.append(f'    {{ let v: {ctt} = unsafe {{ rt::private(c, {off}) }};')
                out += self.cmp_code(rule, '(&v)', bexpr, 8)
                out.append('    }')
            elif rule[0] == 'channel_index':
                out.append('    cx.id(rt::channel_index(rt::raw_ptr(&c.external_cache), rt::raw_ptr(&c.external_channel)), rt::PortId::port_id(&p.external_channel));')
            else:
                out += self.cmp_code(rule, f'(&c.{f})', bexpr, 4)
            out.append('    cx.pop();')
        out.append('}')
        out.append('')

        out.append(f'pub fn conv_{cname}(k: &Kx, c: &{cty}) -> {pty} {{')
        if maps or feat:
            out.append(f'    let mut m = {pty}::default();')
            for f, t, kind, rule, pfield in plan:
                if kind == 'rule':
                    out.append(f'    {pf[pfield][2]} = {self.conv_code(rule, f"(&c.{f})", pf[pfield][0])};')
            out.append('    let _ = k;')
            out.append('    m')
        else:
            inits = []
            for f, t, kind, rule, pfield in plan:
                if kind == 'rule':
                    if rule[0] == 'channel_index':
                        expr = 'rt::channel_index(rt::raw_ptr(&c.external_cache), rt::raw_ptr(&c.external_channel)).map(|x| x as usize)'
                    else:
                        expr = self.conv_code(rule, f'(&c.{f})', pf[pfield][0])
                    inits.append(f'        {pfield}: {expr},')
                elif kind == 'private':
                    off = self.private_offset(cname, cfields, f)
                    ctt = self.rust_c_type(t)
                    expr = self.conv_code(rule, '(&v)', pf[pfield][0])
                    inits.append(f'        {pfield}: {{ let v: {ctt} = unsafe {{ rt::private(c, {off}) }}; {expr} }},')
            filled = {line.split(':')[0].strip() for line in inits}
            rest = [x for x in pf if x not in filled]
            ctor = 'f::' + pt[1]
            out.append('    let _ = k;')
            out.append(f'    {ctor} {{')
            out += inits
            if rest:
                out.append('        ..Default::default()')
            out.append('    }')
        out.append('}')
        out.append('')

    def rust_p_type(self, t):
        if t[0] == 'array':
            return f'[{self.rust_p_type(t[1])}; {t[2]}]'
        if t[1] in PRIM:
            return t[1]
        if t[1] in ('Vec', 'Option', 'Box', 'String'):
            pre = t[1]
        elif t[1] == 'Arc':
            pre = 'std::sync::Arc'
        else:
            pre = f'f::{t[1]}'
        if t[2]:
            return f"{pre}<{', '.join(self.rust_p_type(a) for a in t[2])}>"
        return pre

    def rust_c_type(self, t):
        if t[0] == 'array':
            return f'[{self.rust_c_type(t[1])}; {t[2]}]'
        if t[1] in PRIM:
            return 'f64' if t[1] == 'Real' else t[1]
        if t[2]:
            return f"u::{t[1]}<{', '.join(self.rust_c_type(a) for a in t[2])}>"
        return f'u::{t[1]}'

    def private_offset(self, cname, cfields, f):
        """the field's offset: after the last public field before it, the private ones between added aligned"""
        names = [x[0] for x in cfields]
        ix = names.index(f)
        j = ix - 1
        while j >= 0 and not cfields[j][2]:
            j -= 1
        if j < 0:
            expr = '0usize'
        else:
            expr = f'core::mem::offset_of!(u::{cname}, {cfields[j][0]}) + core::mem::size_of::<{self.rust_c_type(cfields[j][1])}>()'
        for q in range(j + 1, ix + 1):
            ty = self.rust_c_type(cfields[q][1])
            expr = f'rt::align({expr}, core::mem::align_of::<{ty}>())'
            if q < ix:
                expr = f'({expr} + core::mem::size_of::<{ty}>())'
        return expr

    def gen_enum(self, name):
        cvars = self.c.enums[name]
        pvars = self.p.enums.get(name)
        out = self.out
        if not pvars:
            self.enum_notes.append(f'{name}: no plain port enum of that name')
            return
        exact = {v: v for v, _ in cvars if v in pvars}
        pairs = {}
        rest_p = [v for v in pvars if v not in exact.values()]
        keys = {}
        for v in rest_p:
            keys.setdefault(norm_variant(v), []).append(v)
        for v, val in cvars:
            if (name, v) in VARIANT_ALIASES:
                pairs[v] = VARIANT_ALIASES[(name, v)]
            elif v in exact:
                pairs[v] = v
            else:
                cand = keys.get(norm_variant(v), [])
                if len(cand) == 1:
                    pairs[v] = cand[0]
                    self.enum_notes.append(f'{name}::{v} paired with the port\'s {name}::{cand[0]} by spelling')
                else:
                    self.enum_notes.append(f'{name}::{v} (= {val}) has no port variant')
        for v, val in cvars:
            if v in pairs and pvars.index(pairs[v]) != val:
                self.enum_notes.append(f'{name}::{v} is {val} in ufbx, position {pvars.index(pairs[v])} in the port')
        for v in pvars:
            if v not in pairs.values():
                self.enum_notes.append(f'{name}::{v} exists in the port only')
        arms = ' | '.join(f'(u::{name}::{v}, f::{name}::{pairs[v]})' for v, _ in cvars if v in pairs)
        out.append(f'pub fn eq_{name}(c: u::{name}, p: f::{name}) -> bool {{')
        out.append(f'    matches!((c, p), {arms})' if arms else '    false')
        out.append('}')
        out.append('')
        out.append(f'pub fn conv_enum_{name}(c: u::{name}) -> f::{name} {{')
        out.append('    match c {')
        for v, _ in cvars:
            out.append(f'        u::{name}::{v} => f::{name}::{pairs.get(v, pvars[0])},')
        out.append('    }')
        out.append('}')
        out.append('')

    def run(self):
        self.want('Scene', P('Scene'))
        while self.queue:
            name = self.queue.pop(0)
            self.done.add(name)
            self.gen_struct(name)
        for e in sorted(self.enums_used):
            self.gen_enum(e)

    def consts(self):
        def q(s):
            return '"' + s.replace('\\', '\\\\').replace('"', '\\"') + '"'
        o = []
        o.append('pub const MISSING: &[(&str, &str, &str)] = &[')
        o += [f'    ({q(a)}, {q(b)}, {q(c)}),' for a, b, c in self.missing]
        o.append('];')
        o.append('pub const MISMATCH: &[(&str, &str, &str, &str)] = &[')
        o += [f'    ({q(a)}, {q(b)}, {q(c)}, {q(d)}),' for a, b, c, d in self.mismatch]
        o.append('];')
        o.append('pub const SKIPPED: &[(&str, &str, &str)] = &[')
        o += [f'    ({q(a)}, {q(b)}, {q(c)}),' for a, b, c in self.skipped]
        o.append('];')
        o.append('pub const MAPPINGS: &[(&str, &str, &str)] = &[')
        o += [f'    ({q(a)}, {q(b)}, {q(c)}),' for a, b, c in self.mappings]
        o.append('];')
        o.append('pub const PORT_ONLY: &[(&str, &str)] = &[')
        o += [f'    ({q(a)}, {q(b)}),' for a, b in self.port_only]
        o.append('];')
        o.append('pub const ENUM_NOTES: &[&str] = &[')
        o += [f'    {q(a)},' for a in self.enum_notes]
        o.append('];')
        o.append('pub const REPR_NOTES: &[(&str, &str)] = &[')
        o += [f'    ({q(a)}, {q(b)}),' for a, b in self.repr_notes]
        o.append('];')
        o.append('pub const MISSING_STRUCTS: &[&str] = &[')
        o += [f'    {q(a)},' for a in self.missing_structs]
        o.append('];')
        o.append(f'pub const COMPARED_STRUCTS: usize = {len(self.done)};')
        return o


def find_ufbx(cargo, manifest):
    """ufbx-rust's generated.rs where cargo resolved the ufbx crate"""
    env = os.environ.get('ORACLE_UFBX_SRC')
    if env:
        return env
    meta = subprocess.run([cargo, 'metadata', '--format-version', '1', '--offline', '--manifest-path', manifest],
                          check=True, capture_output=True, text=True).stdout
    hits = [p['manifest_path'] for p in json.loads(meta)['packages'] if p['name'] == 'ufbx']
    if len(hits) != 1:
        sys.exit(f'gen.py: cargo metadata names {len(hits)} ufbx packages; set ORACLE_UFBX_SRC')
    return os.path.join(os.path.dirname(hits[0]), 'src', 'generated.rs')


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    ap = argparse.ArgumentParser()
    ap.add_argument('--out', default=os.path.join(here, 'generated_cmp.rs'))
    ap.add_argument('--port-src', default=os.path.join(here, '../src'))
    ap.add_argument('--cargo', default=os.environ.get('CARGO', 'cargo'))
    ap.add_argument('--manifest', default=os.path.join(here, 'Cargo.toml'))
    ap.add_argument('--report', action='store_true', help='print the pairing report')
    args = ap.parse_args()
    c = CSide(open(find_ufbx(args.cargo, args.manifest)).read())
    texts = [open(os.path.join(args.port_src, f)).read() for f in ('types.rs', 'math.rs', 'error.rs')]
    p = PortSide(texts)
    g = Gen(c, p)
    g.run()
    head = ['// Written by gen.py from ufbx-rust generated.rs and load3d types.rs; do not edit.',
            'use super::rt::{self, Cx, Kx};',
            'use super::special::*;',
            'use ufbx as u;',
            'use load3d as f;',
            '']
    text = '\n'.join(head + g.out + g.consts()) + '\n'
    with open(args.out, 'w') as fh:
        fh.write(text)
    if args.report:
        print(f'{len(g.done)} ufbx structs compared, {len(g.enums_used)} enums')
        for title, rows in (('missing in the port', g.missing), ('type mismatch', g.mismatch),
                            ('skipped', g.skipped), ('mappings', g.mappings), ('port only', g.port_only)):
            print(f'-- {title} ({len(rows)})')
            for r in rows:
                print('  ' + ' | '.join(r))
        print(f'-- representation notes ({len(g.repr_notes)})')
        for r in g.repr_notes:
            print('  ' + ' | '.join(r))
        print(f'-- enums ({len(g.enum_notes)})')
        for n in g.enum_notes:
            print('  ' + n)
        if g.missing_structs:
            print('-- ufbx structs without a port struct: ' + ', '.join(g.missing_structs))


if __name__ == '__main__':
    main()

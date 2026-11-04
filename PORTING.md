# Porting notes

## Where Rust differs from C

- A pointer to an element is an index into the scene's list of that type (`node.mesh: Option<usize>` into `scene.meshes`);
  an element's id indexes `scene.elements`.
- Lists are `Vec`, strings `String`, the scene is owned:
  there is no `ufbx_free_scene` or `ufbx_retain_*`.
- The `_len` variants are the functions themselves, as `&str` carries its length;
  the `ufbx_catch_*` variants are the plain functions, which panic where ufbx's would call `ufbx_panic`;
  `ufbx_format_error` is `Display` for `Error`.

## Names

A function ported from ufbx keeps its C name without the prefix, so that a name read in ufbx finds its Rust here:
`ufbxi_ascii_next` is `ascii_next`, `ufbxi_obj_parse_file` is `obj_parse_file`, `ufbx_load_file` is `load_file`.
The prefixes are `ufbx_`, `ufbxi_`, `ufbxi_macro_` for a macro and `ufbxt_` for a helper of ufbx's tests;
a public function's `_len` goes too, as a `&str` carries its length.
A test helper whose shorter name is one of load3d's own functions keeps its prefix:
`ufbxt_inflate`.

Where the name cannot be taken, a doc comment names the C function and the file that holds it:

```rust
/// `ufbx_find_prop` in ufbx.c: In these props, then their defaults
pub fn find(&self, name: &str) -> Option<&Prop> {
```

That is so for

- a public method, whose name is load3d's API:
  `Props::find`;
- a name another function holds:
  `ufbxi_evaluate_props` beside the public `evaluate_props`;
- a Rust keyword:
  `ufbxi_match` is `match_pattern`;
- one Rust function for several C functions, as `VertexAttrib::get` for `ufbx_get_vertex_real`, `_vec2`, `_vec3` and `_vec4`;
  where it can take one of their names, the comment names the others:
  `` /// With `ufbxi_next_prop_slow` in ufbx.c ``;
- a piece of one C function, or one of its typed variants:
  `` /// Part of `ufbxi_finalize_scene` in ufbx.c ``.

A C function that Rust writes inline, as a comparison passed to a sort, is named by a line comment above that code:
`` // `ufbxi_less_external_file` in ufbx.c ``.
A function that only stands in for C code it does not port says what it imitates:
`` /// Like `ufbxi_stdio_read` in ufbx.c ``.
A function with no C twin, as a constructor or an accessor, takes a Rust name and no such comment.
A comment that names a C struct or table names its file the same way.
`src/compat/` takes ufbx-rust's names.

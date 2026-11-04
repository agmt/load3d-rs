# Changing load3d

`README.md` says what the crate is for;
`FEATURES.md`, what each feature gives;
`PORTING.md`, where Rust differs from C and how a function ported from ufbx is named;
`TESTING.md`, how the crate is checked.

- **ufbx's results, bit for bit:**
  a load gives ufbx's scene, warnings and errors, its quirks included.
- **`src/types.rs` holds ufbx's fields alone**, as `oracle/gen.py` pairs them with ufbx's.
  A function beyond ufbx goes into `src/simple.rs`, behind the feature `flatten`, and `FEATURES.md`'s "The short way in", and takes no ufbx name.
- **No dependency** without the owner's word;
  a candidate is judged by its use, its releases and its tests before its speed or size.
- **A bug fix is two commits that both pass:**
  the first pins the current, wrong behaviour in a test, each wrong expectation marked `// ToDo: expected: <correct>`;
  the second fixes it, flipping those expectations and dropping the marks.
- **A commit message** is a conventional-commit subject, then at most three lines of why.
  What was measured, found or left goes there, not into a file of the repository.
- **Comments** say only what the code cannot:
  a non-obvious constraint, a quirk of a format or of ufbx, a choice that looks wrong until explained;
  one or two lines where they apply.
  The C names a port carries are `PORTING.md`'s.
- **A lint the port means to break**, as `!(a < b)` kept for C's NaN, is allowed where it is, with the reason beside it.
- **Before a push:**
  `./test.sh` passes on every new commit, then `./test-all.sh` on the last.

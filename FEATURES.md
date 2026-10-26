# load3d's features

Each format is a feature, so that a build carries only the readers it needs. `format-fbx`, `format-obj`,
`geometry-cache` and `skinning-evaluation` are on by default, the last three after ufbx's `UFBX_ENABLE_*`; FBX, which
ufbx always builds, is a feature here too, so that a build that reads only `.obj` files leaves its reader out. A load
that needs one turned off fails with "Feature disabled", its info the feature's name.

```toml
# FBX files only
load3d = { version = "0.1", default-features = false, features = ["format-fbx"] }
```

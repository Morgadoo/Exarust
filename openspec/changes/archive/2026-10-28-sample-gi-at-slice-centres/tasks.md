# Tasks

- [x] Rebase the lane onto main (already current).
- [x] Check the slice centre against the shipped `gi_inject_v/g/f`, the cube depth scale and the bias ladder against the assembly, the per-object offset and its callers' flag, and the flags2 0x10 splat test.
- [x] Take the per-object X/Z offset's scale and lift in double.
- [x] Add the GI uniform layout test against naga's layout of `gi.wesl`.
- [x] Label the unranged-light cube lookup as a guess; correct the README's bias source and the shaping lane note.
- [x] fmt, clippy `-D warnings` and tests for `exa_render`, `exa_world`, `exa_assets`, `exa_character` on the worker; workspace check.
- [x] `openspec validate --all --strict`

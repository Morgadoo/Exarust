# Tasks

- [x] `is_level` and the level list.
- [x] The restart arguments and the restart.
- [x] The panel: current map, switches, level list, F9/Escape.
- [x] The pointer rule and input swallowing; fix the pointer under Cursor Look.
- [x] Tests: `is_level`, `restart_args`, the pointer rule.
- [x] Docs: `docs/DEBUGGING.md`, `docs/UI.md`.
- [x] `make check`
- [x] `openspec validate --all --strict`
- [x] Two NVIDIA GPU launches on the final tree with the panel opened (scripted F9). The restart on a click is covered by `restart_args`'s test; the click itself needs a real pointer (Bevy's focus reads the OS pointer, which cannot be warped on Wayland) and is left to a manual check.

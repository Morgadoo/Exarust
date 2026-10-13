# Tasks

## 1. Reverse engineering

- [x] 1.1 Confirm REA is ready (`rea doctor --json`, Ghidra provider) and open `Exanima.exe` through the REA MCP tools
- [ ] 1.2 Locate the chunk-tree mesh reader (start from the reader class names in `docs/research/engine-classes.md` and xrefs to the `0x3D03` tag) and record stream, topology and transform layouts in `context/artifacts/`
- [ ] 1.3 Histogram `(semantic, type)` stream pairs and topology chunk sizes across Objlib, Objects, Components and Resource to confirm the layouts

## 2. Decoder

- [ ] 2.1 Add `exa_formats::rfc` mesh types and a total decoder for `0x3D03`
- [ ] 2.2 Handle variants `0x3D01`, `0x3D06`, `0x3D0C` or report them as unsupported with a reason
- [ ] 2.3 Triangulate polygon groups and attach material names
- [ ] 2.4 Convert to metres, Y-up, right-handed
- [ ] 2.5 Unit tests on synthetic mesh chunks (valid, truncated, out-of-range index)

## 3. Tooling

- [ ] 3.1 `exa-tool corpus meshes` per-pack report
- [ ] 3.2 GLB writer with embedded PNG textures resolved by name
- [ ] 3.3 `exa-tool export-gltf <pack> <entry> <out.glb>`

## 4. Verification

- [ ] 4.1 Run `cargo exa corpus meshes` against the installation (skipped when `EXANIMA_DIR` is unset); every static mesh record decodes or has a recorded reason
- [ ] 4.2 Export `trn_grassa01`, `afr_cornflag_a01`, a door and a weapon part; check size and orientation in Blender against the running game
- [ ] 4.3 Remove probes and dumps from the tracked tree

## 5. Documentation

- [ ] 5.1 Update `docs/formats/RFC.md` mesh section to verified, with the evidence used
- [ ] 5.2 Update `docs/PLAN.md` Phase 1 status
- [ ] 5.3 `make check`

# Tasks

## 1. Reverse engineering

- [x] 1.1 Confirm REA is ready (`rea doctor --json`, Ghidra provider) and open `Exanima.exe` through the REA MCP tools
- [x] 1.2 Locate the chunk-tree mesh reader (start from the reader class names in `docs/research/engine-classes.md` and xrefs to the `0x3D03` tag) and record stream, topology and transform layouts in `context/artifacts/`
- [x] 1.3 Histogram `(semantic, type)` stream pairs and topology chunk sizes across Objlib, Objects, Components and Resource to confirm the layouts

## 2. Decoder

- [x] 2.1 Add `exa_formats::rfc` mesh types and a total decoder for `0x3D03`
- [x] 2.2 Decode the shared node header for every node kind; report `0x3D01`, `0x3D06`, `0x3D0C` (non-mesh nodes) and instance references without failing
- [x] 2.3 Triangulate polygon groups and attach material names
- [x] 2.4 Convert to metres, Y-up, right-handed
- [x] 2.5 Unit tests on synthetic mesh chunks (valid, truncated, out-of-range index)

## 3. Tooling

- [x] 3.1 `exa-tool corpus meshes` per-pack report
- [x] 3.2 GLB writer with embedded PNG textures resolved by name
- [x] 3.3 `exa-tool export-gltf <pack> <entry> <out.glb>`

## 4. Verification

- [x] 4.1 Run `cargo exa corpus meshes` against the installation (skipped when `EXANIMA_DIR` is unset); every static mesh record decodes or has a recorded reason
- [ ] 4.2 Export `trn_grassa01`, `afr_cornflag_a01`, a door and a weapon part; check size and orientation in Blender against the running game
- [x] 4.3 Remove probes and dumps from the tracked tree

## 5. Documentation

- [x] 5.1 Update `docs/formats/RFC.md` mesh section to verified, with the evidence used
- [x] 5.2 Update `docs/PLAN.md` Phase 1 status
- [x] 5.3 `make check`

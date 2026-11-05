# Tasks

## 1. Trace

- [x] 1.1 Reflection: no mirrored camera or clip plane; the `reflection` template, its `FrmBuf` and the material writer's binding
- [x] 1.2 The low field: the post-processor's three textures and when the chain runs
- [x] 1.3 `ViewPrj`: the projection element it scales
- [x] 1.4 Which records get `LocalRef`; where the sewer's water comes from
- [x] 1.5 The sub-frame loop: viewports, jitters, clocks, the resolve
- [x] 1.6 `VXGIPos`: the point the lookup takes and the half-voxel flag

## 2. Port

- [x] 2.1 The low field, its copy after the post pass and its binding
- [x] 2.2 The local reflection in `exa.wesl`; the factory's bit in `exa_assets`
- [x] 2.3 `ViewPrj` from the projection's scale
- [x] 2.4 The GI cell from the bounds centre in the mesh tag
- [x] 2.5 `EXA_SUBFRAME`, `EXA_LOCAL_REF_DEBUG` and the debug logs

## 3. Proof

- [x] 3.1 Tests: the push, the mix weight, `ViewPrj`, the low field's size, the tag packing and the shader constants
- [x] 3.2 GPU: lamp globes in arena1 before and after, the low field's read, edges through the resolve and two sub-frames, frame time
- [x] 3.3 Docs, spec, READMEs

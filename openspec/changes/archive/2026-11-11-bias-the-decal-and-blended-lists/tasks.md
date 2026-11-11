## 1. Find
- [x] 1.1 List the TRenderer, GL immediate-draw helper and FLX_GPUPrograms routines and their state in Exarust
- [x] 1.2 REA: the scene pass's projection bias against the assembly (double add, single store, first view's term, both projection sets)
- [x] 1.3 Check the compile fallback and grey-override premises

## 2. Wire
- [x] 2.1 `passes::biased_depth_term` and `passes::list_depth_shift`, tested
- [x] 2.2 `exa_vertex.wesl` and the material key bit; `--no-list-depth-bias`
- [x] 2.3 GPU grid against round 194, lockstep traces, determinism grid

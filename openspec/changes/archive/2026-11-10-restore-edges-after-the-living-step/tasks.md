## 1. Trace

- [x] 1.1 Read the living limb step, its tree edge pass, the anchor record and the sector list's append order against the assembly
- [x] 1.2 Read the link iteration's damping and saturation and the muscle setup's smoothing against the assembly
- [x] 1.3 Check add-to-world, the pin re-scale, the health write-back and the heavy-claws starters' write against Exarust

## 2. Port

- [x] 2.1 The tree edge pass at the end of the living step
- [x] 2.2 The muscle's damping term in double and the RSQRTSS/RCPSS saturation
- [x] 2.3 The setup smoothing's precision, order and NaN
- [x] 2.4 Unit tests

## 3. Check

- [x] 3.1 fmt, clippy -D warnings and workspace tests on the worker; play build
- [x] 3.2 GPU grid before/after traces (exanima01, arena duel)
- [x] 3.3 Update the exa_physics README

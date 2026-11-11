## 1. Trace

- [x] 1.1 Exit trace with stage times, thread states and in-process stack samples
- [x] 1.2 Reproduce in four-scene grids until a stall is caught and its stacks resolved
- [x] 1.3 Read the flee start's caller and its call site in REA

## 2. Fix and port

- [x] 2.1 Patch rodio's resampler products to u64
- [x] 2.2 Run the flee start from the target pass's flee outcome
- [x] 2.3 Unit test of the flee start through the pass

## 3. Check

- [x] 3.1 fmt, clippy -D warnings and tests on the worker; play build
- [x] 3.2 GPU grids: twelve Constant Displacement grids with the fix, lockstep traces against main, flee scenes
- [x] 3.3 Update the exa_physics, exa_ai and exarust READMEs

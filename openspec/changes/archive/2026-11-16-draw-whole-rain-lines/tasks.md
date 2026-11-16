## 1. Find the cause

- [x] 1.1 Reproduce F550 on the build worker's GPU (arenao2, seed 777) with the rain hidden, in debug colour, without GI and without shadows
- [x] 1.2 Read the original rain draw's line call per sub-frame

## 2. Fix

- [x] 2.1 Whole lines in every supersample copy (`streak_vertices`)
- [x] 2.2 Test for an odd drawn count

## 3. Checks

- [x] 3.1 fmt, clippy -D warnings, the workspace tests, openspec validate, the play build
- [x] 3.2 Before and after shots on the build worker's GPU

## 1. Find
- [x] 1.1 List the TDomainWheel and TThaumTree routines with their callers and Rust state
- [x] 1.2 REA and the disassembly: the wheel's hit, hover, click, paint and label, the GUI level switch, the wheel texture load, the tree's domain flag writes

## 2. Port
- [x] 2.1 The hit test and the paint follow the GUI unit (scale and size word)
- [x] 2.2 The confirm gated by the tree's fade-out bit, the click ORing it
- [x] 2.3 The tree's wheel child built from the constructor form
- [x] 2.4 Tests

## 3. Check
- [x] 3.1 GPU grid shots of the power screen; lockstep trace and determinism compare against main
- [x] 3.2 fmt, clippy -D warnings, tests, play build, openspec --strict

# Tasks

- [x] `SolverStage::coupled` and the pairs' coupled bodies.
- [x] Free bodies' iterations and friction at once, in parallel; the rest interleaved.
- [x] Integration, level detection and post-step in parallel.
- [x] Tests: four free bodies, and a colliding pair with three free bodies, end where the interleaved path leaves them.
- [x] Every existing physics, combat and game test unchanged.
- [x] Measure: play exanima01 34 → 51–54 FPS; the fixed step 18.1 → 8.5 ms a frame.
- [x] `make check`
- [x] `openspec validate --all --strict`
- [x] Two NVIDIA GPU launches on the final tree.

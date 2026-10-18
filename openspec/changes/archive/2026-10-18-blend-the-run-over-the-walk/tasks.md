# Tasks

## 1. Overlay the run

- [x] 1.1 While Walk plays, slerp each bone toward the Run sample at the same keys by 0.7 times the run blend, before the pose hold. The root stays the walk's. A synthetic pair 90° apart at run blend 1 gives 63°, and run blend 0 stays on the walk.
- [x] 1.2 Forward locomotion plays Walk, walking or running. The jog clock rate for that walk matches the rate the run clip used.

## 2. Docs and check

- [x] 2.1 Update `docs/PLAN.md` and `docs/formats/RSQ.md`: the run overlay is on the walk; sprint weight, the backpedal clip, gait plants and the locomoting bend stay not done.
- [x] 2.2 Corpus skipped: no format change. `make check` and `openspec validate --all --strict` both pass.

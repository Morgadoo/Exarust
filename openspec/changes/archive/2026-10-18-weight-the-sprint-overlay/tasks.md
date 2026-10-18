# Tasks

## 1. The sprint weight

- [x] 1.1 Shift with the cursor further than 3 m keeps the sprint latch. Shift without that cursor does not.
- [x] 1.2 While Walk plays, the overlay is the run blend when the latch is kept, the stance level is above 0.9 and the speed is above 3.5 cm a tick, and 0.7 times the run blend otherwise. A sprint at run blend 1 meets a 90° run bone; at 0.5 it is 45°. Level 0.9 and speed 3.5 cm stay at 63°. The root stays the walk's.

## 2. Docs and check

- [x] 2.1 Update `docs/PLAN.md`, `docs/PLAYER.md` and `docs/formats/RSQ.md`: the sprint overlay weight is in. The sprint row, its clock, the tighter run cap, the backpedal clip and gait plants stay not done.
- [x] 2.2 Corpus, skipped without a format change. `make check` and `openspec validate --all --strict` both pass.

# Tasks

## 1. The sprint clock

- [x] 1.1 While sprinting, a forward gait blends from the walk rate toward the feral sprint row by √run, then multiplies by 1 − 0.1 × encumbrance. Feral 0 at run blend 1 and 4.337 cm a tick gains 0.727 frames; feral 1 gains 0.894; feral 0 at encumbrance 0.5 gains 0.691.
- [x] 1.2 A jog, a backpedal, and a latch that is not sprinting (level 0.9, or 3.5 cm a tick) keep their clocks. The capsule's speed is unchanged.

## 2. Docs and check

- [x] 2.1 Update `docs/PLAN.md`, `docs/PLAYER.md` and `docs/formats/RSQ.md`: the sprint clock is in. The sprint row's thrust, the tighter run cap, the backpedal clip and gait plants stay not done.
- [x] 2.2 Corpus, skipped without a format change. `make check` and `openspec validate --all --strict` both pass.

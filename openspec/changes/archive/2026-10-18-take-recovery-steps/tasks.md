## 1. Trigger, target and episode

- [x] 1.1 A standing body starts one recovery step from 1.5 · R · E⁴ · S, with the human bonus making a settled reach of 45 cm and bonus 0 staying 22.5, and aims the target by the velocity lead
- [x] 1.2 The footwork chooses by the stance score and the timers, one foot at a time, and a free foot under 5 cm stays
- [x] 1.3 The episode ends on its four rules, plants the best free foot and moves the standing position onto that foot's pose spot
- [x] 1.4 While the step runs and the chest is not past the trigger, the thorax pins lead by the direction term (a still 40 cm sideways error moves the target 33.6 cm) before the clamp, and the pelvis pin does not

## 2. Docs

- [x] 2.1 Update `docs/PLAN.md`, `docs/formats/RSQ.md` and the physics crate README: the recovery step's trigger, scores, timers, episode and thorax direction term are in; leg IK and gait walking stay not done

## 3. Check

- [x] 3.1 Corpus (skipped without `EXANIMA_DIR`): no format change. The human balance bonus of 1 that the reach multiplies is already pinned by the race tests (`shipped_races` / the character-stats tests)
- [x] 3.2 Run `make check` and `openspec validate --all --strict`, and verify both pass

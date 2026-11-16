## 1. Check

- [x] 1.1 `schedule_ambiguity_test`: the game's App headless in lockstep, every fixed-step schedule and the frame's `PreUpdate`, `Update`, `PostUpdate`
- [x] 1.2 Allowlist with a reason per entry; TODO counts for `Update` and `PostUpdate`
- [x] 1.3 `EXA_AMBIGUITY_DUMP=1` debug listing

## 2. Order

- [x] 2.1 `CharacterTick` (9 pairs) and `FixedPreUpdate` (15 pairs)
- [x] 2.2 `FixedUpdate` (1125 pairs): the step's phase sets, then each plugin's systems in them
- [x] 2.3 `PreUpdate` (28 pairs)
- [ ] 2.4 `Update` (5796 pairs) and `PostUpdate` (273 pairs): TODO

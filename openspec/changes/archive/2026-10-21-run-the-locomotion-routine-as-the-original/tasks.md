## 1. Research

- [x] 1.1 Decompile the locomotion routine and read its instructions in `exe.asm` (precision, compares, the fraction and maximum helpers, the dot-product and projection orders); read its constants from the PE
- [x] 1.2 Read the body tick's move and stop branches (stance factor, leg routine calls, gait reset) and its standing clock branch; the leg routine; the gait loader; the character init's feral sprint row
- [x] 1.3 Check the readers of the turn's countdown factor (dead) and the stance facing (the footwork's stance frame); record everything in `context/re/locomotion-kernel.md`

## 2. The routine

- [x] 2.1 `GaitRow`, `GaitTable` (the loader's per-tick scaling, the built-in stops, the backpedal's negation, the feral sprint row, the clock factor); `RaceMuscles::gaits` in place of the clip rates; the viewer scales `races.rdb`'s rows
- [x] 2.2 `locomote`: run blend, gait columns, walk start (`Start`), boost ramp, drive, speed and direction, clock step with and without support; `stand`; `advance_clock`; `fraction`
- [x] 2.3 `decide` resets the gait to the walk row; `turn` wraps once and keeps the stance facing; `Locomotor` fields; `Footing::gaits`; `Ragdoll::root_moved`

## 3. Wiring

- [x] 3.1 `stages::movement`: the gait table from the race and the character, the routine for a moving body (v from the capsule and the pelvis), the walk start or resume, the kept clock; `stand` otherwise; the bearing's leg mode
- [x] 3.2 `stages::sampler` and `limp::clip_frames`: the clock takes the locomotion's step; Exarust's walking clock removed
- [x] 3.3 Muscle gains: the leg routine and the stance factor by `Bearing::legs`, `stance_run` and `full_stance` (posture stage: dormant or the shock's stagger), double precision, skipped collapsing
- [x] 3.4 `balance::turn_to` snaps the stance facing

## 4. Tests

- [x] 4.1 `exa_physics` gait: the scaled rows (1.07108, 0.079837, 0.3253, −0.33735, 50/200/250/50 cm, the half-feral sprint); the run blend (20 ticks, 0.85, 0.8, 0.5 held at 0.81, 0.55 unsupported, 0.835, 0.36, 0, 0.475); standing (0.48, 0, 0 getting up); columns (0.23670, 0.26265, 125/150/125/125 cm, thrust × level, the backpedal row); the clock rate (0.19 latched jog, 1/0.9, 0.95 crouched, none at 0.09); start keys (2, 4, 5, 8, none, 2 at 1 rad, 600 ms) and the boost ramp; the drive (0.85687, 0.18207, 1.17819, ×1.36, boost, brake); the speed; the clock step (0.76433, ×0.976, ×1.12, stopping 3 and 6 cm); unsupported (0.85, 0.58361, 0.35, unset); wrap 79.5 and the fraction
- [x] 4.2 `exa_physics` muscles: the stance factor (0.705, 0.85, 1 full stance), the thighs (0.6 running, 0.25 backpedalling and standing, kept collapsing), the limit (0.4, 0.5); `the_stop_distance_follows_the_run_blend` on the gait columns
- [x] 4.3 `exa_game`: the walk starts at key 2 in the decision's tick, steps 27/83, resumes at 600 ms and restarts at 612 ms; the standing fall; the clock takes the stride; the player's walk starts in the tick W is decided
- [x] 4.4 Real-data check: none needed (the gait rows are `races.rdb`'s, already checked by the races parser's corpus test)

## 5. Docs and checks

- [x] 5.1 `docs/formats/RSQ.md` (Locomotion, Movement decision, Gains, clock table), `docs/PLAYER.md` (Movement), `crates/exa_physics/README.md`
- [x] 5.2 `cargo fmt --all`, remote `test` and `clippy -D warnings` for `exa_physics`, `exa_game`, `exarust`; `openspec validate --all --strict`

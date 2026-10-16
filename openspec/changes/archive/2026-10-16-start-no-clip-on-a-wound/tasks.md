## 1. Reactions

- [x] 1.1 `exa_combat::react_to_hits`: remove the branch that started Head Pain (bone ≥ 9) or Rebalance once a blow's penetration plus impact passed 0.2; a struck fighter's clip is left alone; a comment says stance changes come from balance, not from the blow
- [x] 1.2 A struck fighter with `Attacking` loses it (its blade is no longer tested) and loops the stance its attack would have looped (`stance_of`: the clip's follow-up, else `Idle` for the Default moveset or `Cmbt - Idle`), commented as the stand-in for the original's swing stall until C21; a killing blow also removes `Attacking`
- [x] 1.3 Health, the shove with its daze, `GoLimp` on death, the hit log and the clean-up of finished attacks are unchanged

## 2. Tests

- [x] 2.1 The damage tests' sword record becomes a `#[cfg(test)] pub(crate)` helper (`damage::sword_a`)
- [x] 2.2 `a_wound_starts_no_clip`: for bones 13, 9, 10 and 4, in a fresh app each, the 20 cm/tick cut 70 cm along the sword on bare flesh passes 0.2 of penetration plus impact and does not kill; afterwards the fighter plays `Idle`, looping, no follow-up, time 0, yellow under 0.25 (equal to `Health::take` of the outcome), not down, exactly one shove on that bone with the outcome's daze, no `GoLimp`; a fighter getting up keeps `Get up`
- [x] 2.3 `a_wound_stops_a_swing`: a fighter 12 frames into `Attack Mid` with `Attacking` is cut on the forearm: no `Attacking`, `Cmbt - Idle` looping from time 0, one shove
- [x] 2.4 `a_killing_wound_still_goes_limp`: at yellow 0.011 the chest cut sets down, yellow 0, inserts `GoLimp`, and ends an attack in progress
- [x] 2.5 The three tests fail on the previous `react_to_hits` (Head Pain, follow-up `Idle`; `Attacking` kept)
- [x] 2.6 Corpus (skipped without `EXANIMA_DIR`): `shipped_elbows_and_knees_bend_within_their_links` turns each forearm and lower leg of the installed `humanoid.rsq` about its hinge in 0.1° steps: elbows 0–120° (10° past straight stretches the long link 1.77 cm beyond its slack, 45° past 7.04 cm), knees 10.0–130.0°; `exa-tool rsq` lists the elbow links (3 and 12 cm) and Head Pain as action `0x2d`

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh --gpu 4000`, with a temporary probe (each elbow's flexion about its hinge, every arm link's stretch beyond its slack, clip changes, hits, a frame 70 ticks after each hit on the player); "before" is main at 78f3510 with the same probe; no `Validation Error` or panic in any log:
  - R1 `--opponent play arena` (150 s): before, 9 wounds (bones 12, 9 and 8, three each) and the player held Head Pain for 882 of its 1 374 living rows (64 %), elbows up to 125°, down at tick 1948; a second run started Head Pain 5 times in 9 wounds, and its frame 70 ticks after a wound shows both hands on the head. After, 8 wounds on bone 12 (and 4 contacts past the tip): the player stays in Idle for all 1 634 living rows, elbows 17.9–27.5°, every arm link within 0.05 cm of its slack, down at tick 2316 and limp; a second run, 8 wounds, Idle throughout (all four frames after a wound), down at tick 2302. The duellist's elbows stay within 15.2–108.5° and never pass straight, arm links at most 0.78 cm over attacking and 1.19 cm in its stance (the clavicle's 0.73 cm link)
  - R2 the same with `--weapon ub05ssworda` and the probe making the player face the duellist and attack every 1.3 s: 18 wounds, 11 of them on a fighter in the middle of an attack (5 on the player, 6 on the duellist), and each of those attacks stopped in the tick of the wound, back in `Cmbt - Idle`; every one of the 18 wounds came from an attack still running; the duellist, killed at tick 2177, went limp. Living elbows 15.3–108.4°, arm links at most 1.20 cm over
  - R3 `--shove 60 play arena --trace`: falls 13 ticks after the blow (before 12), `Get Up 2` 207 ticks after the fall and Idle 147 ticks later (both as before), standing 105.76 cm at the end (before 105.76)
  - R4 `--weapon ub05ssworda character "One Handed/Attack Mid"`, `Up`, `Down`: elbows 19.8–94.0°, 22.0–92.3°, 15.0–107.6°, never past straight; worst arm link 1.90, 1.77 and 1.12 cm over its slack (the clavicle's 0.73 cm link)
  - R5 `--limp character`: limp at tick 30 (before 29); the limp elbows lie between 2.4° past straight and 122.1° (before 3.2° and 122.0°), the elbow's long link at most 0.32 cm over its slack (before 0.35), every arm link within 1.40 cm (before 1.44)
- [x] 3.2 Remove the probe from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/OTHER.md` (Damage: what the struck character does), `docs/formats/RSQ.md` (Physical body: joint ranges; Motion set: Head Pain), `docs/PLAN.md` (Phase 4)
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (233 passed, 3 ignored)

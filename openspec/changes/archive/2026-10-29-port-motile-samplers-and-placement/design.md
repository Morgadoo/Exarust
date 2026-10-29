## Context

The rules live with their nearest callers: the heights in `exa_physics` `body_pose_heights.rs` (`body::pose_heights`), the placement arithmetic in `body_placement.rs` (`body::placement`), the probe next to the hovering probe in `body_feet.rs`, the key pick and the queued-clip switch next to the shared bracket in `exa_anim` `clock.rs`, the hunch in `exa_game` `stages/sampler.rs`. Lengths are the original's centimetres. The living sampler's rules are in `docs/formats/RSQ.md` (Motion playback); this change adds no format.

## Decisions

- **Heights as data.** The carry's pelvis target (`body_lift.rs`) is the feet routine's; the samplers' target is what the original's feet modes and the next sampler read. Calling it would change the lift, so it stays a tested rule.
- **The pose-only walking target as the assembly has it.** It adds the pelvis's own posed height to the stand height, unlike the living sampler's twice the run blend; kept, and named in the routine map's corrections.
- **Placement arithmetic not called.** Exarust's spawn puts the clip pose on the character frame (pelvis at the frame plus the clip root); the original lowers the pose onto a frame at pelvis height. Round 23 showed a spawn change can make every body squat, so the spawn stays.
- **Hunch moved.** The original writes the hunch into the pinned part's posed placement after the twist; children keep their placements. A clear difference, feral characters only, so it is wired.
- **NaN rules.** The lowest contact skips a NaN (`COMISS` with the parity branch), a NaN root subtracts, the crouch and get-up tests widen to double and refuse a NaN.

## Open questions

- Which field of Exarust's body would take the down probe's ground (the original's body ground and previous ground): answered when the down body's ground is modelled (REA on the readers of the two fields).
- Whether the placement's pelvis-height factor ever leaves 1 (only the constructor writes it in the character class; REA cross-references show one writer elsewhere, in another class).

## Risks

- The hunch change alters feral characters' head and arm targets while they hunch. Check on GPU with a feral opponent (`play arena1 --opponent`), plus `--trace stand.csv --frames 2200 play arena` (pelvis height about 101 cm) and `--press W@150-420 play arena` for the unchanged human player.

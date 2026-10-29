## Context

The rules live where their callers are: the drive reset in `exa_physics` `stand` (the movement stage calls it for every body that does not locomote), the golem smoothing on `Ragdoll::with_race` from `RaceMuscles::golem` (set by `exa_game` `character_muscles` from `CharacterStats::golem`), the pain sounds in `exa_combat::pain`, the feet modes in `exa_physics` `body_feet.rs`, the leg order in `body_balance.rs`.

## Decisions

- **Drive reset in `stand`.** The original zeroes the drive before its locomotion call; the locomotion routine writes it again when the body moves, so clearing it where Exarust's standing branch runs is the same rule.
- **Golem smoothing at build.** Every body link (the original's pivot flag) of every part takes 0.995, overriding the distance rule.
- **Pain sounds as data.** `pain_sounds` returns the wound sound's and the voice line's loudness; the voice request (`voice::Voice`) applies the character's gains and wait. Nothing plays voice samples yet, so it is not called.
- **Leg order not called.** The human's first thigh sits at negative x, where nothing swaps; calling it would only matter for other skeletons, unverified against the installed models.
- **Feet modes not called.** No Exarust body carries the flat-floor condition bit, and the race hover flag is not passed to the body.

## Open questions

- The down body's ground probe adds an offset from the character's scene object to the pelvis; its source is not identified (REA on the probe's caller).
- Whether any installed skeleton has its first leg at positive x (corpus check over the race models).

## Risks

- The drive reset changes live stepping after a walk: a body that stops and takes recovery steps loses the forward push it kept. Check on GPU with `--trace stand.csv --frames 2200 play arena` (pelvis height about 101 cm), `--press W@150-420 play arena` (the stop after release) and `play arena1 --opponent`.

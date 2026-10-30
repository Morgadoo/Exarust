# Proposal

## Why

Phase 6 (AI) of docs/PLAN.md, the golem guardians of `exanima06`. The
alarm zones now wake the statue golems, but Exarust built a physical body
only for the human frame (race bytes 0–2): a golem was a static model with
a brain but no muscles, no mind and no duellist, so a woken golem rated the
player at pass level 3 and could neither enter combat nor move. The
original builds every race's body with the one character builder (race
model, shared skeleton links, the character init's race presets), so a
golem walks and fights with the same machinery as a human.

Evidence (REA, Ghidra listing of `Exanima.exe`): the character builder
(race-generic; no race gate on the body), the spawn from a placement (the
initial motion Stand 2, motion 0x19, for a record whose race word carries
the AI bit `0x1000`, else Idle 0x13, both current and requested, with a
blend of 20), the sentry state (pose word 2 and the statue-rest byte when
the statue byte is set and the current motion is the requested one, else
pose 1; the first time, the state-flag bit `0x400` and the sleep scales on
the parts' and held bodies' contact mobility; the request of Stand 2), and
the golem wake (pose word and statue-rest byte to 0, the latch cleared,
every part's contact mobility back to its base, the held bodies times 50).
Corpus: `exa-tool races` (golem races 16–20: scale 1.2, size 1.4/1.5,
moveset Golem, damping ratio 0.95), `exa-tool chunks Resource
golembase.rfc` (the eighteen bones `0100Pelvis` … `1800LHand`, the same
numbering as the human's), `exa-tool clips Golem` (the Golem moveset
overrides only Walk).

## What Changes

- `exa_game::scene_bodies::physical_frame`: the human frame and the golems
  (race bytes with bit `0x10` other than the skeleton and the wraith) are
  built physical; only the human frame is dressed.
- The level's placed characters take the character init's muscles
  (`character_muscles`: the golems' arm relaxation and 0.995 link
  smoothing), and a golem gets a mind and a duellist like a human.
- `exa_ai::golem`: the start motion (`start_motion`, `start_action`), the
  sentry tick (`sentry_rest`) and the sleep scales and their undoing
  (`sleep_scales`, `wake_scales`).
- `exa_game::golem_body`: the live sentry tick (`StatueRest`, the latch,
  the scales, the Stand 2 request) and a debug trace of the guardians.
- The posture stage gives a resting statue the dormant drive; the golem
  wake ends the rest and takes the scales off.
- `StateFlags::TURNS_BLOWS` is the sentry's latch: a resting statue turns
  blows until it wakes.

## Out of scope

- The alarm return (awake golems sent home and asleep) and the glow fade.
- The sentry's stop of a running locomotion and its move-goal copies.
- Why the golems' halberd blows land with zero impact in the GPU run
  (they strike with the haft at 110–150 cm along): the duellist's standoff
  for a long weapon on a scaled body is not traced here.
- Other non-human races (ogre, demons, beasts) stay static models.

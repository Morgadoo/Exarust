## Context

Exarust builds a body from `PartSetup`s whose pins are the skeleton's raw
records in metres. The original's add-to-world multiplies the same raw
records, in centimetres, by the character's size before the muscle setup.

## Decisions

- The re-scale is applied to the setup's pins before `Ragdoll::new`. The
  chest weights, the support pins and the chest point then follow without
  further change, as the original derives its weights and world point
  from the re-scaled record.
- The pick for the attack starters is a system of its own, ordered before
  the starters' aim. The steering keeps reading the behaviour's stored
  target from the previous tick, because the original's steering runs
  before the auto-target in the same behaviour tick. The pick runs twice
  in a step on the same inputs (the cursor's aim changes only in the frame
  schedule).
- A plain part takes the limp retention (or the environment's, when set)
  and gravity. Its countdown and wake requests are not kept apart from the
  body's: the living root never reads them.

## Open questions

- The original multiplies centimetres and Exarust multiplies metres, so a
  scaled pin may differ in its last bit.
- The brain's target for NPC starters is still copied in the character
  tick, a tick after the brain picks it. The AI's own order was not
  traced in this lane.

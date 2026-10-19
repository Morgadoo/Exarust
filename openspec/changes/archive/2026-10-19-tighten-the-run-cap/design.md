# Design

## Cap

The exploration steering computes the sprint latch first. That latch is Shift with the cursor further than 3 m. `run_cap` then uses 0.4 times encumbrance when the latch is kept and 0.3 otherwise, subtracted from 1 in double precision and stored as a float. The walk keys spend the latch, so they keep 0.3. A blow does not knock the fighter out.

## Open questions

- The locomotion clock's stride terms are not this cap.
- Spawning the practice partner is not this cap.

## Docs

`docs/PLAN.md`. `SG_PlayerControls` stays 8%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

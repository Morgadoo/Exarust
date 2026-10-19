# Design

## Wind

`react_to_hits` calls `answer_low_health` when a blow leaves the fighter alive under 0.05 yellow. A granted wind whose ready clock is behind the game clock sets the active flag, moves the ready clock forward by 180000 ms, and writes `SecondWindFired` with volume bits 0x3dcccccd and pitch bits 0x3f800000. It does not write `Weakened`. At that ready clock, and without the armour, it writes `Weakened` and leaves the wind alone.

## Open questions

- Limb impairment is not this check.
- A low yellow bar does not knock the fighter out from this path.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

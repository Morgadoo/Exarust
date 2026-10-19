# Design

## Heal

The living health tick calls `heal_second_wind` after the heal pool and before the regeneration ramp. An active wind adds the double 0.0003 to yellow and stores the float. The end clock is the ready clock minus 180000 ms plus 3000 ms, compared as a signed value. While the game clock is at or before that end the wind stays active. The first clock past it still adds, then the active flag clears. A quiet wind returns yellow unchanged. A dead fighter does not take this tick. Attacking holds the ramp and does not hold the wind.

## Open questions

- Limb impairment is not this tick. Low health does not slow a limb.
- A low yellow bar does not knock the fighter out from this path.
- Practice mode's dummy and sparring partner are not this heal.

## Docs

`docs/formats/OTHER.md` (Regeneration). The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

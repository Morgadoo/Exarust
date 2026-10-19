# Design

## Parry

`unarmed_parry` starts from the threat, not from a clip already chosen. It refuses a helpless fighter, a break-tangle motion, and yellow under 0.01. The attacker must be attacking, not cleaving, in front, and within reach plus 150 cm. The clock must sit from 0.3 through 0.9 unless the combo flag is set. Size 1.3 or more needs a shield. The speed is 1 and the blend is round(20 times that). The height row is the motion's pair. The side fraction is the wrapped yaw over pi, plus one half. The down clip is chosen when that height is at or below the chest difference.

## Hold

`stick` still builds the link. `grab_of` copies its span, flags, and pull of 1 into a grab. `BiteHold` runs on the solver's link passes and calls `Ragdoll::pull_grab`. The pull is that strength over 1 plus 0.001 times the gap in centimetres, shared equally and oppositely. The strain falls each pass and the grab lets go below 0. A depth of 0.1 never opens a grab, so the solver leaves the parts apart.

## Open questions

- Armed parry grips, and the clip that plays on the body, are not in.
- The lifetime unit is still passed in.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

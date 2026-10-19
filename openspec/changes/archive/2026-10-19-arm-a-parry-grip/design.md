# Design

## Grip

`armed_grip` starts from the defender's handling, the horizontal distance, and whether the blow has passed. A cleave returns nothing. The falloff is 1 below 80 cm, 1 minus the distance past 80 cm divided by 170 through 250 cm, and 0 after that. A weapon that has not passed uses the handling as its speed and (1.4 − handling) times the falloff times 0.75 times 0.6 as its strength. A passed weapon squares the handling and sets the disabled flag 0x100000. A shield's speed is 1 minus half the weight times stamina. Its strength is stamina times weight times the falloff times 0.75 times 0.4, and its flags are 0x800. The blend is 20 times the speed, rounded. Slot 1 is chosen only while the defender is attacking with a motion outside 8 through 10 and at least two weapons are held. The other hand is disabled when it is held.

## Open questions

- The predicted point along the attacker's weapon is not built here. The caller passes the horizontal distance and whether the blow has passed.
- The clip still comes from the unarmed height table.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

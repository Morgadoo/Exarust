# Design

## Contact

`dying_tick` still spawns a solid weapon as an awake rigid body. `step_bodies` is the shipped solver over every body, including that weapon and a body whose sphere starts on the weapon's. Detection records one touch between those two entities and pushes them apart, so the weapon's first particle is not where one step of the weapon alone leaves it.

## Open questions

- The striking word is still cleared outright. The original flag word only loses bits 0, 16 and 17.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

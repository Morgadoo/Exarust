# Design

## Estimate

`estimate::fighting_base` is computed once when a brain's record is
applied; `fighting_estimate_with_base` adds the held weapon's raw stat
bytes (kept on `WeaponRecord::stats`) times the character's reach and the
body scale at combat entry, so the duellist tick does no record work.
`exa_game::experience::fighting_estimate` calls the same function.

## Origin

`measured_origin` takes the scored combatant's body, chest, heading,
locomotion flag and speed, step flag and goal, attack phase and its own
skill. The duellist reads the target's `Locomotor` (amount for the flag,
speed in cm a tick for the lean) and its brain personality's skill (0
without one); the step flag is not exposed, so the stepping branch is not
taken. The scene-node offset the original adds is zero in Exarust's single
world frame.

## Arc

`Swing::heading` is the head look's yaw (callers pass the body heading
while the tracker does not run on characters) and `Swing::facing` the
published look.

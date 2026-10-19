# Design

## Rate

`charge_rate` turns the tick in milliseconds into seconds as a double, stores that as a float, then multiplies by five sixths as a double and stores the float.

## Step

`step_charge` charges while the state is 0: the amount rises by the rate, and a value above 1 clears to 0, sets the state to 1 and reports a release. A value of exactly 1 keeps charging. A negative state fizzles by twice the rate, and a value below 0 becomes 0 with state −2. A state of 1 or more adds three times the rate.

## Chunks

`grow_radius` adds the tick times 0.7 of the speed plus 3, times 0.0006, in double precision. `separate_chunks` pushes the first chunk along the gap and the second the other way, by half the overlap, when the gap is shorter than twice the first radius and not zero.

## Open questions

- Wave, torrent, barrier and shell meshes are not in.

## Docs

`docs/formats/OTHER.md` gains the charge rule. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` leave `TForceBlastEffect` at 20%.

# Design

## Rate

`wave_rate` is the tick in seconds, then five thirds of that. The charge state is the blast charge: passing 1 releases, exactly 1 keeps charging, a fizzle falls by twice the rate, and a release rises by three times the rate.

## Front

`charge_wave_radius` adds 0.0015 while charging or fizzling. `release_wave` adds 0.6 to the reach, 0.0006 of the speed to the radius, and sets the trail step to 0.07 of the speed plus 1.

## Open questions

- The trail ring, the wave mesh, torrent and shell are not in.

## Docs

`docs/formats/OTHER.md` gains the wave rule. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the wave charge at 20%.

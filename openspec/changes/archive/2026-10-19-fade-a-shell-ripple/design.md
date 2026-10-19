# Design

## Ripple

`step_shell` subtracts 0.04 times the scale from the strength. The shown value then moves toward that strength, and the move is limited to 0.06 times the scale upward and 0.04 times the scale downward. If both are then under 0.05 the ripple is gone. The stored scale is 1, so those limits are 0.06 and 0.04.

## Open questions

- The copied mesh, torrent, and the shell's 800 cm size are not in.

## Docs

`docs/formats/OTHER.md` gains the shell ripple. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell ripple at 10%.

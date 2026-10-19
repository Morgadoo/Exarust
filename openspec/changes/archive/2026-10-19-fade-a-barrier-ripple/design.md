# Design

## Ripple

`step_ripple` subtracts 0.05 from the strength, then moves the shown value toward it by at most 0.04. If both are then under 0.07 the ripple is gone. Otherwise the place moves 0.05 of the way from where it is toward the hit.

## Extent

`dome_extent` is 30 plus 1.25 times the number of ripples. No ripples is 30, which is the value stored before the first step.

## Open questions

- The polyhedron the dome is built from, and wave, torrent and shell, are not in.

## Docs

`docs/formats/OTHER.md` gains the ripple rule. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the dome ripple at 20%.

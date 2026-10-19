# Design

## Mix

`step_torrent` sets the level from the charge while the phase is below 1: (2 minus the charge) times the charge, then 0.4 of that. From phase 1 on, the level falls by 0.01, and a result below 0.02 stops. Phase 1 eases one channel toward 0.5 and the other toward 0.6, each by 0.06 of the gap. Phase 2 multiplies both channels by 0.96.

## Open questions

- The stream mesh, the pushes, and the torrent's tick rate are not in.

## Docs

`docs/formats/OTHER.md` gains the torrent mix. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the torrent mix at 10%.

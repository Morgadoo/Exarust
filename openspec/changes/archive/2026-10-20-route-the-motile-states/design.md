# Design

## Behaviour

The collapse bit 0x800 skips the dispatcher. The shock bit 0x40 routes to the shock window. Balance below 0.6 or condition 0x300 enters the posture block: 0x300 is get-up, balance below 0.3 with motion below 25 is lying still, an upright lean is upright, a forward dot above 0.4 with slow velocity below 8, balance above 0.1 and offset below 80 staggers, otherwise the body falls. Outside the posture block, clips 5–12 attack; tone below 0.85 is weak; clip 0x0D kicks; parrying, clip 4, clips 0x22–0x28, clips 0/1 and clip 0x1F take their branches; the rest stand.

## Model

`exa_physics::motile` adds `Route` and `route`.

## Open questions

The branch bodies, animation calls and limb pull.

## Docs

`TMotileDynamics` is **32%**. The headline rises slightly.

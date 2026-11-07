# Design

Extends [docs/RENDERING.md](../../../../docs/RENDERING.md) (Pass order).

## Behaviour

Bevy sorts its transparent phase ascending by view depth plus the
material's `depth_bias`, and uses that bias only for sorting. Exarust sets
the bias from the original's list order: decals at the lowest block (in
registration order), then the blended list in the `sort_blended` order of
its states (registration order fed into the selection, so the opacity
quirk is kept), each rank 2048 m apart (twice Bevy's far plane), and
everything without a state (effects drawn in the original's later passes)
at its plain view depth. With at most 512 ranks per list the largest bias
is about 3.2·10⁶, where f32 still separates depths 0.25 m apart.

## Deviations

- No per-object sector in Exarust: all sectors are one walk.
- Within one state Bevy's distance order stays, where the original uses
  the sector's registration order.
- Build order stands in for the scene's material list order.

## Evidence

Logged state counts per level and before/after grids at a fixed camera:
exanima01 differs by 0.134% of pixels against a same-build floor of
0.125%; exanima06 is within its floor.

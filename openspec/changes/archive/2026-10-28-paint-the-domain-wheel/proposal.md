# Proposal

## Why

Phase 7 (Game modes and content) carries the thaumaturgy tree screen
(`TThaumTree`) and its domain wheel child (`TDomainWheel`). After the
round-24 review, four gaps were left. The running screen showed the bare
`domainwheel` sheet where the wheel paints itself. The sheet was not
resampled to the 32 GUI units the load asks for. The press that would
enter the node and link-turn edit drags was missing. The link ribbon was
said to lack its spline texture.

## What Changes

- `exa_ui::wheel_draw` paints the wheel in the original's order: the
  flamelets backdrop, six sector bands, the choice glow, the six icons
  with the hot icon's glow copies, the ring frame and the label. It keeps
  the three wheel programs' sources and their fragment arithmetic.
- The canvas evaluates those programs with blend `0x103` over the
  flamelets sheet, which repeats.
- The tree writes its focus total into the wheel's own focus value (48
  with no record). The running screen hovers the wheel every frame and
  draws its paint.
- `exa_assets` resamples `domainwheel` to `unit · 32` pixels through the
  engine's resample-to-size path. The format word and flags come from the
  file. It resamples again when the unit changes.
- No shipped press enters the edit drags. The press clears the drag state
  and only starts the pan, and nothing else writes the node, link-turn or
  link states. This is now documented and pinned by a test, not invented.
- The links' texture unit is unbound and the spline program samples
  nothing, so the ribbon has no texture to add. The docs say so.

## Evidence

Read in the executable's listing: the wheel's paint slot, its label
routine and its three program builders and their sources. Also read: the
immediate-draw helpers (vertex, ink and texture-coordinate slots), the
skin's rounded box, the line skin's constructor inks, the wheel texture
load and the tree's press. Every write of the tree's drag-state field was
checked, as was the screen draw's texture binding before the links.

## Out of scope

The GUI form class (`TGUIForm`) and the base form's key handlers. A GPU
submission of the tree's draw. The mip-mapped sampling of the flamelets
sheet (the canvas samples its base level).

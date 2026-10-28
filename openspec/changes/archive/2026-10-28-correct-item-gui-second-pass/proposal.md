# Proposal

## Why

The second item-GUI pass (Phase 4, Combat and items, in docs/PLAN.md)
added the item icon render plan, the description cards' rows, the card
window, the panel's mouse composition, the item icon program and the
panel's drawing through the GUI path. A review against the machine code
found rules that followed the pseudo-code rather than the instructions,
and two copies of rules Exarust already had.

## What Changes

- The icon render's projection adds the packed transform's terms in pairs;
  the extents subtract in single precision before the double widening;
  the scene's depth bounds read the camera axis, start at the first kept
  node's stored depth and take the radius over the whole half-extent
  vector; the alpha global's screen-quad pass replaces the downsample and
  halo.
- The description cards: the title tint is a double product and only the
  weapon and apparel builders tint, while the rank gate is off; the colour
  blend rounds half to even; the value row has its own gate; a weapon's
  swatch adds height only over two striking rows and its points read the
  skill bits less one for the `0x10` flag pair; a fixed shield shows no
  rank, points or value; the refresh fills the striking bars in order of
  the non-zero bytes.
- The panel's mouse event checks the drag bit, then the double click, then
  the right click; a use-class-1 double click closes the card; the right
  click reads the use section's kind. The card window keeps the existing
  card timer.
- The item icon program's arithmetic lives once (the GUI's `HltCol`); the
  panel's icons are placed in the pack panel's client area and clipped to
  it, and the atlas is copied only when an icon draws.

## Evidence

The item GUI pack's assembly for the extents, box extents, scene bounds
and render routines, the weapon, apparel and shield card builders, the
card refresh, the panel's mouse handler and the card timer routines; the
engine's packed transform, two-vector subtract, vector length and colour
blend helpers.

## Out of scope

The off-screen item renderer, the forms the cards and popups would build,
binding the player's carried list to the pack, the socket manager's
per-level cell, and the streamed-object sentinel.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: card gates, tint, refresh slots, the panel's mouse
  order and the icon render plan.
- `gui-draw-path`: the item panel's icons are placed in the pack panel.

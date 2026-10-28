# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items), the itemgui2 and
wire-body-inventory paragraphs: the icon render plan was a tested function
and the item panels' `HltCol` icon layer drew an empty atlas. "Still open:
the off-screen item renderer" and "Next: the off-screen item renderer so
the panels draw icons".

## What Changes

- `exa_game::item_icon_render`: each icon a panel reserves is rendered by a
  private off-screen Bevy camera (at most two started a frame), read back
  asynchronously, resolved to the icon size and put in the manager's atlas
  and cache slot; the inventory pack and both loot panels draw the cells.
  A headless App, a blank readback and a lost one paint the same scene on
  the CPU.
- `exa_items::item_icon`: the icon camera from the plan, the framing sun,
  the baked scene and a CPU rasteriser; `ItemCatalogue::model` gives the
  item's ground model and flags; `PanelIcons::reserve` takes the original
  acquire's cache key.
- Review fixes: the camera is the engine's scene fit
  (`item_renderer::frame_distance`) with the icon view's 40° field instead
  of a guessed 30° field, framing floor and 0.9 fill; the camera node starts
  at (0, 0, 500); the extents are projected in the original's centimetres;
  the icon camera, sun and meshes are kept off the world (an `Offscreen`
  marker every world system skips, a dedicated layer below the lights'
  shadow layers, no shadow casting); the icon systems do nothing with no
  pending icon or job.

## Evidence

The icon view's constructor writes 40.0 to the view's field-of-view field
(the character view writes 30 to the same field) and 10 to the next. The
icon render writes (0, 0, 500) to the camera node and the origin to its
target, then calls the engine's frame-the-scene routine with 70, 30 or 20.
That routine, read from the assembly: the larger cross half (an ordered
below-compare picks the second, with depth weight 0.5 instead of 1.0),
bent while the framing is above it by `F·((r·0.11 + 1) − 1/(8r + 1))`
(`1/(8r + 1)` in single precision, the rest in double), divided by
`tan(fov·π/360)` in double, plus the weighted depth half in single
precision; the camera moves that far from the box centre along its
direction. The extents helper widens by 0.0001 in the original's
centimetres. The acquire's key (lit bit 1, effect bit `0x2000`, streamed
bit) matches the instructions.

## Out of scope

The subtype subdivision pass, the asynchronous content-stream icon path,
re-rendering cached icons on a GUI-scale change, releasing icon references
when a panel closes, the item cards, popup and parchment forms and the
character view's 3-D figure. The sun direction formula and the screen
pass's sampler stay labelled guesses.

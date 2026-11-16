## Why

Side note of the terraincheck lane, checked by lane `terrainrel`: on arenao5
the terrain blank-block rule reported 176 camera draws left out while the
play view holds about 30 blocks. With `EXA_TERRAIN_DRAW_DEBUG=1` the
per-camera log names the camera that lists every blank block: the loading
screen's camera (order 10, a `Camera2d`), which stays after the load as the
interface camera. It had Bevy's default render layer, so its orthographic
box (window pixels wide, 2 km deep, about the world origin) took part in the
visibility check: every world mesh inside it was listed in its view and
marked as seen.

## Evidence

- Worker runs (arenao3/arenao5, free and play cameras): the order-10 camera
  listed all 176 blank blocks and their plant chunks in every scene; the
  game camera only those in its frustum (0 in the play view, 93-176 in the
  free views).
- The original draws its interface without walking the scene; its world
  draws are per-renderer hooks of the scene, and the interface is no scene
  renderer. Nothing of the world is culled for it.
- Consequence beyond the count: anything reading a mesh's view visibility
  took every mesh in that box as seen: the terrain composite selection's
  visible bit for blocks the live scene does not hold (an outdoor arena's),
  so hidden in-reach blocks ranked as visible, and the cut-away detector's
  "seen" test.

## What Changes

- The interface camera is named `_InterfaceCamera` and belongs to no render
  layer (`RenderLayers::none()`); the interface (Bevy UI) ignores render
  layers and draws as before.

## Impact

- `exa_ui::loading`.
- On screen: no change to the interface. The terrain selection ranks hidden
  in-reach blocks as hidden (by their light) as the original; with more
  blocks in reach than sets, the visible ones take the sets first.
- CPU: one camera's visibility pass over every mesh less per frame.
- Lockstep traces unchanged (draw side only).

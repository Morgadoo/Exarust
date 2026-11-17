## Why

Roadmap task B4 (lane `worlddraw`, triage lane L08 "world draw: terrain,
grids, weather"): the world-draw rows the open ledger still held. The
automatic sun ignored the grid's rooms, a fire lit only its host's first
part, every flame row took the first row's fade, and the camera's wall
probe asked the meshes the view drew instead of those of the sectors it
saw.

## Evidence

REA (Exanima.exe 0.9.5), the assembly where order and constants matter:

- The scene environment's nodes-added slot: in automatic mode a sun only
  when the scene has no lights and no sectors (grid rooms); more than one
  room deletes it.
- The particle system's emissive pass writes the scale to the host node's
  first sub-object material and then to the first material of every child
  whose kind is geometry.
- The flame row's draw sets `Anim` from its own fade before its draw.
- The camera's wall probe walks the scene's sectors whose word has the
  main-view bit, then their meshes.

## What Changes

- The level load's record/automatic sun runs the environment's automatic
  rule with the grid's room count.
- A fire host remembers its geometry children's first parts and scales
  their emissive with the host's.
- Each flame row draws with its own copy of `_FlameMat01`.
- The wall probe asks an obstacle mesh when its sector was seen by the main
  view (a mesh the live scene does not hold always), and never a hidden one.

## Impact

- Code: `exa_world` (environment, lib, fire, scene_live), `exa_game`
  (item_ability, player).
- Pictures: a fire's host children glow with it; two burning blades fade
  each on its own; a grid level without lights and with rooms gets no sun.

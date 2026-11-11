## Why

PLAN Phase 6 (thaumaturgy effects), SG_Dynamics_Effects: the level portal
effects were ported as rules (the marker's light, the dissolve seeding, the
portal effect's ring spawn) but nothing drew a portal particle. A level
exit with the portal bit queued `Step::PortalUse` with no executor, the
seeding and the burst start were never called, and the marker's
`_PortalPrt` sparks were not simulated.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The level change's portal bit runs the portal use on the player before
  the AI hand-over and the transition. The use builds a node with a
  particle system from the dissolve template, renames it `_PrtlDisp`,
  hangs it under the body's parent, stores the pull −0.015, empties the
  ring and seeds it over the body's model nodes around the body's box
  centre, measures the system's box, clears the state word's `0x2` bit,
  and plays `portaluse01` (loaded once) at the body at 0.6, pitch 1.
- The seeding walks a node then its children: `round(half / 6)` steps a
  side (none for a negative count), two cells in five, life less up to a
  quarter, a 5-bit look, the cell through the node's two frames, the
  position moved halfway to the previous, jittered ±1.5 cm (double) and
  pushed by the pull from the centre; it stops at the ring's capacity.
- The two templates of the unit: the marker's sparks (140/s, 1.5 s, 36 cm,
  16 cm/s, retention 3, growth 1, stretch 1.5, spread 1, pool 2048, flags
  `0x4400_2B01`, six blue keys, light (0.49, 0.69, 1) 16/900/0.25,
  `Portal01` at 0.1) and the dissolve (60/s, 0.5 s, 36 cm, 48 cm/s, no
  loss, growth 1, stretch 2, spread 1, pool 2048, flags `0x4000_2B11`, six
  keys, no light, no sound). Their keys are not in the fires' load-time
  scaling list.
- `_PrtlPrt01` (built once per scene from its 240-byte record): colour 1,
  blend word 0x101 (ONE, ONE), flags `0x0040_0008`, texture `PrtDot02.rfi`.
- The marker import (kind `0x3dd0_e004`, in a scene whose flags carry
  `0x10000`) builds the portal effect's system at the marker's place with
  the sparks template at scale 1 and the light as its child 50 cm up. The
  portal effect's spawn moves the ring index on first; the word the
  earlier port read as a "closing state" is the ring's head (the full
  check of the ring).

## What changes

- `exa_world::particles`: `SpawnRule::PortalRing` (the portal effect's
  spawn in the emitter's spawn loop) and `Emitter::seed` (the seeding
  pass's ring write, box and bit clear); `state::RING`.
- `exa_world::flame`: a flame may name any texture and the additive blend
  (`Flame::additive`), loaded on first use; `Flame::emitter_mut`.
- `exa_render::flame`: `FlameMaterial::additive` (its own pipeline; alpha
  written as zero under the premultiplied blend, so ONE, ONE).
- `exa_physics::body::Part::previous_transform`.
- `exa_game::effects::portal`: the `SPARKS` and `DISSOLVE` templates; the
  marker's spark system with its light as child; `PortalUse` and
  `use_portals` (the seeding over the body's parts, the system, the
  sound); `--debug-portal-use FRAME`. The old `ParticleRing`,
  `start_dissolve`, `burst_bounds` and `effects::portal_effect` are
  replaced by the emitter's own.
- `flow`: `Step::PortalUse` writes `PortalUse` for the player.
- `zone_scripts`: `--debug-exit FRAME` crosses the level's first portal
  exit.

## Not changed

- The template light of the sparks is not driven (the import's 0.75 over
  500 cm light stays as it was).
- The body's model nodes are taken as its physics parts with their
  spheres' boxes [guess]; the dissolve's own spawn runs at the level's
  origin [guess: the node hangs unplaced under the body's parent].
- `Step::AiHandover` still has no executor.

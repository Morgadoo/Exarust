## Why

The original can move a scene node along a keyframe track: a node
sub-chunk holds a Catmull-Rom curve of positions, which the scene plays
on the node's translation once the object is used. Exarust skips the
sub-chunk, reads sub-chunks only after mesh bodies, and has no way to
play a node's curve, so the one object that uses it, the secret switch
of `exanima03` (a brick that sinks into the wall when pressed), can never
move.

Phase 2 (Static world), milestone M2 "walkable world": master-plan change
M20 of lane B (animation). Small and optional: one track ships.

## What Changes

- Sub-chunks after every stored node's body, whatever its kind, as the
  original reads them: after a light's body (40 bytes, 8 more for a spot,
  16 more under its body flags `0x30000`), a voxel light's (32), node kind
  `0x3D08`'s (8) and a plain node's (none), not only after a mesh's. The
  GI points stay a mesh's; rigid bodies and cloth markers are read where
  they are (only on meshes in the data).
- Keyframe tracks (`0x3AC1`): a skipped word, the play mode (0 free,
  1 once, 2 reverse, 3 loop; any other word free), the duration (s), the
  last key's index, then the keys (time in s, position in cm, stored in
  metres). A node keeps each track it carries; one that does not decode
  keeps its error.
- Playing them, as the original's scene does: a start (the original's
  use of an operative object starts every track of the object's nodes)
  puts each track not already playing on the playing list with the
  clock's time and swaps once and reverse, so the next start plays it
  back the other way. Every frame each playing track's clock gives a
  curve time (free as it is, once clamped at the duration, reverse
  counted down from it, loop wrapped by it), and the node's translation
  takes the curve's position there: the cubic Hermite curve of the
  bracketing keys, each inner key's slope the chord of its neighbours
  scaled by the segment's share of their span, no slope leaving the first
  key, the last segment arriving along its own chord; the first or last
  key's position outside them. Once and reverse end at their limit and
  leave the list. The clock is the game's frame clock (Bevy's virtual
  time, still while paused), not the body tick's.
- The node entities `exa_world` spawns carry their tracks; a message
  starts the tracks of a node and every node under it.
- Debug helpers: F7 starts every keyframe track of the level, logging
  each start (node, mode, duration, position) and, at debug level, each
  end; `exa-tool corpus tracks` lists every track of every chunk tree.
- Docs: RFC.md (Scene nodes' sub-chunks, Keyframe tracks), DEBUGGING.md
  (F7, Keyframe tracks), PLAN.md (Phase 1 table, Phase 2), crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and its disassembly: the node reader (the
  sub-chunk loop after any kind's body for nodes that are not instances;
  the track's fields and the output pointer to the node's local
  translation), the light and voxel light body readers (their lengths),
  the track's class (its evaluate slot), the clock (modes, the finished
  flag), the curve (basis constants 2, 3, 1, −2 and the 0 and 1 slope
  factors read from `.rdata`), the scene's per-frame pass (time since the
  start in ms × 0.001, removal of ended tracks), the start (skip when
  playing, swap 1 ↔ 2), the recursive start over an object's nodes, and
  the use of an operative object of class 0 that calls it
  (`context/re/master-animation.md` §12 and "M20 findings").
- Corpus: every chunk tree of every installed pack, nested packs included
  (7 653): one track, on `Box001` of `Objects.rpk` `uwa01 brkswtch`; the
  sub-chunk walk ends exactly at every node's end for every kind; no
  rigid body or cloth marker on another kind; the switch's object record
  (operative class 0 kind 1) and its placement in `exanima03`.
- In game: a light run of `exanima03` with F7 shows the brick sunk into
  the wall and back at rest after 3 s of game clock.

Out of scope:
- using objects (picking under the cursor, the interact click, keys,
  containers) and the operative signals a use sends (the switch's
  delayed signal to its target); until then only F7 starts tracks;
- the meaning of the skipped word and of the `0x3D0CEC04` sub-chunk;
- the original's linear position controller, which nothing builds.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `motion-playback`: node keyframe tracks decoded, their curve, and
  playing them when started.
- `asset-inspection-cli`: `exa-tool corpus tracks`.

## Impact

- `exa_formats` (`key_track`: `KeyTrack`, `PlayMode`, `PositionKey`;
  `rfc`: `KEY_TRACK`, `Node::key_tracks`/`tracks`, sub-chunks after every
  kind's body)
- `exa_anim` (`tracks`: `TracksPlugin`, `NodeTracks`, `StartTracks`,
  `PlayingTracks`, `clock`, `position_at`, `swapped`, F7)
- `exa_assets` (`ModelNode::tracks`), `exa_world` (`spawn_model` inserts
  `NodeTracks`; depends on `exa_anim`); one-line `tracks: Vec::new()` in
  test and weapon model nodes (`exa_world::grid`, `exa_character::blood`,
  `exa_assets::weapon`)
- `exa_tool` (`corpus tracks`), `exarust` (key list)
- `docs/formats/RFC.md`, `docs/DEBUGGING.md`, `docs/PLAN.md`,
  `crates/exa_formats/README.md`, `crates/exa_anim/README.md`,
  `crates/exa_world/README.md`, `crates/exa_tool/README.md`

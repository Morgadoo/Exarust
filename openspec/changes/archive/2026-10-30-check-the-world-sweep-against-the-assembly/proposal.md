# Proposal

## Why

The round-44 world sweep lane added rules to eleven small world, level and
terrain components. Checked against the instructions, several were
invented, swapped or duplicated existing Exarust code; the rest needed
corrections before they can reach main.

Phase: Phase 2 (static world: terrain, grid sets, scene bookkeeping) and
Phase 3 (bodies: lever and mechanism signals).

Evidence (REA on the 0.9.5 build and the objdump listing):

- Terrain blend walk: the input is tested with two unordered-aware
  compares; an ordered input at or below the low bound takes the first
  child whole, at or above the high bound the second, and anything else, a
  NaN included, splits (the second child walked first). Only a walk's start
  is gated, the weight widened to double against the double 0.001; the
  remainder loses each painted layer in turn.
- Terrain manager: the record's block node is a single zero-filled 90-byte
  root whose child pointer the terrain unit never sets, so on its own the
  four-way recursive update only refreshes the whole-tile descriptor (no
  writer of the children was found in the unit). The tile hand-off is a GPU
  read-back: renderer feature bit 0x2000 copies the 244² height plane into
  the record, 0x1000 the 240² × 3 packed plane to the manager's callback;
  the record then loses 0x2000 and gains 0x4000. Queuing a bake clears the
  record's 0x100. No release clears all three bits, and no 122-texel
  texture exists.
- Set-piece loader: the marking channel is the model's channel of kind
  0x100 when present; otherwise a record is appended whose first word is the
  first channel's word with 0x4000000, whose second word is 0x2b40100, and
  which copies the first channel's third word and slot count, filled with
  0xFF bytes. A part is marked when its alpha passes and its last triangle
  is not before its first.
- Scene watch pairs and cell lists: the pair add swaps a track's once and
  reverse modes and the step clocks the track (the pair's second object)
  in single precision; the cell lists are the directional light fill and
  its removal. Both are already reproduced (`exa_anim::tracks`,
  `light_flood::CellLights`); the duplicates are removed.
- Lever light: the kind 6 finder starts at the parent of the arm body's
  node (the operative's node), the node first, then each child in order.
- Fire job: a fire at the node's position of the signalled size, added to
  the node's sector; the node's lights are untouched. The light job powers
  the electric object under the node; it scales no light.
- Scene task: the update byte the lane added is the viewport repaint byte
  `TaskTick::repaint` already reports. The environment constructor hook is
  engine lifecycle with no rule to model.

## What Changes

- `exa_world::terrain::blend`: the walk's NaN split, the walk-start gate in
  double and the running remainder.
- `exa_world::terrain::manager`: the invented block tree and upload slot are
  replaced by the read-back hand-off (`queue_readback`, `take_readback`).
- `exa_world::grid`: `added_channel`, `find_channel`, `marks_part`.
- `exa_world::bodies`: the lever light search from the operative's node.
- `exa_world::mechanisms`: a fire job spawns only its fire; light jobs are
  not run.
- Removed: `WatchList`, `CellPairs`, `TaskTick::scene_flag`,
  `ConstructorHook`, `EnvironmentAttach`; the deleted rotated-room test is
  restored.

## Out of scope

- The GPU bake and read-back themselves (Exarust builds tiles on the CPU).
- Electric objects and relay listener lists in the ECS.
- The transparent-alpha compare in double (on the round-43 branch).

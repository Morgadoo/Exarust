## Why

Phase 3 (the simulation thread) and Phase 4 (combat) in docs/PLAN.md, TSimSector. `step_bodies`, the pair stage
(`Pairs::find`/`meet`) and `rebuild_sector_bodies` sorted their bodies by Bevy's `Entity`, whose `Ord` in Bevy 0.20
compares the stored bits (index inverted, generation above): newest first for fresh ids, any order once ids are
recycled, and not the "spawn order" their comments claimed (archived change 2026-11-10-dress-in-spawn-order). Round
154 also found the original's sector step walking its list from the end. This change settles the original's order
against the assembly and makes Exarust follow it with an explicit key.

## What the original does

REA and `exe.asm` of the sector's grid step and all-pairs step, the list appender, the node-tree add, the scene scan,
the node-add hook and its upkeep drain, the purge, the pair key writer:

- **The list.** A sector keeps one list of bodies (every part is a body). The appender puts a body at the end and
  writes its index into the low 16 bits of the body's key word (the order key round(100/mobility) is the high 16
  bits); in a grid sector it also links the body at the head of its cell's list. The node-tree add and the scene scan
  append a node, then walk its children in order (a node before its children). A node added to the scene during
  play is queued by the scene's add hook and appended, tree by tree in queue order, at the simulation's per-sector
  upkeep before the next step. The character builder scans the new character's node tree at once; items put on later
  are queued. The purge (a scene's nodes leaving) compacts the list keeping the order and renumbers the indices.
- **The step.** Both steps walk the list from its end; every part that is awake is integrated and appended to the
  solver's awake list, which the 20 iterations, the cell migration, the detection, the friction and the post-step
  all walk from its start: the last part to join first, a tree's root last.
- **The grid detection.** Each awake part in that order is stamped, then walks its nine cells (own cell first, the
  sector's offset table) and each cell's list from its head, skipping stamped parts; a part changing cells is moved
  to the head of its new cell's list during the migration that precedes the detection. Of the two, the part whose
  key word is smaller (unsigned) brings its spheres: the more mobile one, and on a key tie the one with the lower list
  index, whichever was walked first.
- **The all-pairs detection** (a sector without a grid) walks the list from its start: each awake part against every
  later part, awake or asleep; friction and post-step walk the list itself there.

## What Changes

- `Ragdoll::list_place` (a process-wide count handed out by an `on_insert` hook the first time a body is inserted;
  a body lent across the simulation bridge or copied back keeps it; `runtime_clone` gets a new one), with
  `Ragdoll::list_parts` (the tree's node order) and `body::walk_order` (list from its end; unplaced bodies last).
- `step_bodies` and `rebuild_sector_bodies` sort by `walk_order`; the body grid lists each body's parts root last.
- The pair stage walks each body's parts in reverse list order, meets the parts of each cell in the order the cell's
  list holds them (the last to join first, cells in the original's order), breaks key ties by list place; the
  all-pairs walk follows its list from the start, awake parts against later parts only.
- `limp::build_bodies` and `prop::register_props` insert their bodies in spawn order (entity index), so the places are
  the same in every lockstep run (a run without this gave the two duellists' bodies places 6/7 in one run and 7/6 in
  the other, and the traces split at tick 465).

## Out of scope

- The original moves a part to the head of its new cell's list when it changes cells; Exarust rebuilds its cell
  lists each tick in joining order. Its list index is 16 bits; Exarust compares the whole place.
- A body's own parts are stepped as one `Ragdoll::step`, not interleaved with other bodies' parts.
- The free-running simulation thread can insert bodies while the frame does: their places then follow the clock.

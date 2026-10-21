# Proposal

## Why

Phase 6 (AI, docs/PLAN.md) needs NPC paths, and every NPC path in the
original comes from one planner on the navigation grid. A lane extended
the planner, the dirty-rectangle refresh, the roles unit's script
functions and the search activity's world edits, but ported them largely
from pseudo-code: the planner truncated its heights and heuristic,
measured the heuristic's height leg from the start, read the lower floor
whatever layer a node stood on, counted an end within 100 m as reached and
string-pulled the wrong corners; the refresh compared samples with the
wrong byte and never restored a floor once a mover left; the corpse
scripts and the search's drop did not follow their routines. A second,
unused and disagreeing copy of the planner's pieces sat in `exa_ai`.

## What Changes

- The planner (`exa_world::nav::find_path_with`) follows the original's
  instructions: single-precision cell rounding ties to even, heights and
  heuristic rounded, the height leg to the goal's height, the node's own
  layer floor, the blocked window border, the margin on every grid, the
  accept test after the node cap, unsigned compares, the 100 cm rule and
  the rough cut, the corner chain and the budget line walk (far corner's
  floor held, every cell to the kept point). Buffers are kept per thread.
- The refresh starts each rectangle from the floors as built, which the
  grid now keeps (the straight walk reads them too); samples go to the
  upper floor at or under the lower slab's underside; the sample voxel
  carries the half voxel; the ring takes rectangles during the scene-wide
  countdown; a moved box's rectangle has no half-cell shift.
- `exa_ai::path`'s unused planner pieces are removed.
- Corpse scripts take each role's first Player entry; the dialogue wrapper
  is dropped (the per-role lookup is `Role::dialogue_for`).
- The search's worn take checks the corpse's grip first; wearing puts the
  item on before taking it; dropping waits on a locked pack and compares
  the item's size with the container's largest item.
- Debug: `exarust --debug-path x,y,z` plans from the player and draws it.
- A corpus check times a long plan on each shipped level (under 1.1 ms).

Evidence: reverse engineering of `Exanima.exe` 0.9.5 through REA (the
planner, its line test, the squared-distance helper, the window reset,
the refresh, the dirty-rectangle and ring slots, the roles unit's corpse,
dialogue, relation-removal and script-registration routines, the corpse
handler in the player's controls, and the search activity's take, wear,
carry and drop executors), read at the instruction level where floats or
compares matter; the installed levels for the timing.

Out of scope:

- NPCs planning and following paths in the running game (chase and flee
  belong to the combat state, another lane), the one-plan-a-tick slot;
- props marking rectangles and supplying samples (props are static);
- running the corpse, condition and `StartDialogue` scripts;
- the search ticking in the running game (no item instances there).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `navigation-grid`: the planner's rules, the refresh from built floors;
  the pseudo-code planner pieces are removed.
- `npc-roles`: corpse scripts, the interaction condition, the script
  registration, the level relation removal.
- `npc-search`: the deeds' world edits.
- `world-viewer`: `--debug-path`.

## Impact

`exa_world` (`nav::path`, `nav::mover`, `nav::query`, `NavGrid`),
`exa_ai` (`path`, `roles`, `search`), `exa_game` (`path_probe`),
`exarust` (`--debug-path`), `docs/NAVIGATION.md`, `docs/DEBUGGING.md`,
crate READMEs.

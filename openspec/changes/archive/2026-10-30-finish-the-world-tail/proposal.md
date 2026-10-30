# Proposal

## Why

A second pass over the world tail (simulation, scene, scene editor,
terrain manager and grid loader) closed rules the first left open or got
wrong: the retention conversion took a NaN down the logarithm branch, the
terrain block descriptor clamped a flat block to an invented 1 mm, the
scene's sector creation replaced a shared sector's node, the grid loader's
transparent-alpha test compared in single precision, and the run-scene
set-up had no default camera record or listener step. The review against
the machine code also removed two invented rules of the pass itself (a
copied-bounds attach for the terrain kd-tree and a transform-refresh call
site for the scene's matrix snapshot) and corrected its sector creation.

Phase 3 (Bodies: skeleton, physics, control) for the simulation, and
Phase 2 (Static world) for the scene, editor, terrain manager and grid
loader.

## What Changes

- `exa_physics::dynamics`: the retention conversion's ordered `> 0`
  test (a NaN stores zero); the constructor's scale, backlog and
  environment-count words; sector creation that hands a new sector's
  children back for immediate registration and queues them as pending in
  a shared first sector, for every node of the batch.
- `exa_physics`: the capsule world's gravity is the body solver's
  −9.80665 m/s² constant; the controller base's false and float
  pass-through slots.
- `exa_world::terrain::manager`: the block descriptor's vertical half
  extent is the raw half plus 1 cm; the descriptor walk over a block tree.
- `exa_world::grid`: the alpha test in double, the set link walk and the
  top-level `+F` piece selection; set entries keep the name's case.
- `exa_world::scene_nodes`: the matrix snapshot callback and the scene and
  sector destroy state.
- `exa_game::scene_editor`: the default seven-word camera record, the
  physics, maximise and view-toggle option bytes and the listener step.

## Out of scope

- Live wiring: the scene, simulation sectors, grid link walk and scene
  editor stay tested models; the capsule world's gravity is the only
  change the running game sees (0.03 % weaker).
- The sector's per-frame node pass that calls the snapshot callback, the
  reuse of a simulation object the scene node already carries and the
  node add hook a shared scene installs.
- The six shipped environment preset records and the block tree's
  construction.

## Evidence

The original's machine code (objdump listing and REA) for the
environment conversion, the simulation constructor and its sector slot
and node hook, the block descriptor and its walk, the terrain kd-tree
attach, the grid loader's alpha test and link walk, the scene constructor
and its frame pass, and the run-scene set-up and task tick; data words
read from the executable (the default environment's −980.665 gravity, the
1.0 scale word, the 0.5 and 1.0 block constants, the f64 0.01 alpha bound
and the seven-word camera record). All tests use synthetic data.

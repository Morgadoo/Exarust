# Proposal

## Why

A level's corpse (a placement with condition 0xA00) was spawned standing at
its placement and simulated until it collapsed, falling into the furniture
it should lie on, and a stand-in marker kept that fall out of limb impacts.
The original's corpse builder never does this: it puts every body part at
the world frame saved with the level and registers the bodies so they run
one tick and sleep.

## What Changes

- `exa_formats::characters` keeps a placement's saved pose as a bone tree
  (frame, word, child count per record) or the legacy flat frames, the tree
  preferred whatever order the chunks come in.
- `exa_character::pose::place` puts the pose on a body model: the bone tree
  through `Body::restore_tree_pose` (nearest child rest offset within 20 cm,
  the threshold now in the tree's metres; an unmatched record skips its
  subtree), the flat frames through `Body::restore_flat_pose` by the saved
  part table (slots 1–18), else the built pose. Both restores re-derive only
  the local rows, as the original's do; the character frame's translation is
  the restored root's world origin.
- `exarust` spawns a level's dead from that pose: same model, outfit, hands
  and armour as the living, condition dead, no animator, every part on its
  saved world frame. `exa_physics::body::Ragdoll::register_placed` builds it
  limp with zero velocity and countdown 0, so it sleeps in its first tick's
  post-step and is in deep sleep from the next. The placement's position and
  yaw are not used.
- `exa_combat::impact::LayingOut` and `finish_laying_out` are removed; the
  single-placed-body pair gate in `exa_physics` stays.
- `--corpse` keeps the built pose and the limp fall (it has no saved pose).
- The solver post-steps a sleeper a pair contact woke in the same tick, as
  the original's solver list does: without it a corpse at countdown 1 woke
  the furniture it lies on, whose push then read as speed (256), and the
  two fought for seconds with limb impacts.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-generator`: the saved-pose restores, their units and the flat
  slot table.
- `physical-body`: placed corpses registered at rest; woken sleepers take
  the tick's post-step.
- `world-viewer`: `--corpse` is the one dead body that falls.

## Impact

- `exa_formats::characters`, `exa_character::{pose, runtime}`,
  `exa_physics::body`, `exa_game::limp`, `exa_anim` (`SkinnedRoot`),
  `exa_world`, `exa_combat::impact`, `exarust::viewer`; crate READMEs,
  `docs/formats/RFC.md`, `docs/DEBUGGING.md`.
- Evidence: the original's corpse builder (model, stance, equip, motion 3,
  condition 0xA00, muscles off, tree-before-flat, frame translation from the
  root node's world origin, registration), its bone-tree restore (squared
  distance against 400 cm², strict, first nearest kept; the helper writes
  the three local rows only) and its flat restore (root local = world, the
  others' rows only) and the sector step's post-step loop over the solver
  list after detection, read from the instruction listing; the shipped
  `exanima01` (bone tree) and `exanima05_sw` (flat) corpses.
- Out of scope: the retail runtime part order for flat poses (the saved
  table is used), the corpse's stored motion and the dead body's character
  scale word, saved poses on living placements (none ship), revive of a
  placed corpse (no caller in Exarust).

## Phase

Phase 3, Bodies: placed corpses from their saved poses (docs/PLAN.md, the
character generator's saved poses and the body sleep paragraphs).

# Proposal

## Why

TMotileAI was rated 61%. The brain ran in the game, but many of its
routines had no Rust code: the room bookkeeping, the out-of-combat turn,
the role re-application, the ally-class sense, the idle driver with its
go-to-object state, the head look, the gait, the walk and the alert look,
the lifecycle hooks and the reset given when a level is written out. The
motileai2 lane added them. This review checked them against the
instruction listing and fixed what differed.

## What Changes

- The AI tick links each brain into its 600 cm room cell (floor helper,
  half-offset flat index, nothing done off the grid), gathers the cell's
  characters as its nearby list, decays the group alert every 10 s, and
  turns an idle brain's model toward its eased yaw. That yaw is the
  out-of-combat turn: a sway of up to ±1.8 rad drawn every 4 to 8 s, eased
  by `0.09 − 0.06·encumbrance`, in double precision. The review fixed the
  model snapping to the bare look.
- Roles are applied again whenever the held set changes.
- The reset the original gives every character when the level is written
  out (heal to the lasting pool, forget contacts, put a roaming AI down
  along fresh wander paths and make it leave combat) is a tested function.
  It skips collapsed, dead and dormant bodies. The lane had wired it to
  entering the scene, which healed NPCs and made duellists leave combat;
  the review removed that wiring.
- The ally-class sense is rewritten from the instructions. It floors the
  concern at 3, gives a fellow AI attitude 100, uses a range of 1200 cm
  (1800 cm while tracked) and the view term, and weighs the threat to an
  ally by the observed state. A threat above the concern sets the concern
  target to 0.3 of it.
- New tested routines: the head look, look smoothing, the walk and its
  stop, the steer line, the way share, the body in the way, the alert
  look, the aim, settle and snap, the idle driver and go-to-object state
  (the last usable object wins), the investigation, the room walk, the
  object goal, the path's flat length, and the script, removal, destroy
  and disable hooks.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-perception`: the ally-class sense and the AI tick's room cell,
  alert decay and idle turn.
- `npc-idle`: the level-leave reset and the idle object search.

## Impact

- `exa_ai::{brain, motion, rooms, lifecycle, presence, combatant, route}`,
  `crates/exa_ai/README.md`.
- Evidence: the AI tick, the out-of-combat turn, the room bookkeeping and
  its floor helper, the ally sense, the object search and the reset
  reached through the level export's per-character call, all read from
  the instruction listing of `Exanima.exe`.
- Out of scope: the level's cell-to-room table and room exits (each cell
  stands for a room), a level write on leaving (so the reset has no
  caller), an NPC locomotor (the head look, gait, walk and alert look are
  not wired), the ally sense's call from the perception pass, the scene
  node half of presence, and the combat state's remaining gaps.

## Phase

Phase 6, AI: the NPC brain (docs/PLAN.md).

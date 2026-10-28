# Proposal

## Why

The scene-node unit's second pass (lane `scenenodes2`) added light aliases,
per-alias bucket lists, light activation, the membership rebuild, the
frame order, the material queues and the unit's scene list to the
`exa_world::scene_nodes` model (docs/PLAN.md Phase 2, static world: the
scene graph the room view will draw from). A review against the assembly
found several rules ported from the pseudo-code or invented, and fixed
them.

## What Changes

- The membership rebuild follows the original: aliases are owned by a
  sector, a removed processed alias's slot is filled from the end (the
  order is not kept), the bucket chains are remapped and the waiting
  aliases linked, a dropped mesh gives its bucket faces to the aliases'
  shadow maps, and the removed alias's owner (not the scene) is marked.
- Light activation snapshots the light's position (height zeroed in the
  flat mode), keeps the light's own flag word apart from the option word,
  starts a new alias moved, stores the light's cell and reports the flood
  only in grid mode (an alias relink otherwise), and drops a departed
  light's aliases.
- The node passes treat the state word's "just moved" value as the moved
  marker, relink every moved bucket mesh, link faces for every seen bucket
  mesh, report moved aliases and moved parts as a change, walk only the
  processed meshes, and keep the shadow map's needed and linked faces
  apart.
- The fade's fall-back branch sets the dithered bit without testing the
  main bit; the frame delta is read unsigned.
- The doorway distance and the portal facing sum `(x + y) + z`.
- Alias removal and subtree moves mark the alias's and the old sector, not
  the scene's own sector.
- A world light clears only the first part of each mesh child.
- The bucket list's growth is zero-filled and an empty list keeps its head.
- The scene list frees, at cleanup, only scenes released while they had an
  owner and not marked owned.

## Evidence

The assembly listing of the scene-node unit (the membership rebuild, light
activation, both node passes, the doorway test, the frame and the scene
list routines) and the light alias's shadow-face hand-off, read
instruction by instruction.

## Out of scope

The draw path (the renderer still draws from the room states), the alias
shadow test's own body and relink, the light flood invocation and flicker,
and the unit's leftover-node list.

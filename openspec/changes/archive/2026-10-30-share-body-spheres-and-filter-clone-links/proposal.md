# Proposal

## Why

The tetrahedral body's clone slot was the largest TTetraRBD routine Exarust did not reproduce: it shares the collision-sphere array with its template (copying it only before a write) and copies only the links that are neither run-time code links nor breakable ones. Exarust's `Clone` copied every link and every sphere list.

## What Changes

- A part's collision spheres are one shared array behind a body and its copies.
- New: `Ragdoll::runtime_clone`, the clone slot: links carrying the code or breakable flag are left behind (which drops every origin pivot), the kept links' indices are remapped (a leg's limit link follows its link) and each part's link range keeps its surviving links.
- `Clone` stays a whole-state copy, sharing the sphere array the same way; its part link ranges are carried over unchanged.

## Capabilities

### Modified Capabilities

- `physical-body`: run-time body clones and the shared sphere array.

## Impact

- `exa_physics::body` (and the sphere readers in `exa_physics`, `exa_game`, `exa_combat` tests); the `exa_physics` README.
- Evidence: the clone routine's assembly (the `and eax, 0x401000` link-flag test before each appended record, the sphere count and pointer copied rather than duplicated) and the copy-on-write routine that allocates `count × 16` bytes and moves the template's spheres only when the owner is another body.
- Out of scope: the clone's link-point rewrite for node-linked bodies (Exarust's links carry no node frame), the world links' sound sources, a run-time caller (the original's callers are the push surface, rotator, door and motile part builders, which Exarust does not clone), and the weighted point sums' pairwise order in `Part::point`.

## Phase

Phase 3, bodies and physics: the tetrahedral rigid body (docs/PLAN.md).

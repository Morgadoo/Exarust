# `exa_ai`

NPC behaviour: perception, path finding on the level grid, combat AI that drives the same character controls as the player.

**Phase:** Phase 6 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

It already drives the play mode duellist: from the fight start it enters
combat in its guard, guards at four reaches or more from its target and
stands in Cmbt - Idle within, walks in with Exarust's interim combat walk
and strikes at intervals, each strike through the original's attack
executor (`exa_combat::attack`: enough energy, the cooldown since its last
attack), holding every swing through (it never releases one, so its
swings stall only when struck); it can be told to hold its ground (a
debug aid).

## Scope

- Navigation over tile grids and terrain; activity/idle schedules.
- Combat decision making producing control inputs (movement, attack direction, stance) for the physics character.
- Companions/hirelings and arena opponents.

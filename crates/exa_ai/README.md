# `exa_ai`

NPC behaviour: perception, path finding on the level grid, combat AI that drives the same character controls as the player.

**Phase:** Phase 6 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: partial.

It already drives the play mode duellist: from the fight start it enters
combat in its guard, guards at four reaches or more from its target and
stands in Cmbt - Idle within, walks in with Exarust's interim combat walk
and strikes at intervals, each strike through the original's attack
executor (`exa_combat::attack`: enough energy, the cooldown since its last
attack), holding every swing through (it never releases one, so its
swings stall only when struck); it can be told to hold its ground (a
debug aid).

| module | what it holds |
|---|---|
| `roles` | role lists (adding with inherited parents, newest first and counted; removing explicit roles), the new story character's and the arena fighter's roles, relations (global and per locale, relation roles with values, familiarity, the last attitude, a former level character's move), the attitude of one character toward another, encounter scripts and the entry finder |
| `activity` | applying roles: the init scripts' assignments (newest role first), the mind-control follow, the idle state per activity, the idle-activity registry (the search as 6) |
| `search` | the companion's search: gathering over the room grid, picking, the look sweep, the forced path, arrival, kneeling, looting worn items, packs and containers, item worth, wear room, comments to the leader, re-arming and tidying |
| `manager` | the AI manager's frame (dormant ticks, rising undead, the 3 cm zone check), zone deeds, the group alert and the approach clamp |
| `idle` | roam, patrol, sentry and the wait, the wander and home-side points, follow, sleep, knock-out and waking |
| `sense`, `hear`, `fight`, `place`, `path`, `order` | contact rating, noise and alert, attack choice, combat positions, path search and following, steering orders |

**Roles and attitude.** Adding a role adds it, or marks it explicit, and
while the role just handled inherits (flag `0x10000`) its parent: a parent
held already counts one more, a new one comes in as inherited; the new
roles go in front. Removing an explicit role drops it (even when others
also counted it) and counts its parents down, dropping them under 2. The
attitude of A toward B starts at A's familiarity with B. A's relation roles
toward B decide first (a relation role's entry for the partner); then A's
roles newest first, each entry in order, matching an entry for one
character in its locale, or one of A's relation roles toward B, or one of
B's roles. The first match gives `attitude · w + (1 − w) · familiarity`
(w the role's weight share) and its tolerance; none gives the familiarity
and tolerance 0. Shipped matrix (familiarity 0): zombie → human −20,
skeleton → human −50, demon → human −100, demon → demon +100 (tolerance
60), skeleton → golem +30, human → demon −50.

Not in yet: a script engine (encounter, dialogue and condition scripts are
found, not run; only the init assignments run), the mind powers' attitude
overrides and the perception's group blend; the roam and patrol walk only
once the running game has a navigation grid, and the search has no item
instances, containers or corpses to act on there.

## Scope

- Navigation over tile grids and terrain; activity/idle schedules.
- Combat decision making producing control inputs (movement, attack direction, stance) for the physics character.
- Companions/hirelings and arena opponents.

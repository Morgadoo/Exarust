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

A level's living characters carry a brain (`brain`): their record's roles,
their relations and traits. When they appear their roles choose their idle
activity; every frame they rate the player from both sides' roles, which
feeds the contact rating (`sense`), and a human-framed character whose
rating engages duels the player. A sleeper that rates the player above a
notice drops the sleep role and wakes.

| module | what it holds |
|---|---|
| `roles` | role lists (adding with inherited parents, newest first and counted; removing explicit roles), the new story character's and the arena fighter's roles, relations (global and per locale, relation roles with values, familiarity, the last attitude, a former level character's move), the attitude of one character toward another, encounter scripts and the entry finder |
| `personality` | what the brain derives from the record (skill, confidence, restraint, courage, eagerness, blend, jitter, arousal) and the attach step (contact sense, dead and statue cases) |
| `activity` | applying roles: the init scripts' assignments (newest role first), the mind-control follow, the idle state per activity, the idle-activity registry (the search as 6) |
| `search` | the companion's search: gathering over the room grid, picking, the look sweep, the forced path, arrival, kneeling, looting worn items, packs and containers, item worth, wear room, comments to the leader, re-arming and tidying |
| `manager` | the AI manager's frame (dormant ticks, rising undead, the 3 cm zone check), zone deeds, the group alert and the approach clamp |
| `brain` | the components and systems that carry roles into the running game |
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
## The combatant

`combatant` is the shared base of the original's character behaviours: one
contact per character it has met (`Contact`: its level, attitude,
tolerance, hostility, concern and concern target, alarm, closeness, facing,
threat and death time), the references to its combat target, look target and
holder, its published combat rating and reach, its courage, its alert level
and its nav footprint.

`perceive` is one pass of one character looking at another. It measures the
distance with vertical offsets under 150 cm ignored, halves the 1200 cm
detection radius while asleep and adds 600 cm for a tracked contact; a
hidden contact's radius is scaled by the observer's perception through the
0.9 and 0.8 factors (or a masking power's two factors). It publishes
closeness (at most 0.99), noticeability (3·c² dead, 24·c down, the
look/forward/closeness product otherwise, +10 while attacking), the
observer's view weight, the facing term, the bearing, the group-blended
attitude and the threat estimate. The threat weights each of the other's
contacts by trust and by how that contact faces it, at 32 while the other
attacks, 16 or 8 in or out of combat, 2 down, and ×4 for a fast other; the
largest third-party weight and the most negative one give the threat and
the crowd penalty, published back onto the observer's contact. A lost
tracked target raises the observer's alert to 40.

`effective_attitude` and `trust` apply the mind powers to a raw role
attitude: kind 0x105 sets it to half the negated aggression toward everyone;
loyalty or command sourced on the other makes it 100, or averages it with
100 under the resistance technique; the aimed power makes it 100; the
suspect power takes 50, not below −100 (and trust ignores that one).
`add_concern` is the script's `AddConcern`: the attention-weighted rise,
capped at 15, with its event bits at 1.2 and 4. `look_direction` holds the
look within 45° of the body; `candidate_cost` is the footwork step's
pathfinder cost; `outside_radius` and `clamp_step` are the carry and move
clamps; `foot_cell` and `stamp_footprint` are the 30 cm nav footprint with
the fallback cell and its once-only overflow warning. `forget`, `replace`
and `reset` clear or remap every reference when a character dies or is
swapped.

`place` is the combat position score against one opponent: the close band
at its reach plus 150 cm, the strike ideal at the observer's reach plus a
size correction, a step's plateau and a fade to 600 cm, the danger of a live
side swing by its time to impact (4 at the edge, 1 by 900 ms, nothing by
10 s) and of a live thrust (1 or 8 by its arc), the banded approach term,
the opponent's facing and attack state, the press flag and the feral and
skill scales. `publish_arc` is the opponent's published sector and its
recovering kind; `blow_is_live` its is-attacking test; `between` the
original's arc test; `arrival_ms` its time to impact. `choose_footwork`
turns the terms into hold, approach, retreat or a circle out of a live
swing, and the duellist runs the chosen footwork (with a short hold timer
standing in for the original combat state's switching cost) before it
takes its stance and strikes.

Not yet: the role database's raw attitude and tolerance at run time (both
are inputs), the encounter scripts and familiarity writes, the mind
powers' own effects, the target selection and morale rules, the full
eight-candidate footwork search, and the auto-parry.


## Scope

- Navigation over tile grids and terrain; activity/idle schedules.
- Combat decision making producing control inputs (movement, attack direction, stance) for the physics character.
- Companions/hirelings and arena opponents.

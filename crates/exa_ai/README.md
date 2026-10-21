# `exa_ai`

NPC behaviour: perception, path finding on the level grid, combat AI that drives the same character controls as the player.

**Phase:** Phase 6 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: partial.

It already drives the play mode duellist with the original's combat
state (`combat`), once per body tick: it enters combat in its guard with
its eagerness at a half, and each tick weighs up to seventeen places
(where it stands, eight steps and eight long steps) through the position
score (`place`, with the foe's published arc), keeping its contact with
the target through the combatant's perception pass (`combatant`); it
steps to the cheapest, strikes from the cheapest strike place through the original's attack choice (`fight`) and executor
(`exa_combat::attack`), guards in Cmbt - Idle when fresh and in the
Defence stance when tired, and turns its heading 0.03 of the gap a tick
(by the swing's sweep while it swings, leaning against a foe's side
swing on guard). Its eagerness drifts up at full energy and its press
comes within 3 s, so it hesitates before it closes. Exarust walks a
chosen step in its interim combat walk until it arrives; the duellist
does not release a swing yet; it can be told to hold its ground (a debug
aid).

A level's living characters carry a brain (`brain`): their record's roles,
their relations and traits. When they appear their roles choose their idle
activity; every frame they rate the player from both sides' roles, which
feeds the contact rating (`sense`), and a human-framed character whose
rating engages duels the player. A sleeper that rates the player above a
notice drops the sleep role and wakes.

| module | what it holds |
|---|---|
| `roles` | role lists (the shared `exa_formats::roles::HeldRole`; adding with inherited parents, newest first and counted; removing explicit roles), the new story character's and the arena fighter's roles, relations (global and per locale, relation roles with values, familiarity, the last attitude, a former level character's move), the attitude of one character toward another, encounter scripts, corpse scripts (each role's first entry aimed at the Player), the interaction-condition wrapper, the entry finder, and the `StartDialogue`/`ChangeRole` registration |
| `personality` | what the brain derives from the record (skill, confidence, restraint, courage, eagerness, blend, jitter, arousal) and the attach step (contact sense, dead and statue cases) |
| `activity` | applying roles: the init scripts run through `exa_script`'s interpreter (newest role first, with the role's own variables), the mind-control follow, the idle state per activity, the idle-activity registry (the search as 6) |
| `search` | the companion's search: gathering over the room grid, picking, the look sweep, the forced path, arrival, kneeling, looting worn items, packs and containers, item worth, wear room, comments to the leader, re-arming, tidying, and the world edits its deeds carry out (`apply_deed`) |
| `manager` | the AI manager's frame (dormant ticks, rising undead, the 3 cm zone check), zone deeds, the group alert and the approach clamp |
| `brain` | the components and systems that carry roles into the running game |
| `idle` | roam, patrol, sentry and the wait, the wander and home-side points, follow, sleep, knock-out and waking |
| `combatant` | the behaviour base: contacts, perception, the attitude and trust overrides, concern, references, the queued step, the nav footprint |
| `combat` | the combat state: the follow check, practice, the foe estimate, the weapon draw and the lunge, the chase count and eagerness, the footwork drives, the seventeen-place search, the attack decision with the friend test and the keep-off, the step, the guard, the fierce stance and the body turn |
| `contacts` | the contact classifier (the stress flight, the engagement threshold, the concern flight, the watch and the notice score), the twelve-contact list, the target pass's choice, the alarm mean and the sleeper's waking |
| `flee` | the flee start's morale, the way out of a room away from a threat, the fled-room memory, the flee state's tick and the cornered fall-back |
| `sense`, `hear`, `fight`, `place`, `path`, `order` | contact rating, noise and alert, attack choice, combat positions, path following (the planner is `exa_world::nav::path`), steering orders |

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

**Combat state.** Each tick: practice from struck foes pays four per
point past a gain of 10; with probability `(0.8k² + 0.2)²` the foe is
looked at again, blended back by `0.6 − 0.6k²`; the foe's contact gives
the level; engaged and balanced, a far, high or unseen foe counts the
fighter stuck (a chase after 21 ticks) and the eagerness drifts by
`(3 − k)(0.02·draw − 0.006)` up to 6 at full energy. The attack, strike
and spacing drives weigh the places: each costs its ground, the change of
step, three times each contact's danger and twice its closeness, less the
strike drive times the foe's strike term; a place whose approach times
the attack drive (less half its danger times the spacing) passes 1 can be
struck from. A strike wins by 2 (1 committed) when no friend stands in
the way; a forward strike place gives way to where it stands; otherwise
it steps only to a place cheaper by 1. Arithmetic keeps the original's
order and its double-precision comparisons. The position score's ground
cost, line walk, sight test and nav snap have defaults (no navigation
queries in the running game), and the chase route, the follow check and
casting are hooks with no answer yet.

**Contacts and flight.** A contact flees first when the stress
(base + stress − courage) passes 3 with an alarm above 0.75 and a way out;
then it engages past `7b² + (2 + b)` (`2 + b` for b ≤ 0) plus the
stress; then a concern above 1 flees past 8 less the stress, or watches;
else it is noticed. A flight halves the courage and adds 2 to the stress;
a cornered fighter fights on with its courage doubled. The way out is the
open exit most opposite the threat (cosine under 0.2), and fled rooms are
remembered (eight). A combatant's contact is classified through
`ContactRecord::of`; the combatant's own list still drops its oldest
contact when full, where the original's admission (`contacts::admit`)
replaces the lightest. The running game has no room graph yet, so no
flight starts there.

Not in yet: role init scripts other than Activity and IdleActivity have no
effect (the interpreter runs them, but the AI's host exposes only those two
slots); encounter, corpse, dialogue and condition scripts and
`StartDialogue` are found and modelled, not run from the AI; the brain feeding the
combatant's mind-power overrides and group blend (they are in `combatant`);
the roam and patrol walk only
once the running game has a navigation grid, and the search's deeds have no
item instances, containers or corpses to act on there yet.

## The combatant

`combatant` is the shared base of the original's character behaviours: one
contact per character it has met (`Contact`: its tracked and noticed flags,
the noticed level and the noticeability, concern and concern target, the
effective attitude and tolerance, the threat, the crowd penalty carried
across passes, closeness, view weight, the selected third party and the
event bits), the references to its target, attention target, other target
and holder, its published combat rating and reach, its courage, the
struck rating and mark a blow leaves on the attacker, its alert level, its
queued combat step and its nav footprint.

`perceive` is one character looking at another; it writes the observer's
contact for it. An invisible character is skipped. The distance ignores
vertical offsets under 150 cm. The contact stays tracked inside 1200 cm
(halved while asleep; a hidden character's radius is cut by
`1 − 0.9·(1 − v)` and its inner radius by `1 − 0.8·(1 − v)` in double
precision, or by a masking power's two factors; 600 cm more while
tracked) and is lost outside both radii, which raises the alert of an
observer it targeted to 40. Closeness is `1 − d/1200`, at most 0.99.
Noticeability is `3c²` for a dead or asleep character (capping the noticed
level and clearing concern), `24c` down (an unconscious one's concern
decays by 0.99), otherwise `(f²/2 + 1)·((g + 2)·(2c⁴ + (1 − c²)c))` from
the other's look and facing toward the observer, +10 while it attacks.
The view weight is `0.25·((1 − look·u)/2)² + 0.75·(1 − fwd·u)/2`. For a
character neither dead nor asleep the attitude is blended halfway toward
the best trusted third party the other likes, and the threat is estimated
from the other's contacts: each third party's weight (its closeness and
the other's view of it; 32 while the other attacks, 2 down, 16 or 8 in or
out of combat) to the eighth power, the share of trust against the
observer's own blended attitude, the largest term, the observer's own and
the worst crowd term; it raises the concern target of the observer's
contacts with threatened third parties, sets the quick-swing event for a
mid-swing attack with a threat over 16 and concern over 1, quadruples the
threat of a fast body, and returns the familiarity drift
`elapsed·(1 − |raw|/100)²·((a + 0.25) − 0.75·all)·0.01`.

`effective_attitude` and `trust` apply the mind powers (kinds compared
under their mask) to a raw role attitude: frenzy makes the attitude half
the negated aggression and trust 0; binding or command sourced on the other
makes it 100, or averages it with 100 under the resistance technique; a
charm aimed at the other makes it 100; suspicion takes 50, not below
−100. `add_concern` is the script's `AddConcern` (the flags word becomes 3,
the attitude-weighted rise, capped at 15, the event at 1.2 or else at 4).
`look_direction` is the direction the original builds from a point back to
the eye, held within 45° of the body; `candidate_cost` is the footwork
step's pathfinder cost; `outside_radius` and `clamp_move_point` (a far move
point is put 8·speed + 20 cm straight ahead) are the carry and move
clamps; `Movement`'s two clear routines drop the queued step and set the
body's movement bits; `foot_cell` and `stamp_footprint` are the 30 cm nav
footprint (rounded to even, the fallback cell on the offending axis, the
once-per-run overflow log). `take_hit` lowers courage on a clean hit and
marks the attacker once; `forget` (the last contact takes the slot),
`replace` and `reset` clear or remap references.

`place` is the combat position score against one opponent: the close band
at its reach plus 150 cm, the strike ideal at the observer's reach plus a
size correction, a step's plateau and a fade to 600 cm, the danger of a live
side swing by its time to impact (4 at the edge, 1 by 900 ms, nothing past
10 s) and of a live thrust (1 or 8 by its arc), in and out of a fight, the
approach term peaking at the strike ideal, the recovering, never-arriving
and thrust scales of the approach, the observer's heading and attack state,
the press flag on the opponent's facing, and the observer's feral and
skill scales, with the original's double-precision steps.
`publish_arc` is the opponent's published sector and its recovering kind;
`blow_is_live` its is-attacking test; `between` the original's arc test;
`arrival_ms` its time to impact. `choose_footwork` (Exarust's reduction of
the original combat state's candidate search) turns the terms into hold,
approach, retreat or a circle out of a live swing, and the duellist runs
the chosen footwork (a short hold timer stands in for the switching cost)
before it takes its stance and strikes. The duellist perceives its target
each frame through `perceive`, with its stored attitude standing in for
the role database.

Not yet: the role database feeding `perceive` (the brain's attitudes are
not passed in), the encounter scripts and the familiarity write, the mind
power calls the pass and a hit make, the scene's forget broadcast on
removal, the pathfinder's footprint insert, the target selection and morale
rules, the full candidate footwork search, and the auto-parry.

## Scope

- Navigation over tile grids and terrain; activity/idle schedules.
- Combat decision making producing control inputs (movement, attack direction, stance) for the physics character.
- Companions/hirelings and arena opponents.

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
chosen step in its interim combat walk until it arrives; a guard lets go
of a swing in progress (once a swing), and a fighter stuck for 21 ticks
runs the chase route through the navigation grid when the level has one,
walking the route while the combat tick ends at the chase; it can be told
to hold its ground (a debug aid). The fighter's skill and blend come from
its brain's personality, else Exarust's default (skill 0, blend 1).

A level's living characters carry a brain (`brain`): their record's roles,
their relations and traits. When they appear their roles choose their idle
activity, and they choose it again whenever the held roles change; every
frame they rate the player from both sides' roles, which feeds the contact
rating (`sense`), and the brain's target pass decides whether the character
engages the player. The AI tick links each brain into its 600 cm room cell,
gathers the characters of that cell as its nearby list, decays its group
alert a step every ten seconds, and out of combat sways its look about its
heading and turns its model to it (the original lets the body's locomotion
turn the body; the running game ticks no locomotion request for an NPC, so
the model is turned directly). The original's reset when the level is
written out on leaving it (the body heals to its lasting health, forgets
every contact, and a roaming AI is put down along fresh wander paths and
leaves combat) is `presence::reenter`; Exarust writes no level on leaving,
so nothing calls it yet. Every frame the brain's
perception pass clears its contacts' concern targets, closeness and view
(as the original's target pass does before it perceives), looks at every
character within the largest perception reach and every one it already
knows, with that character's own contacts, the role database's attitude
and the relations' trust in its threat estimate, and writes the contact
words the integration reads. The original weighs only the characters in
its line of sight; Exarust has no such test yet, so the reach stands in.
A sleeper that rates the player above a notice drops the sleep role and
wakes.

| module | what it holds |
|---|---|
| `roles` | role lists (the shared `exa_formats::roles::HeldRole`; adding with inherited parents, newest first and counted; removing explicit roles), the new story character's and the arena fighter's roles, relations (global and per locale, relation roles with values, familiarity, the last attitude, a former level character's move), the attitude of one character toward another, encounter scripts, corpse scripts (each role's first entry aimed at the Player), the interaction-condition wrapper, the entry finder, and the `StartDialogue`/`ChangeRole` registration |
| `personality` | what the brain derives from the record (skill, confidence, restraint, courage, eagerness, blend, jitter, arousal) and the attach step (contact sense, dead and statue cases) |
| `activity` | applying roles: the init scripts run through `exa_script`'s interpreter (newest role first, with the role's own variables), the mind-control follow, the idle state per activity, the idle-activity registry (the search as 6, registered by `register_search`), and the activity base's four virtuals (reset and tick answer false, stop and restart do nothing) |
| `entity` | the AI's perceivable things: the base's position from the linked scene object (the origin when unlinked), its abstract assessment, the zero avoidance hint and the aim point that defaults to the position, and the search's goal entity (weight 5, attraction `1 − d/1200 cm` capped at 0.99, out of reach past 12 m) |
| `behaviour` | the character controllers' base: the body's ground point, the reset calling the body's hook then the state's, the attach clearing the body's two pending handles and remembering the scene object, and the event counts (kind 0 adds 3 to the body's behaviour count, kinds 1 and 2 add 1 to the controller's in a clinch) |
| `search` | the companion's search: gathering over the room grid, picking, the look sweep, the forced path, arrival, kneeling, looting worn items, packs and containers, item worth, wear room, comments to the leader, re-arming, tidying, and the world edits its deeds carry out (`apply_deed`) |
| `manager` | the AI manager's frame (dormant ticks, rising undead, the 3 cm zone check), zone deeds, the group alert (raised, held ten seconds and dropped one every ten), the approach clamp, and the scene node the level's AI start-up makes once (`_Temp`, flag `0x100000`) |
| `lookup` | a simulation's character by record id and a status effect by a masked word (first match in list order) |
| `brain` | the components and systems that carry roles into the running game: applying roles again when the held set changes, the attitude rating and waking, the perception pass, and the AI tick (morale, the group alert's ten-second decay, the contact integration, the target pass, the room cell and nearby list, the engagement, and the out-of-combat body turn) |
| `idle` | roam, patrol, sentry and the wait, the wander and home-side points, follow, sleep, knock-out and waking |
| `motion` | the AI's look and body motion: the head look toward a point (a quarter turn, 0.02 easing), the whole-look smoothing (`(0.0062831854·gap + sway)·0.7`, a 30° write threshold), the idle look (0.1 of the gap, 1 % pitch), the step/combat walk (the 0.7/0.007 facing threshold, the 150 cm goal hold, the run and sprint flags), the walk stop (8·crouch + 20), the steer line (120 cm, 3600 cm²), the way share, the alert pan (one draw in 300 and 75), the out-of-combat sway, and the aim, settle and snap |
| `rooms` | the 600 cm room cell grid: the floored, half-offset flat index, the per-cell occupant lists, the old-cell memory, and the cell's nearby characters |
| `lifecycle` | the script-object factory, the scripted action hook, the removal/destroy/disable hooks, the investigation of a disturbance, the idle driver's state machine (turn, plan a room, walk, test the exit, use the object, plan home, search the room) with its exit/use/object checks and room walk, the go-to-object state, and the object goal |
| `combatant` | the behaviour base: contacts, perception, the ally-class races' sense (the 3 concern floor, attitude 100 for a fellow AI, the 1200/1800 cm range, the threat to an ally weighed by the observed state, 0.3 of it as the concern target; a tested function, not called by the perception pass), the attitude and trust overrides, concern, references, the queued step, the nav footprint |
| `combat` | the combat state: the follow check, practice, the foe estimate, the weapon draw and the lunge, the chase count and eagerness, the footwork drives, the seventeen-place search, the attack decision with the friend test and the keep-off, the step, the guard, the fierce stance and the body turn |
| `contacts` | the contact classifier (the stress flight, the engagement threshold, the concern flight, the watch and the notice score), the twelve-contact list, the per-tick contact integration (the hostility drift from the perceived attitude, the concern rising toward its target, the noticed level following the sighting, the attention, the bad-route marks, the aggregates and the readiness), the target pass (the scripted focus, the visible and nearby lists, the sleeper's waking, the clean-up, the alarm mean, the encounter and the greeting, the round-robin dialogue refresh), the nearby list (sixteen nearest seen within 30 m, by squared distance) |
| `flee` | the flee start's morale, the way out of a room away from a threat, the fled-room memory, the flee state's tick and the cornered fall-back |
| `cast` | the power-cast chooser (ten slots, per-slot cooldowns, the power system's word, the pool budget after a round to even, the flat 20 and the cumulative draw, the 6 s rest written to the slot of the chosen power's table place) |
| `chase` | the chase route: the bark, the fresh-target and moved-foe (3-D) tests, the goal on the foe's ground and the route through the navigation grid, the reach test from the estimate to the route's end, the unreachable mark and its 3 s plus draw, and the leave-without-timers case |
| `route` | the one-planner-a-frame turn (the longest waiter remembered, strictly), the route start (two points and 150 cm, or forced), the path's flat length, and the navigation grid as the router (`exa_world::nav::find_path`, not a second pathfinder) |
| `sense`, `hear`, `fight`, `place`, `path`, `order` | contact rating, noise and alert, attack choice, combat positions, path following (the planner is `exa_world::nav::path`), steering orders |
| `golem` | the golem guardians: wake (`golem_act`, the glow material at full, the limbs' mass restored) and sleep (the hidden render bit, a twentieth of the mass, a fiftieth of each part's value), the alarm trigger's arming and wake callback, the homeward callback, the `Alarm_` box that holds the armed point, the strict plane-distance radius query over a 6 m sector, the player's post 200 cm past the box at 10 cm up, and the pose reset |

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
queries in the running game). The per-place scorer's notice and band
terms are overwritten by its close formula, so they are not kept. A stuck
fighter runs its chase route through the navigation grid when there is
one (`chase`, `route`; the duellists share one plan turn a frame); only
while it follows the route does the combat tick end there. A running body
stops its run before the drives (the step goal pulled to `8·speed + 20`
cm along its facing). The cast chooser runs at the first place for a body
not attacking with a foe, and a cast running past a second is torn down
once the power system is done (`cast`). A duellist with `DuelPowers`
asks it as its power system: the game answers its slots before
`DuelSystems` and casts the chosen power after it
(`exa_game::thaum::cast`); a level character whose record knows powers
gets one. The follow check is ported and not yet
wired into the running game.

**Morale.** The AI tick's morale (`combat::drift_morale`): stress falls
0.01 a tick and the alarm by 0.9999; a drop in the health ratio
(`health/max`, scaled by `0.4 + 0.2·confidence` while down) lowers the
courage; above `(h + 1)(confidence + 1.1)/2` it eases off, then drifts
toward `max(2·(stress + alarm), 2·total, confidence + 0.5)` at that factor
× 0.0003 (0.00015 from 1 up), or decays by 0.999 at or over it; panic
takes 0.001 of the stress over the courage and decays by 0.995; the
jitter walks a hundredth of a centred draw a tick and falls 0.003 while
down, held in ±1. It runs in `brain::Npc::run_pass`, which is not
scheduled yet.

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

`brain::Npc::run_pass` drifts the morale, integrates a brain's contacts and
runs the target pass over every character seen this tick, and reports
the engagement, the leave, the encounter and the greeting (Exarust has no
dialogue engine). The AI tick (`brain::ai_tick`) runs it after the
perception pass filled the contact words and before the duellist's combat
state, keeps the report (level, attention, stress, cursor) on the brain,
and feeds the report's stress and the integration's total back into the
next morale drift. The combatant's contacts follow the brain's concern and
hostility, as the original keeps one list. A pass that enters combat
against the duellist's target starts the duel (the duellist enters combat
on the next frame) and a calm pass leaves it; the mind's own rating no
longer decides the engagement. Exarust's duellist fights only its own
target, so an entry against another character does not start it. A
duellist without a brain, the practice opponent, is started by the fight's
setup as before. Not yet: the original's one brain a tick (the longest
waiter), the line-of-sight filter, the nearby list and the room graph.

The golem guardians' rules are plain functions with tests; no level has a
golem entity or an alarm zone yet.

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
before it takes its stance and strikes. A duellist with a brain is perceived
by the AI tick (with the target's own contacts and the relations' trust);
the practice opponent, which has none, perceives its target through
`perceive` with the role database's attitude when it is loaded.

Not yet: the encounter scripts and the familiarity write, the mind
power calls the pass and a hit make, the scene's forget broadcast on
removal, the pathfinder's footprint insert, the full candidate footwork
search, and the AI's own parry choice (the body's auto-parry runs in
`exa_combat::parry`).

## Scope

- Navigation over tile grids and terrain; activity/idle schedules.
- Combat decision making producing control inputs (movement, attack direction, stance) for the physics character.
- Companions/hirelings and arena opponents.

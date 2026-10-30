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
engages the player. A character wearing a mind ward carries
`brain::MindWarded`, the original's mind-masked bit: perception, the
brains' and a duellist's, sees it hidden, its range shrinking with the
observer's perception. The AI tick links each brain into its 600 cm room cell
and the level's room table, gathers the characters of that cell as its
nearby list, decays its group alert a step every ten seconds, and out of
combat runs the idle states' head (`walker`: the alert look-around while
the panic is at least 0.2 and the body keeps its in-combat mark, which
fighting sets; then the attention to the attended contact, whose notice
score must reach 1, with a look window of `clamp(1 − 0.1·score, 0.25,
0.7)·π`; only then is the mark cleared), the original's look chain
(`motion::look_chain`) and turns its
model with the eased look (the original lets the body's locomotion turn the
body; the running game ticks no locomotion request for an NPC, so the model
is turned directly). A roaming or patrolling brain walks its legs
(`brain::idle_walk`): on its turn of the one-planner-a-frame turn it picks
a wander goal a quarter of the way from a drawn door's near cell to the
cell behind it, routes it through the navigation grid (followed past
150 cm end to end), walks it with the walk clip through the NPC driver,
settles its look and pauses 0.5–2.5 s, then (a roam) walks a forced route
into the room it aimed at and pauses 1–5 s. A body the scene takes back
mid-leg faces its route's goal. The original's reset when the level is
written out on leaving it (the body heals to its lasting health, forgets
every contact, and a roaming AI is put down along fresh wander paths and
leaves combat) is `presence::reenter`; Exarust writes no level on leaving,
so nothing calls it yet. Once a body tick (not once a frame, as the
original runs it from the body tick) the brain's perception pass clears its
contacts' concern targets, closeness and view (as the original's target
pass does before it perceives), looks at every character within the largest
perception reach that the level's sight callback lets it see (in a grid
level the room reach test, `rooms::in_sight`: the same room, or an open
door whose point lies within 2 m of the other character; a level without a
grid sees everyone) and every one it already knows, with that character's own contacts, the role database's attitude
and the relations' trust in its threat estimate, and writes the contact
words the integration reads. A race-class-`0x12xx` brain and every non-AI
race run `combatant::perceive` (the standard sense `combatant::standard_sense`
is tested but not scheduled, see below); the other AI races use the
ally-aware one (`combatant::ally_sense`), whose fellow AI is a brain whose
race word carries the AI bit (`Npc::race_ai`). A sleeper that rates the player above a notice drops the
sleep role and wakes. A knocked-out or dead brain stops after its alert decay,
before its morale and state (the original's condition bit `0x800`). A placed
brain takes its sense, statue mark and effect from the record's profession
word, the race word the attach reads: the races whose word carries bit
`0x1000` (the golems, race ids 17–21) install a sense, the class `0x1200`
(the Golem Suit) the standard one and the others the ally-aware one, and the
statue bit marks a statue, whose sense clears what it sees. Every other race
keeps the behaviour base's perception. A statue keeps that mark
until a golem alarm zone wakes it (`exa_game::golem_alarm`, the only live
caller of `golem::wake`): the player stepping into an alarm zone without its
pass item wakes every statue within 800 cm of it. The original's tick ends by registering the character's
nav footprint unless it follows a route with its locomotion active at 1 cm
a tick or faster (`brain::stamps_footprint`). The game's footprint system
asks that gate for every NPC each fixed tick: the route-following flag is a
roam or patrol leg, a flight route or a duellist's chase route
(`brain::follows_route`), the locomotion is the walk the AI asks of the
body and the speed is the last fixed step's walk along the facing; a
refused NPC keeps its footprint off the grid. The tick's grabbed flag
(`brain::grab_timeout`) stays while the body reports the grabbed condition,
unless the state is not the combat state and the body's hook refuses event
`0x10`, which releases the grip and clears it; outside that condition it
clears once 500 ms have passed since the grab (at exactly 500 ms it
holds).
The sense's familiarity drift is added to the observer's relation with the
character it saw when that relation exists (a stranger's is dropped),
scaled by the elapsed count the brain's ticks seen carry (the contact
field the integration counts). A character that leaves the
scene is forgotten by every brain and combatant (its contact, nearby entry,
targets, attention, encounter and greeting references). Entering combat
sets the body's in-combat word (the duellist's entry then takes the Defence
stance, motion 3, through the set-stance helper). The target pass leaves
combat through the leave-combat routine (`motion::leave_combat`): the
body's look yaw goes into the brain's look, heading and head look, the
eased offset and sway (with the head offset, its target and the alert
pan's, which the original keeps in the same slots) are zeroed, the alert
look's wide pan waits 3 s and the sway and narrow pan 0.5 s.

| module | what it holds |
|---|---|
| `roles` | role lists (the shared `exa_formats::roles::HeldRole`; adding with inherited parents, newest first and counted; removing explicit roles), the new story character's and the arena fighter's roles, relations (global and per locale, relation roles with values, familiarity, the last attitude, a former level character's move, `migrate_former`), the attitude of one character toward another, encounter scripts, corpse scripts (each role's first entry aimed at the Player), the interaction-condition wrapper, the entry finder, and the `StartDialogue`/`ChangeRole` registration (`start_dialogue` carries the topic the script asked for) |
| `personality` | what the brain derives from the record (skill, confidence, restraint, courage, eagerness, blend, jitter, arousal) and the attach step (contact sense, dead and statue cases) |
| `activity` | applying roles: the init scripts run through `exa_script`'s interpreter (newest role first, with the role's own variables), the mind-control follow, the idle state per activity, the idle-activity registry (the search as 6, registered by `register_search`), and the activity base's four virtuals (reset and tick answer false, stop and restart do nothing) |
| `entity` | the AI's perceivable things: the base's position from the linked scene object (the origin when unlinked), its abstract assessment, the zero avoidance hint and the aim point that defaults to the position, and the search's goal entity (weight 5, attraction `1 − d/1200 cm` capped at 0.99, out of reach past 12 m) |
| `behaviour` | the character controllers' base: the body's ground point, the reset calling the body's hook then the state's, the attach clearing the body's two pending handles and remembering the scene object, and the event counts (kind 0 adds 3 to the body's behaviour count, kinds 1 and 2 add 1 to the controller's in a clinch) |
| `search` | the companion's search: gathering over the room grid, picking, the look sweep, the forced path, arrival, kneeling, looting worn items, packs and containers, item worth, wear room, comments to the leader (the original's own phrase templates), re-arming, tidying, the small object edits its deferred jobs ask for (`take_from_container`, `take_worn`, `take_carried`, `add_to_container`, `drop_item`), the world edits its deeds carry out (`apply_deed`), and an item record read as its weighing reads it (`Loot::of_payload`: apparel body, weapon section, operative or container part) |
| `manager` | the AI manager's frame (dormant ticks, rising undead, the 3 cm zone check), zone deeds, the group alert (raised, held ten seconds and dropped one every ten), the approach clamp, and the scene node the level's AI start-up makes once (`_Temp`, flag `0x100000`) |
| `lookup` | a simulation's character by record id and a status effect by a masked word (first match in list order) |
| `brain` | the components and systems that carry roles into the running game: applying roles again when the held set changes, the attitude rating and waking, the perception pass (line of sight, the ally sense and the sense a race installs, the familiarity drift written to the relations), the AI tick once a body tick (morale, the group alert's ten-second decay, the contact integration, the target pass with the room graph, the room cell and nearby list, the engagement with its in-combat word, the leave-combat look and its 3 s pan timer, the one-time attach snap, the look chain, the out-of-combat body turn and the footprint gate), the removal forget, and the flight's walk to a room's exit |
| `presence` | the activation range (200 m, a NaN out), the 80-tick hold, the scene move's end, the entering body's reset, the level-leave reset, and the placement an entering body takes (a collapsed body keeps its stored one; others take their point and face their path goal) |
| `idle` | roam, patrol, sentry and the wait, the wander and home-side points, follow, sleep, knock-out and waking |
| `walker` | the idle states' head (the attention's notice score and clamped window, the 50° focus window), the roam and patrol step on the planning turn (the goal picked only on the turn, the 150 cm follow span, the forced home leg, the settle before the pause), and the walk along the followed route (100 cm flat arrival, Exarust's give-up deadline) |
| `motion` | the AI's look and body motion: the head look toward a point (a quarter turn, 0.02 easing), the whole-look smoothing (`(0.0062831854·gap + sway)·0.7`, a 30° write threshold), the idle look (0.1 of the gap, 1 % pitch), the out-of-combat look chain (the body look toward the head offset, the 0.4/0.64 smoothing and the gait helper's 30° write, with the stepping, live-target and combat cases), the step/combat walk (the 0.7/0.007 facing threshold, the 150 cm goal hold, the run and sprint flags), the walk stop (8·crouch + 20), the steer line (120 cm, 3600 cm²), the way share, the body in the way (each body shifted 20 cm along the walk direction, skipping the remembered and carried bodies), the alert pan (both draws every tick, one in 300 and 75, the body turned by the sway with a foe), the out-of-combat sway, and the aim, settle and snap |
| `rooms` | the 600 cm room cell grid (the floored, half-offset flat index, the per-cell occupant lists, the old-cell memory and the cell's nearby characters) and the level's side of it: the cell-to-room table, a room's exits at the cell centres behind its door links with shut doors from the room tracker, a room's reference point, and the sight test (the room reach test) |
| `lifecycle` | the script-object factory, the scripted action hook, the removal/destroy/disable hooks, the idle return, the investigation of a disturbance and its goal callback (the pathfinder's nearest walkable point, mode 20, radius 100 cm), the idle driver's state machine (turn, plan a room, walk, test the exit, use the object, plan home, search the room) with its exit/use/object checks and room walk (every unvisited exit weighed, the previous cell included; going back clears the avoided exits; the first avoided exit sets its marker), the go-to-object state, and the object goal |
| `estimate` | the fighting estimate from a record's skills and combat skill, the held weapon's stat bytes and reach, and the body scale |
| `struck` | the hit reaction in the running game: each blow `exa_combat` notices (a wound, a clash or a block, in the damage routine's order) runs the combatant base's courage and attacker mark, then the AI's concern rise with the threat estimate over the brain's nearby list (only under the cap); a non-duellist attacker's lesson goes out as `Taught` |
| `combatant` | the behaviour base: contacts, perception, the AI's standard and ally-class per-contact senses, the ally-class races' sense (the 3 concern floor, attitude 100 for a fellow AI, the 1200/1800 cm range, the threat to an ally weighed by the observed state, 0.3 of it as the concern target, the observed body's in-combat mark weighing 0.25 over 0.1; run by the perception pass for every AI race outside class `0x12xx`), the attitude and trust overrides, concern, references, the queued step, the nav footprint |
| `combat` | the combat state: the follow check, practice, the foe estimate, setting a target (fresh, committed, stuck cleared, the estimate on the foe, the flat bearing and its aim), the weapon draw and the lunge, the chase count and eagerness, the footwork drives, the seventeen-place search, the attack decision with the friend test and the keep-off, the step, the guard, the fierce stance and the body turn |
| `contacts` | the contact classifier (the stress flight, the engagement threshold, the concern flight, the watch and the notice score), the twelve-contact list, the per-tick contact integration (the hostility drift from the perceived attitude, the concern rising toward its target, the noticed level following the sighting, the attention, the bad-route marks, the aggregates and the readiness), the target pass (the scripted focus, the visible and nearby lists, the sleeper's waking, the clean-up, the alarm mean, the encounter and the greeting, the round-robin dialogue refresh), the nearby list (sixteen nearest seen within 30 m, by squared distance) |
| `flee` | the flee start's morale, the way out of a room away from a threat, the fled-room memory, the flee state's tick and the cornered fall-back |
| `cast` | the power-cast chooser (ten slots, per-slot cooldowns, the power system's word, the pool budget after a round to even, the flat 20 and the cumulative draw, the 6 s rest written to the slot of the chosen power's table place) |
| `chase` | the chase route: the bark, the fresh-target and moved-foe (3-D) tests, the goal on the foe's ground and the route through the navigation grid, the reach test from the estimate to the route's end, the unreachable mark and its 3 s plus draw, and the leave-without-timers case |
| `route` | the one-planner-a-frame turn (the longest waiter remembered, strictly), the route start (two points and 150 cm, or forced), the path's flat length, and the navigation grid as the router (`exa_world::nav::find_path`, not a second pathfinder) |
| `sense`, `hear`, `fight`, `place`, `path`, `order` | contact rating, noise and alert, attack choice, combat positions, path following (the planner is `exa_world::nav::path`), steering orders |
| `ground` | the pathfinder's answers the combat state asks in centimetres: a place's ground cost (the cost sample through the combatant's formula), the line walk's clamp and the obstacle test, each the abstract pathfinder's without a grid |
| `golem` | the golem guardians: wake (the race word's statue bit and the statue byte cleared, `golem_act`, the glow material at full, the limbs' mass restored) and sleep (the statue bit, a twentieth of the mass, a fiftieth of each part's value), the zone check (the 3 cm step, the radius and box tests, the pass search through carried containers), the alarm trigger's arming, reset and wake callback (the contacts become the player at concern 10), the homeward callback, the `Alarm_` box that holds the armed point, the strict plane-distance radius query over the AI-bit characters of a room and the rooms behind its exits, the player's post 200 cm past the box at 10 cm up, and the pose reset |

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
order and its double-precision comparisons. The duellist asks the level's
navigation grid as the original asks its installed pathfinder (`ground`):
each place costs the cost sample under it through the combatant's formula
(`b·40 + ((a² + a)·120 + 1)`, 1 on open ground), each place and the long
step's continuation are pulled back where the grid's line walk stops, and
the chase check counts a target behind an obstacle as out of sight; the
snap stays the abstract pathfinder's no-op, as the level's never overrides
it. Around its combat ticks the duellist lifts its own footprint and stamps
it again, as the original's AI tick does around its state, and lifts its
foe's as the combat state does, stamping it again after the place pass
(an exit before it leaves the foe's to the foe's own controller), so it
pays only third parties' footprints. Entering combat takes the target
through the set-target rule with the estimate routine's lead
(`combat::foe_estimate`: a moving foe's chest led 30 ticks along its
facing at its speed, read from the player's locomotor) and copies the aim's
yaw into the brain's look; each combat tick's estimate reads the same
lead. Without a grid every answer is the abstract
pathfinder's (cost 1, no clamp, in sight). The per-place scorer's notice and band
terms are overwritten by its close formula, so they are not kept. A stuck
fighter runs its chase route through the navigation grid when there is
one (`chase`, `route`; the duellists share one plan turn a frame); only
while it follows the route does the combat tick end there. The step gate
(`combat::can_act`) also stays shut while the body's trailing-foot request
is pending (`Locomotor::plant_request`, up to 500 ms after a clip
replaced Walk or Run). The combat state leaves its stance motion alone while
the body's auto-parry holds (`Fighter::parry`'s parrying flag). A running body
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
`ContactRecord::of`; both lists admit as the original does: a full list
gives up its lightest contact (noticeability·10 + concern, the first slot's
noticeability unsigned; `contacts::admit`, `Combatant::lightest`), and the
perception pass puts the lightest back when the newcomer's sensed weight
falls short of it. The classification asks the level's room graph with
the contact as the threat (`rooms::level_room`, `rooms::room_exits`, the
door links' cells as exit points), as the original's way-out query does,
and `brain::flee_walk` runs a fleeing brain to that exit at the jog speed,
putting the idle clip back when the flight ends. On a level with a
navigation grid the way out is picked and routed on the brain's planning
turn (`route::plan`, one planner a frame), again while no route is followed
or within 150 cm of the goal; a route whose ends lie more than 150 cm apart
is followed toward its carrot, and one ending in the brain's room within
15 m of the threat remembers the exit's room. Without a followed route the
body steers straight to the exit (the original fights that tick).

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
setup as before. The brain's passes run once a body tick, the perception
filters by the room reach test, and the flight reads the level's room
graph and routes through the navigation grid. The one-planner-a-frame
turn (`route::PlanTurn`) is kept per system: the duellists' chase, the idle
walk and the flight each keep their own.

The golem guardians' rules are plain functions with tests; the alarm zones
of `exanima06` run live through `exa_game::golem_alarm`, while the alarm
return and the sleep have no live caller yet.

Not in yet: role init scripts other than Activity and IdleActivity have no
effect (the interpreter runs them, but the AI's host exposes only those two
slots); encounter scripts are collected by `roles::encounter` and run by the
game's `role_scripts` bridge, while corpse scripts run on the player's
double-click and dialogue and condition scripts are found and modelled but
not run from the AI; `StartDialogue` returns a `DialogueRequest` rather than
calling the dialogue engine; the brain feeding the
combatant's mind-power overrides and group blend (they are in `combatant`);
the roam and patrol choose
their wander and home points from the level's rooms (`rooms::room_exits`,
`idle::pick_wander`) but no system walks them yet. The search runs in the
running game under the follow (`exa_game::npc_search`) over the level's dead
bodies; containers and loose items have no instances there yet.

## The combatant

`combatant` is the shared base of the original's character behaviours: one
contact per character it has met (`Contact`: its tracked and noticed flags,
the noticed level and the noticeability, concern and concern target, the
effective attitude and tolerance, the threat, the crowd penalty carried
across passes, closeness, view weight, the selected third party and the
event bits), the references to its target, attention target, other target
and holder, its published combat rating and reach, its alert level, its
queued combat step and its nav footprint.

`standard_sense` is the routine an AI race of class `0x12xx` installs
as its per-contact sense: the threat word clears, a contact that is not a
character ends it there, the range is 1200 cm (1800 while
tracked, a strict squared compare) with a dead or asleep body (or an asleep
observer) taking a zero sighting, a noticed level capped at 0.5 and no
concern, hostility or selected third party, a knocked-out one 24·c and a
0.99 concern ease, and a live one the look/facing/closeness product plus 10
while it attacks. The observed body's own contacts are then weighed (the
weight from its condition, each contact's closeness and view and the
`min((concern + hostility)·0.15, 1)` softening in the original's mixed
precision); the heaviest picks the selected character, which the original
reads from the observer's own contact list at that entry's position in the
observed's list, and only the observer
(×multiplier×0.5) and the player (×multiplier) raise the concern target.
The player body clears the concern and takes attitude 1. It is tested but
not scheduled: the original installs it *instead of* the behaviour base's
perception, and Exarust's role-driven hostility still comes from
`perceive`'s attitude, which this routine overwrites for the player.

`perceive` is one character looking at another; it writes the observer's
contact for it. A character that cannot be perceived (an unseen power or a
body leaving the scene) is skipped. The distance ignores
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
the other's view of it; 32 while the other attacks, 2 down, otherwise 16
or 8 by the other's locomotion flag — 16 with its body movement bit 2 —
or, not moving, by its in-combat mark) raised through the eighth power,
the share of trust against the observer's own blended attitude, the
largest term, the observer's own and
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
−100. When the observer's concern for a character it looks at is over 1.5
and the raw role attitude lies more than 0.1 below the effective one, the
pass breaks the first power that holds it — a binding sourced on that
character or a charm aimed at it, not a command (`break_mind_power`: the
effect's state becomes 2).
`add_concern` is the script's `AddConcern` (the flags word becomes 3,
the attitude-weighted rise, capped at 15, the event at 1.2 or else at 4).
`look_direction` is the direction the original builds from a point back to
the eye, held within 45° of the body; `candidate_cost` is the footwork
step's pathfinder cost; `outside_radius` and `clamp_move_point` (a far move
point is put 8·speed + 20 cm straight ahead) are the carry and move
clamps; `Movement`'s two clear routines drop the queued step and set the
body's movement bits; `foot_cell` and `stamp_footprint` are the 30 cm nav
footprint (rounded to even, the fallback cell on the offending axis, the
once-per-run overflow log). `take_hit` lowers the brain's courage on a
wound (the factor `(confidence + 1)·0.025 + 0.95` is built in single
precision and applied in double) and marks the attacker's swing once, whose
practice then takes the struck character's rating as its lesson when
higher (`raise_lesson`); `react_to_hit` then sets the attacker contact's
flags word to 3 and, under the cap, raises its concern by
the threat estimate plus a base (half more for a kind-1 hit, a quarter for
a kind 2), floored at zero, scaled by `1 − attitude/100` in double and
capped at 15, and `threat_from_nearby` estimates that threat as 1 less the
largest share of the attacker's look that the observer's nearby bodies take
up (each body's aim point against the attacker's position, within the
attacker's radius plus 100 cm, its offset's dot over its square); `forget`
(the last contact takes the slot), `replace` and
`reset` clear or remap references, and `remove_from_scene` clears the
removed body's grid flags, resets it and has every combatant in the scene
forget it.

`place` is the combat position score against one opponent. Its first word
is the contact's engagement level, as the caller writes it: level 3
scores the strike term (at the observer's reach plus a size correction, a
step's plateau and a fade to 600 cm) and the approach term (peaking at the
strike ideal). Every other level clears those two and doubles the close
band at the opponent's reach plus 150 cm. The danger of a live swing is
scored at every level: a side swing by its time to impact (4 at the edge,
1 by 900 ms, nothing past 10 s), a thrust 1 or 8 by its arc, while the
candidate is closer than (skill + 3) × 20 plus the opponent's reach. The
recovering, never arriving and thrust scales of the approach, the
observer's heading and attack state, the press flag on the opponent's
facing, and the observer's feral and skill scales keep the original's
double-precision steps. The recovering blow lowers the close term by one
and scales the approach by `(2·skill + 2)·2` at any level.
`publish_arc` is the opponent's published sector and its recovering kind
(the swing word's cleaving bit, which also stops the sweep and so the
arrival time), its edges built from the head look's yaw (the body heading
while characters do not run the head tracker) and its look from the body's
facing (turned 20° while a weapon is held), replaced by the arc midpoint
on a side swing; the swing's continuation bits make it committed for
`blow_is_live`
its is-attacking test; `between` the original's arc test; `arrival_ms` its
time to impact. `Origin` and `measured_origin` are the point the score
measures a place from: 10 cm ahead standing, the chest leaned 70 ticks of
the body's speed along the heading (times one less its phase while
attacking), or toward a step target by half the opponent's own skill and
then 10 cm ahead; `score_place_at` scores one place
with its own distance, bearing and heading term, and the duellist feeds
each place's candidate angle and the contact level to the danger terms.
The combat state's seventeen-place search (`combat::search`) is the one
that chooses the footwork; the duellist runs it. A duellist with a brain is
perceived by the AI tick (with the target's own contacts and the relations'
trust); the practice opponent, which has none, perceives its target through
`perceive` with the role database's attitude when it is loaded.

The duellist publishes its fighting values when it enters combat
(`estimate`): two plus 0.4 a skill level (in double precision) plus 0.02
per combat skill, plus
with a held weapon 0.02 of its impact, thrust, slash, crush and pierce
hundredths times its reach, then plus 2 above a body scale of 1.3 or a
quarter below. The experience system keeps the one implementation; the
record's skill and combat-skill part is kept on the brain when its record
is applied, and the held weapon's raw stat bytes, reach and the body scale
are added at the entry. The weapon record keeps its section's raw stat
bytes for rules that read bytes.

Not yet: the encounter scripts, the hit's mind
power call (the take-a-hit rule's last branch), and the AI's own parry choice
(the body's auto-parry runs in `exa_combat::parry`). The head tracker
(`exa_combat::aim`) is the player and character-view behaviour's; Exarust's
player aim is the cursor direction in `exa_game::player`, so the tracker is
a tested rule without a caller yet.

## Scope

- Navigation over tile grids and terrain; activity/idle schedules.
- Combat decision making producing control inputs (movement, attack direction, stance) for the physics character.
- Companions/hirelings and arena opponents.

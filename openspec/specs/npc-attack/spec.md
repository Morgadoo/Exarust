# npc-attack Specification

## Purpose
The NPC combat state: its drives, its footwork over the places it weighs, the blow it picks once a fight has started (overhead, thrust, or a side), its guard and its body turn.

## Requirements

### Requirement: Fight blow
An unarmed fighter inside eight tenths of its reach SHALL be able to swing overhead, and outside that reach SHALL be able to take the other side. An armed fighter stepping forward slowly SHALL thrust when its preference clears the draw, and SHALL take the overhead when its age is at least a half. A facing score above 4 SHALL take the other side. A quadruped beyond eight tenths of reach SHALL be able to thrust. Energy under eight tenths of the maximum SHALL refuse the swing.

#### Scenario: Reach, preference and energy
- **WHEN** an unarmed fighter at full energy is inside eight tenths of reach and the generator starts at 0, then the same fighter is outside that reach, then an armed fighter stepping forward with preference 9, then that fighter at age one half, then a fighter whose attack phase is 100, then a quadruped beyond eight tenths of reach, and the outside unarmed fighter is tried again at 0.79 of its energy
- **THEN** the blows are overhead, the other side, thrust, overhead, the other side and thrust, and the last try starts nothing

### Requirement: A side swing scores danger
A side swing on its arc end at phase 0.5 SHALL score danger bits 0x40000000 at skill 0 and 0x40800000 at skill 1. Phase 0.35 SHALL score bits 0x3ff28f5c. Phase 0 and a sweep that is off SHALL score 0. A held weapon SHALL keep bits 0x40000000 past the limit. Outside a fight the strike SHALL be 0 and that danger SHALL stay.

#### Scenario: Edge at phase 0.5 and 0.35
- **WHEN** a side swing on its arc end is scored at phase 0.5 with skill 0 and skill 1, at phase 0.35, at phase 0, not sweeping, past the limit with a held weapon, and outside a fight
- **THEN** the danger bits are 0x40000000, 0x40800000 and 0x3ff28f5c, phase 0 and the sweep that is off score 0, the held weapon keeps bits 0x40000000, and outside a fight the strike is 0 and the danger bits stay 0x40000000

### Requirement: A thrust and a feral fighter score the place
A thrust at 100 cm SHALL score danger bits 0x3f000000 inside its arc and 0x40800000 outside it. At 200 cm danger SHALL be 0. Feral 1 SHALL halve the close term and score no danger. Dull SHALL keep the close term and score no danger.

#### Scenario: Thrust arc, feral and dull
- **WHEN** a thrust is scored at 100 cm inside its arc, at 100 cm outside it, and at 200 cm, and a side swing is scored at feral 1 and dull
- **THEN** the thrust danger bits are 0x3f000000 and 0x40800000, 200 cm scores 0, feral 1 halves the close term and scores no danger, and dull keeps the close term and scores no danger

### Requirement: A fighter scores the approach
At 25 cm and at 200 cm, with reach 50 cm, size 1 and skill 0, the approach SHALL be bits 0x3ec00000. A wide fighter at 25 cm SHALL score bits 0x3f88b000. At 400 cm, and outside a fight, the approach SHALL be 0.

#### Scenario: Near, wide and past the band
- **WHEN** a fight with reach 50 cm, size 1 and skill 0 is scored at 25 cm, 200 cm and 400 cm, the same spot is wide at 25 cm, and 25 cm is scored outside a fight
- **THEN** the approach bits are 0x3ec00000 at 25 cm and at 200 cm, the wide score is bits 0x3f88b000, and 400 cm and the outside fight are 0

### Requirement: A recovery pulls the close term
A recovering swing at 25 cm, reach 50 cm and size 1, SHALL score close bits 0x403c0000 and approach bits 0x3fc00000.

#### Scenario: Recovering at 25 cm
- **WHEN** a recovering swing is scored at 25 cm with reach 50 cm and size 1
- **THEN** the close bits are 0x403c0000 and the approach bits are 0x3fc00000

### Requirement: A weapon hand publishes the arc
Action 5 at phase 0 SHALL publish kind 1, start bits 0x3fdf66f3, end bits 0xc01c61aa and look bits 0xbeb2b8c4. At phase 0.5 a hand on the side SHALL set the end to bits 0x3fc90fdb and keep that start. A recovering swing of that action SHALL publish kind 4 and keep that end.

#### Scenario: Early edge and a hand at phase 0.5
- **WHEN** action 5 is published at phase 0, again at phase 0.5 with a hand on the side, and again while recovering
- **THEN** phase 0 is kind 1 with start bits 0x3fdf66f3, end bits 0xc01c61aa and look bits 0xbeb2b8c4, phase 0.5 keeps that start and sets the end to bits 0x3fc90fdb, and the recovering publish is kind 4 with that end

### Requirement: A facing scales the place
A fighter who is not attacking, with the candidate beside it, SHALL score approach bits 0x3f400000 at 25 cm. Straight ahead SHALL keep bits 0x3ec00000. A press with the opponent facing aside SHALL score strike bits 0x40300000 and approach bits 0x40840000.

#### Scenario: Beside, ahead and a press
- **WHEN** a fight at 25 cm is scored while not attacking with the candidate beside it, then straight ahead, then pressing while the opponent faces aside
- **THEN** the approach bits are 0x3f400000 and 0x3ec00000, and the press scores strike bits 0x40300000 and approach bits 0x40840000

### Requirement: An idle stamp recovers the arc
An armed action outside a swing SHALL publish kind 4 when the idle stamp is 5 and the clock is 4, with both edges on the heading. Stamp 4 against clock 4, and stamp 0 against the maximum clock, SHALL publish kind 0.

#### Scenario: Stamp above the clock, and not above it
- **WHEN** an armed action outside a swing is published with stamp 5 and clock 4, with stamp 4 and clock 4, and with stamp 0 and the maximum clock
- **THEN** the first publish is kind 4 with both edges on the heading, and the other two are kind 0

### Requirement: The approach peaks at the strike ideal
Inside the strike ideal the approach fraction SHALL be the distance over the ideal, so the term is 1.5 at the ideal; its floor SHALL compare the fraction with 0.1 in double precision, and the eased value SHALL be `(3 − 2f)·f²`. A recovering swing SHALL lower the close term outside a fight too.

#### Scenario: At, inside and at the floor of the ideal
- **WHEN** a fight with reach 50 cm and size 1 is scored at 50 cm, 10 cm, 5 cm and 4 cm, and a recovering swing is scored at 25 cm outside a fight
- **THEN** the approach is 1.5, about 0.016224, about 0.001176 (the f32 fraction 0.1 lies above the double 0.1) and 0, and the recovering close term is one less, doubled

### Requirement: The observer's heading and a thrust's arc scale the approach
When the observer is not attacking the approach SHALL be multiplied by one plus one minus the cosine of the candidate off the observer's heading; the opponent's facing SHALL NOT enter that scale. A live thrust whose arc holds the observer SHALL multiply the approach by `(2·skill + 1)·2` with the opponent's skill, past the danger gate too, unless the candidate inside the gate lies outside the arc. The observer's feral level SHALL scale the danger and the close term.

#### Scenario: A thrust past the gate
- **WHEN** a live thrust whose arc holds the observer is scored at 200 cm (past its 110 cm gate) with the candidate outside the arc, and again with the observer outside the arc
- **THEN** the first scores no danger and approach bits 0x3f400000, and the second approach bits 0x3ec00000

### Requirement: The score's double-precision steps
The live test SHALL compare the phase with `0.3 − 0.15·skill` in double precision; angle wraps and the arc midpoint's bound SHALL test ±π in double precision; the side swing's wait at phase 0.4 or less SHALL be summed in double precision, the thrust's wait SHALL be `(0.6 − phase)·1.5` in double precision, and the danger ramp SHALL be `4 − ms/300` in double precision for 900 ms or less. The recorded bearing SHALL be that of the opponent's point seen from the candidate.

#### Scenario: Bounds at the f32 values
- **WHEN** the f32 pi is wrapped, a midpoint starts at the f32 pi, a phase of 0.3 is tested at skill 0, and candidates behind, in front of and to the −x side of the point record their bearing
- **THEN** the f32 pi wraps by a turn, the midpoint spans across the cut, the phase is live, and the bearings are about 0, −π and about π/2

### Requirement: Practice from fighting
A lesson above 0.005 SHALL feed the gain by `(1 − fatigue)·lesson·0.1` and the fatigue by `lesson·0.0001`, capped at 0.9; a gain above 10 SHALL pay four times its rounded value into the record's practice count and keep the remainder; the lesson SHALL decay by 0.995 a tick and the fatigue always by 0.9998.

#### Scenario: A gain past ten
- **WHEN** a lesson of 1 meets a gain of 9.95 and no fatigue, then a lesson of 0.005 meets a gain of 3 and a fatigue of 0.5
- **THEN** the first pays 40 and keeps a gain of about 0.05 with the lesson at 0.995, and the second pays nothing and only rests the fatigue to 0.5 × 0.9998

### Requirement: Entering combat
Entering the combat state SHALL set the eagerness to a half, stop the press and stamp the clock; a fighter already in combat SHALL keep its state.

#### Scenario: Enter once
- **WHEN** a fighter with eagerness 3 that presses enters combat at 1234 ms, and is entered again later
- **THEN** its eagerness is 0.5, it no longer presses, its stamp is 1234 ms, and the second entry changes nothing

### Requirement: The foe estimate
With probability `(0.8·k² + 0.2)²` (k the skill) the fighter SHALL look at its foe again and blend the new position back toward the old by `0.6 − 0.6·k²`; the distance SHALL be at least 1 cm (the direction then left as it is) and 800 cm when the estimate is more than 300 cm above or below the chest. A stepping foe SHALL be led toward its step's goal by three quarters of the skill and a running one ahead by its speed × 30.

#### Scenario: Novice and master
- **WHEN** a novice's generator at 0 looks at a foe 300 cm ahead after an estimate 100 cm aside, a master looks at a foe 400 cm above, and a stepping foe is estimated by a master
- **THEN** the novice's estimate is 0.6 of the way back toward the old one, the master's distance is 800 cm, and the stepping foe is estimated three quarters of the way to its goal at its chest height

### Requirement: Drawing a weapon and lunging
A fighter with fists or a one-handed class and nothing held SHALL draw a carried weapon, holding its level at a watch while the draw is asked for or plays. Engaged and outside a swing, one tick in 300 SHALL start a lunge that measures the reach 80 cm longer.

#### Scenario: Draw and lunge
- **WHEN** a bare-handed fighter carries a weapon, then asks for the draw, then plays it; and an engaged fighter of reach 90 cm draws 0 of 300
- **THEN** the first asks for the draw, the other two hold the watch, and the lunge measures 170 cm

### Requirement: Chasing a foe
An engaged fighter with balance above 0.85 SHALL count itself stuck while its foe is beyond 600–800 cm (400 cm lying, 200–300 cm running, three quarters of that when it runs), more than 150 cm above or below, or out of sight, and SHALL start a chase after 21 such ticks unless the foe is marked unreachable.

#### Scenario: Stuck, then close
- **WHEN** a novice's foe stays 900 cm away for 22 ticks, then comes to 100 cm
- **THEN** it counts 1 to 21, chases on the 22nd tick, and the count resets close in

### Requirement: Eagerness
With full energy an engaged fighter's eagerness SHALL drift by `(3 − k)·(0.02·draw − 0.006)` up to 6 and fall by 0.01 above it; short of energy it SHALL be scaled by `1 − (1 − E/Emax)²`, and under eight tenths of the energy the press SHALL stop. Off balance or not engaged, the eagerness SHALL be 0.

#### Scenario: Tired
- **WHEN** a fighter of eagerness 2 that presses has half its energy, and then stands at balance 0.85
- **THEN** its eagerness is 1.5 and its press stops, and then its eagerness is 0

### Requirement: The footwork drives
Outside its own early swing the attack drive SHALL be the eagerness plus a half (halved by skill while it recovers or swings), the strike drive the eagerness (half while recovering) and the spacing `(2k + 3)·(2 + k)`. Below the engage level there SHALL be no attack or strike drive. The attack and strike drives SHALL scale with the blend and the square of the energy.

#### Scenario: Below the engage level
- **WHEN** a novice at level 2 works out its drives
- **THEN** its attack and strike drives are 0 and it does not press

### Requirement: Energy and the press scale the drives
At full energy a press SHALL multiply the attack drive by `2 + k` (twice more with fists) and the strike drive by 3; at full energy without a press both SHALL halve and the spacing double; short of energy the attack SHALL halve and the spacing quadruple.

#### Scenario: Fresh and tired
- **WHEN** a novice with fists at eagerness 0.5 presses at full energy, and then at half its energy
- **THEN** its drives are 4, 1.5 and 6, and then 0.125, 0.125 and 24

### Requirement: The swing's commitment
Early in its own swing (before 0.7 of the clock, 0.9 with fists) the fighter SHALL spend eagerness past 0.6 by `(1 − age/2)·(0.5 − 0.2k)·draw` and record its foe's lesson, measure its overshoot past 0.5 (twice the lunge reach less its distance, within −50 and 100, averaged by a tenth), press past 0.4 and commit past 0.3, its attack drive `(eagerness − k)·2 + 2`, doubled committed (four times with a shield). Without a press, the press SHALL come within 3 s of the last.

#### Scenario: Mid-swing
- **WHEN** a novice with a weapon swings at clock 0.55 with a lunge reach of 130 cm, 100 cm from its foe
- **THEN** its overshoot is 60 with a mean of 6, and its drives are 12, 1.5 and 4

### Requirement: The places weighed
The fighter SHALL weigh where it stands (or where its step lands) and eight steps (ahead, back, both sides, the four diagonals), and with skill and energy eight longer steps of `1.6 − 0.3·encumbrance` steps (ten places for a body scale above 1.25). A long step past the first two directions SHALL cost 2 more.

#### Scenario: A master and a big body
- **WHEN** a master at full energy plans its footwork, then one of body scale 1.3
- **THEN** it weighs seventeen places, and the big body eleven

### Requirement: The cost of a place
Each place SHALL cost its ground, the change from its step, three times each contact's danger and twice its closeness; where it stands it SHALL also re-choose its foe, the most pressing contact, whose closeness is then added once. A place SHALL be a strike place when its approach times the attack drive, less half its danger times the spacing, is above 1. The strike drive SHALL pull a place toward the foe by its strike term.

#### Scenario: A crowded place
- **WHEN** a master weighs seventeen places against a contact at level 1 that is close (1) and dangerous (0.5) everywhere
- **THEN** where it stands costs 3.5, the steps 4.5, the long steps past the second 6.5, it stays, nothing is a strike place, and the contact becomes its foe at level 1

### Requirement: The attack decision
A strike place SHALL win when it is cheaper than standing (by at least 2, once committed by 1), when no friend stands ahead within reach + 80 cm (one aside stops it in a swing), and a contact whose concern and hostility pass 1.8 SHALL not count as a friend. Outside its own swing a forward strike place SHALL give way to the place it stands on. A step SHALL keep off the foe's 60 cm × scale circle. Without a strike, a place SHALL be stepped to only when it is cheaper than standing by 1 or more.

#### Scenario: Friends, a foe in reach and a foe out of it
- **WHEN** a friend stands 100 cm ahead, 100 cm aside, behind, 131 cm ahead, or is hostile; a pressing fighter with a foe 100 cm ahead can strike within 120 cm; and the same fighter has a friend 60 cm ahead
- **THEN** the friend tests give 2, 1, 0, 0 and 0; the fighter strikes from where it stands; and with the friend in the way it does not strike

### Requirement: The guard
Without a blow a fighter of size under 1.3 SHALL let its swing go and take the guard when tired (yellow health under 0.01 or energy under its most less 0.05), else the combat idle, and SHALL lean 25° × skill against a foe's side swing inside the foe's reach + 50 cm. A fierce body SHALL instead take the guard at four reaches or more and the combat idle within, re-aiming only past 54°. The heading SHALL move 0.03 of its gap each tick.

#### Scenario: Lean
- **WHEN** a master on guard faces a foe of reach 100 cm swinging its first side 100 cm away
- **THEN** it aims (150 − 100)/120 × 25° aside and its heading moves 0.03 of that

### Requirement: The swing's body turn
During a swing the heading SHALL turn by σ(clock) × the swing's turn amount, σ −80° at the start, −60° to 0.2, a ramp to 30° at 0.5; open hands SHALL turn 50° less, a heavy claw 50° more, and a lunge 1.5 times as far. Late in the swing (0.68 for the main side, 0.78 for the other) the commitment SHALL halve, or clear with a stall, a hit or a release.

#### Scenario: Sweep
- **WHEN** a swing's sweep is read at clocks 0, 0.1, 0.35 and 0.6, then with open hands, a heavy claw and a lunging heavy claw at 0.6
- **THEN** the sweep is −80, −60, (0.35 − 0.2)·300 − 60 and 30, then −20, 80 and 120

### Requirement: The strike plateau
A fight score SHALL be 0.25 at 25 cm when the reach is 50 cm and the size is 1, and 1 from 120 cm through 130 cm when the step is 80 cm. At 600 cm the strike term SHALL be 0 and the fighter SHALL keep closing. The close term SHALL be 0 one centimetre past reach plus 150 cm. Outside a fight the close term SHALL double and the strike term SHALL be 0. A large opponent SHALL make 100 cm score 1. Wanting melee SHALL double 0.25 to 0.5.

#### Scenario: Plateau at 120 cm
- **WHEN** a fight with reach 50 cm, size 1 and a step of 80 cm is scored at 25 cm, 120 cm, 130 cm and 600 cm, the same spot is scored outside a fight, a large opponent is scored at 100 cm, and melee doubles 25 cm
- **THEN** the strike terms are 0.25, 1, 1 and 0, only 600 cm keeps closing, the close term past the limit is 0, the outside-fight strike term is 0 and its close term is doubled, 100 cm scores 1, and melee scores 0.5

### Requirement: The follow band in combat
A fighting follower's band SHALL ease toward 700 cm by a hundredth a tick, or, while the leader sprints, toward 500 cm plus 100 cm × how far ahead of the leader it is by a fifth, in double precision; while following only 0.7 of it SHALL count.

#### Scenario: Easing and a sprint
- **WHEN** a band of 600 cm is checked once, and a band of 400 cm 300 cm behind a sprinting leader
- **THEN** the first becomes 601 and the second stays 400

### Requirement: Answering the leader
Inside the band a follower SHALL stop; outside it the follow SHALL run when the leader runs or is three bands away, else walk; a follower with a route SHALL answer the call, replanning when the leader is more than 100 cm from the route's end, and one without SHALL plan.

#### Scenario: Near and far
- **WHEN** a follower with a band of 600 cm and a route is 300 cm, then 2000 cm, then 500 cm from its leader (the route's end then 101 cm off)
- **THEN** it stops, then it answers running without a new route, then it answers walking with a new route

### Requirement: The chase route
A fighter stuck past 21 ticks SHALL run the chase route: with no route followed and no fresh foe it SHALL drop its foe, focus and level and leave combat; with a fresh foe, or following a route whose end lies more than 100 cm (3-D) from the estimate, it SHALL plan a route to the estimate on the foe's ground through the navigation grid. The combat tick SHALL end at the chase only while a route is followed.

#### Scenario: Fresh foe
- **WHEN** a fighter on a flat grid chases a fresh foe 5.1 m away, then chases again
- **THEN** it follows the route and its tick runs on the first time, and ends at the chase the second time without a new plan

### Requirement: The unreachable mark
A chase plan SHALL mark the foe's contact unreachable for 3 s plus a draw of up to 3 s when the route has fewer than two points, ends more than 150 cm across the ground from the estimate, or more than 100 cm off the estimate's height less 100 cm; otherwise it SHALL clear the mark.

#### Scenario: Off the foe's height
- **WHEN** a chase plans to a foe whose estimate lies 2.01 m over the floor, then 2.00 m
- **THEN** the first marks the contact until 3–6 s on, and the second clears it

### Requirement: Weighing powers
At the first place, a fighter that is not attacking and has a foe SHALL weigh its ten power slots: a resting slot is skipped, an empty one and a refused weightless power rest 30 s, a power of weight under 4 rests 3 s; each weighed power counts a flat 20 and five times its weight, zeroed when its cost passes the pool × 200 (rounded to even) less the costliest usable cost.

#### Scenario: The pool budget
- **WHEN** a usable power of cost 50 and an unusable one of cost 10 are weighed with a pool of 0.1, then 0.25
- **THEN** nothing is cast with 0.1, and with 0.25 only the usable power can be

### Requirement: Casting a power
A draw below the flat part SHALL cast nothing; otherwise the first power whose running total is not below the rest SHALL be cast and the attack drive cleared; a share under 11 SHALL rest for 6 s the slot numbered by the power's place in the table. A cast running past a second SHALL be torn down once the power system is done with it.

#### Scenario: A light power's rest
- **WHEN** slot 0 holds a usable power of weight 4 and slot 3 one of weight 2, and the draw's rest is 21 of their totals 20 and 30
- **THEN** the power in slot 3 is cast, slot 1 rests 6 s and slot 3 rests 3 s

### Requirement: Stopping a run
A running fighter SHALL stop its run before the drives: the run bit clears, the stopping bit sets, and a step goal farther than `8·speed + 20` cm SHALL be put that far along the body's facing, then walked from the ground with the navigation line walk.

#### Scenario: The goal along the facing
- **WHEN** a body at speed 1 facing +x has a step goal a metre ahead along +z
- **THEN** the goal is 28 cm along +x

### Requirement: The can-act gate
A body SHALL act only with no blocking condition bit, a grip of at least 0.7, an awareness above 0.7 (in double precision) and no blocking state; a NaN fails.

#### Scenario: Grip and awareness
- **WHEN** a free body's grip is 0.7 and its awareness 0.7
- **THEN** it may not act until its awareness rises above 0.7

### Requirement: The combat state asks the navigation grid
In a level with a navigation grid, a fighter's footwork SHALL cost each place the grid's cost sample under it as `penalty·40 + ((clearance² + clearance)·120 + 1)`, SHALL pull a place, and a long step's continuation, back to where the grid's line walk from its base stops (keeping the asked height), and its chase check SHALL count a target behind an obstacle as out of sight. The snap SHALL leave a point where it was.

#### Scenario: A wall ahead
- **WHEN** a fighter on a flat grid weighs a place 8 cells ahead with a wall 4 cells ahead
- **THEN** the place is pulled back to the cell before the wall, and a foe beyond the wall is out of sight

#### Scenario: A footprint and an open floor
- **WHEN** a place lies on a cell of full penalty, and another on open ground
- **THEN** the first costs 41 and the second 1

### Requirement: The fighter's own footprint and a level without a grid
The fighter's own footprint SHALL be off the grid while its combat state runs and back where it was afterwards. Without a grid every place SHALL cost 1, no line SHALL be pulled back and nothing SHALL be out of sight.

#### Scenario: Its own footprint
- **WHEN** a fighter whose footprint is stamped where it stands runs its combat state
- **THEN** where it stands costs only the other characters' footprints, and after the state its footprint is stamped there again

#### Scenario: No grid
- **WHEN** the level has no navigation grid
- **THEN** every place costs 1, no place is pulled back and the foe is in sight

### Requirement: The engagement level gates the score
The position score SHALL compute the strike and approach terms only at engagement level 3; at levels 0, 1, 2 and 4 it SHALL clear both and double the close term. A live swing's danger and a recovering blow's close reduction SHALL be scored at every level.

#### Scenario: Off the fight level
- **WHEN** a place 25 cm away with reach 50 cm is scored at levels 0, 1, 2 and 4, and a side swing at phase 0.5 is scored at level 2
- **THEN** the strike and approach are 0 and the close term is twice level 3's, and the swing's danger bits stay 0x40000000

### Requirement: The foe's swing word publishes its arc
The duellist SHALL publish its foe's arc as recovering (kind 4, not sweeping) when the foe's swing word carries the cleaving bit, and committed when it carries either continuation bit. The foe's blades, reach and skill SHALL be read from the fighter under the target, and the thrust's arc test SHALL take the observer's bearing from the foe's measured origin.

#### Scenario: Swing word bits
- **WHEN** an empty-handed foe mid attack publishes with no bits, the cleaving bit, each continuation bit and the release bit
- **THEN** no bits give kind 3, sweeping and not committed; cleaving gives kind 4 and no sweep; each continuation bit commits; the release bit changes nothing

# Spec Delta

## ADDED Requirements

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

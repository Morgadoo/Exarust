## Why

Roadmap task B4 (lane `routewalk`). The original's AI tick calls one walk
after the state call while the character's route is followed; the walk
runs the path follower and writes the body's locomotion request. Exarust
walked routes with four per-state walkers (the roam's, the flight's, the
duellist's chase and the follow's), each with its own route, its own
copy of the driver and a fixed 100 cm flat arrival, while the ported walk
(`motion::walk`) and its head look had no caller and the follower's
pieces (`path::follow_path`, `path::line_replan`) were test-only.

## Evidence

REA (Exanima.exe) and the disassembly, checked against the instructions
where constants, order or branches matter (constants read from the image):

- The AI tick calls the walk right after the state call when the route's
  following byte is set, else the idle head; the walk's caller passes the
  in-combat flag false.
- The walk: a stepping body returns at once; the follower runs; a route it
  ended sets an 80-tick count (counted down, answer 2, by the walk while no
  route is followed; at 0 the follow mode word clears, answer 0); out of
  the scene only the moving byte is set; in the scene the aim, the gate
  (0.7 past 100 cm, else `0.007·|d|` in double), the 150 cm far goal, the
  run test (urgency above 0, the pathfinder's cost at the goal not over 0.5
  with a NaN passing, the alignment over the double 0.9), the sprint at
  urgency 2, the look yaw, the look eased, the head look at the AI's look
  target (else the offset eased 0.02 toward 0 in double), the body in the
  way's soft stop; the flags are written after the follower's.
- The follower: fewer than two points stop; in the scene the nearest
  projection (3-D, the parameter clamped, a NaN kept, the start's best
  `3.4e38`); out of the scene the 2 cm slide, crossing with
  `a − (f·a + 2)` over the next flat length where `a = 1 − (1 − f)·L`; the
  100 cm carrot; the urgency-0 run bit until the carrot passes 0.99 (in
  double) and its 30 cm soft stop, the hurried 100 cm soft stop; the line
  walk's plan request in the scene. The follower makes no indirect call
  but the pathfinder's line walk: the goal record's end test is called only
  by the planner.
- The route stop clears the following and plan bytes and the run bit; the
  soft stop clears the following byte, clears run, sets stopping, pulls the
  goal to `8·speed + 20` and line-walks it from the ground point.
- The AI tick stops the route of a body whose condition has the down bit.

## What Changes

- `exa_ai::route_walk`: `AiRoute` (the character's one route, required by
  the duellist), `follow`, `walk`, `route_stop`, `soft_stop`, and the
  system `walk_routes`, the last of the brains' chain.
- The roam, the flight, the chase and the follow (and its search) plan into
  `AiRoute` and no longer walk; the knocks clear its following byte.
- Removed duplicates: `drive::drive`, `drive::obstacle`,
  `drive::soft_stop`, `walker::follow_walk`, `walker::slide_walk`,
  `motion::walk`, `motion::stop_walk` and the path module's follower
  slices.

## Impact

NPC motion and lockstep traces change wherever an NPC walks a route: the
end of a walk (a soft stop at 30 cm of the carrot at urgency 0, 100 cm of
the last point above, instead of 100 cm flat), the blocked line's plan
request, the out-of-scene slide for every following state, the head's
offset, and the request's flags after a stop.

## Out of scope

The idle head's order against the state systems (Exarust's states are
separate systems run after the AI tick's head), the follow mode word's
clear at the end of the after-route count, the carried body the walk
clears, the body-in-the-way memory's 160-tick count, the door step of the
follower's path (`path::pass_door`), and any GPU scene run (the card is on
hold).

## Why

Phase 6 (AI) in docs/PLAN.md, TMotileAI and TPathFinder. The AI's motion and search helpers still built every yaw
and direction with exact trigonometry where the original calls the engine's fast heading, direction, pitch
direction, two-argument arctangent and normaliser. Round 151 tried the port and reverted it: the brain's idle glue
read each NPC's facing back from the model rotation it had just written, so the fast heading's error (up to 4e-4
rad) fed the next tick's turn and a standing NPC turned. Separately, the story revive's brain reset ran in the
frame's world, which behind the simulation bridge holds no navigation grid: no roaming brain was ever moved.

## What the original does

REA of the turn (`turn_towards`), snap, settle, leave-combat, head-look, gait-smoothing and alert-look routines, the
AI tick's idle look, the search's look routine, the engine's heading, direction, pitch direction, arctangent and
normalisers, the revive reset and the room bookkeeping, the footprint lift and stamp, and the nearest-walkable flood:

- **Reads, not round trips.** The turn reads the body dynamics' look vector through the heading every tick and
  writes only the AI's own eased yaw; it never writes a body direction. The body controller turns the body. The
  snap reads the body's facing through the heading and copies the facing vector itself into the body's heading,
  smoothing and look. The settle and the leave read the heading of the body's heading direction.
- **Which routine.** The head look, the gait smoothing, the alert look and the settle use the fast heading and the
  fast direction; the idle look takes its yaw by the fast two-argument arctangent of the look's (x, z) (the heading
  when a look target still runs) and builds the look by the pitch direction; the walk normalises by the
  approximate reciprocal square root; the search's look routine takes the head's and the object's yaws by the
  heading.
- **The revive reset.** A roaming reset lifts the footprint before it plans, moves the brain into the room of the
  point each round plans from (under a level flag the AI tick also tests), and stamps the footprint again where the
  body ends.
- **The flood's shared table.** The nearest-walkable flood uses the planner's stamp table and clears only a 32 by 33
  cell box of it; documented, not reproduced (it would tie a flood's answer to the order of earlier plans).

## What changes

- `motion::yaw_of`, `dir_of`, `pitched`, `unit` and the new `yaw_atan2` call the engine routines; `model_yaw` keeps
  exact trigonometry for the model's own rotation; `search::yaw_of` is the heading.
- `snap_to_body` and `aim` take the facing vector; the settle reads the body's heading direction.
- The brain's stand-in turn (`Npc::turn_model`) remembers the yaw it turned the model to, and `Npc::facing` answers
  it while the model keeps that rotation (the heading of the forward otherwise); the idle turn, the flight and roam
  walks and the companion's walk use them.
- The revive pass posts its reset to the next fixed step (`revive_brains`), lifts and re-stamps the footprint and
  enters the room per round.

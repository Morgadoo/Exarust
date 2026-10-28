# Proposal

## Why

docs/PLAN.md, Phase 6 (AI), the TMotileAI paragraph: the level's room
table, the room exits, the flight, the look chain and the ally sense were
tested functions with no caller in the brain, the brain's passes ran once
a frame, and the perception had no sight test.

## What Changes

- The brain's perception, AI tick and flight walk run once a body tick
  (`brain::BrainClock`).
- The perception sees only what the level's sight callback lets it see:
  in a grid level the room reach test (`rooms::in_sight`, built on
  `exa_world::rooms::Rooms::light_reach`), otherwise everyone.
- The attach installs the standard sense for race class `0x12xx` and the
  ally sense for the other AI races (the two were swapped); the brain
  perceives through the sense it holds.
- The AI tick reads the level room (`rooms::level_room`,
  `Npc::enter_room`), the room's exits (`rooms::room_exits`), and gives
  each classified contact's way-out query the contact's position and room
  (`flee::way_out` over the brain's fled rooms).
- An out-of-fight body (condition bit `0x800`) stops after the alert
  decay.
- The out-of-combat look chain (`motion::look_chain`) runs in the tick.
- A brain at level 4 runs to the way out (`brain::flee_walk`).

## Evidence

Read from the assembly: the attach routine compares the race word's
`0xFF00` class with `0x1200` and installs the routine the existing
`combatant::perceive` ports there, the ally-aware one otherwise. The
nearby/visible builder calls the world's sight method pointer, which the
grid environment's constructor sets to its light-reach test (the same
object carries the grid's width and height the AI's room index reads); a
world without a grid keeps the default, `mov al, 1; ret`. The room
bookkeeping stores the table word less one, unlinks on −1 keeping the
previous room, and writes the previous room only from a room ≥ 0. The AI
tick's condition-`0x800` return comes after the alert decay. The look
chain's tenth, 1 %, 0.4 and 0.64 are double multiplies, its ±π wraps and
the 1.6 test double compares, and the gait helper reads and rewrites the
body's heading vector against a 30° threshold.

## Out of scope

The activation holder moves and the body re-place, the per-room occupant
lists and the gather over the exits' rooms, a nav route for the flight,
the idle walker for roam and patrol, and a running race word that selects
the ally sense.

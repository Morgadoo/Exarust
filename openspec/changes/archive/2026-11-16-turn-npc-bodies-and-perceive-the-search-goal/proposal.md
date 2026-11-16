## Why

Roadmap task B4 (lane `npcbody`). The combat lane found that most of its
player-visible rows waited on two structures Exarust did not have: an NPC
body that turns and looks through its locomotion, as the player's does
(its duellist turned its model straight to the AI's heading, so no body
heading trailed the look and the look snap, the head tracking and the
out-of-combat heading writer had nothing to act on), and the search's
perceived goal entity (F389: the live search took a look-at goal directly
instead of waiting for the brain to attend the entity its pick creates).

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly where order and constants
matter:

- The AI tick's heading block after the state: with the presence word 1
  and no route followed (a followed route runs the route walk instead), a
  stepping body or one whose condition is 2 copies its look direction into
  its look smoothing and heading; else the in-combat mark calls the look
  snap; else the out-of-combat look chain ends in the gait helper that
  writes the body heading past 30°.
- The look snap: the gap from the yaw of the look smoothing to the yaw of
  the heading (each the engine's heading of the vector), wrapped in double
  precision, strictly over the single product 30·degree moves the heading
  to the look direction (the scale divide on its argument is dead).
- The combat tick's end writes the look direction from the AI heading and
  copies it into the look smoothing; the charge writes the look direction
  toward the target point and takes the heading from it.
- The sweep calls the AI's head tracking (slot 33) while the current
  motion is 11; the tracker reads the previous look smoothing, the eye in
  the character frame, the body's facing and the AI's estimate (slot 32),
  writes the look smoothing, the stab aim point and the look direction;
  the stab start sets the head yaw to the yaw of the look smoothing and the
  pitch to 0. The tick's end then writes the look again.
- The chase's following branch calls the grip-mode setter with 1 every
  tick.
- The search's pick links the brain's goal entity (created on the first
  pick, weight 5) to the picked object and sets the pick flag; its
  release, give-up and loot end clear the flag and unlink it. The target
  pass weighs the goal entity first while the flag holds (its assessment
  writes the contact: alarm 0; within 1200 cm the weight as the noticed and
  sighting levels and attention 0), the calm pass keeps attending it, the
  integration never raises its attention, the idle attention's window is
  50° on it, and the search's look and path wait until the attention is
  that entity.

## What changes

- `exa_ai::drive`: the body directions' heading block (`after_state`),
  their write into the locomotion request (`publish`) and their return
  (`adopt`, `adopt_body_directions` at the start of the brains' tick);
  `face_standing` leaves a body whose AI wrote its directions.
- `exa_ai::motion::look_snap`.
- `exa_ai::route_walk::walk_routes` runs the heading block in the route
  walk's place; the look target resolves the goal entity.
- `exa_ai` duellist: no model turn for a body on its locomotion; reads its
  body heading and look smoothing; writes the tick's look; runs the head
  tracking (`exa_combat::aim::track_head`) from the sweep; the chase's grip
  mode (`exa_character::grip_runtime::GripModeAsk`).
- `exa_ai::combat::Tick`: `cursor` and `track_head`.
- `exa_ai::brain`: the out-of-combat chain (`out_of_combat_look`) runs for a
  body on its locomotion without the mark, also during the staging freeze,
  and no longer turns a muscled body's model; the flight's look direction;
  the goal entity and pick flag, the pass's goal sighting.
- `exa_ai::entity`: `GoalEntity`, `pick`, `release`, `goal_id`.
- `exa_ai::search`: the pick and release hooks.
- `exa_game::npc_search`: the searcher's attention is the brain's.
- `exa_combat::stab`: the duplicate `StabAim::step` is gone.

## Out of scope

The out-of-combat look chain's live look target (none in the running game);
the leave and idle-return look yaws (still the model's facing); the
intermediate callback when the AI's grip mode is overruled later in the
same tick; the goal entity's attraction (no contact field); an unlinked goal
entity at the origin as a look target.

# Proposal

## Why

The duellist stood in for the original's NPC combat state with invented
rules: it walked straight at its target at a fixed range and struck every
2.2 s. The original's combat state (the largest routine of its AI, about
18 KB) decides footwork, attacks, guarding and the body turn of every
fighting NPC from its eagerness, its press, its energy and a search over
up to seventeen places; none of it was in.

## What Changes

- Port the combat state as `exa_ai::combat`: practice from fighting, the
  enter step, the foe estimate and its bearing, the weapon draw, the lunge,
  the chase count and the eagerness, the three footwork drives, the
  seventeen-place footwork search with its costs, the attack decision with
  the friend-in-the-way test and the keep-off, the step (start, queue and
  long step), the guard and fierce branches, the swing's body turn and the
  heading's approach. The original's float and double arithmetic and its
  generator draws are kept in order.
- Make the attack choice compare reach, scale, crouch and energy in double
  precision and order its facing score as the original does; let it take
  the attack executor's answer before the swing turn is drawn.
- Run the duellist on the combat state, once per body tick: the place
  search through the position score, the steps it walks, the stances the
  guard branch picks, the blows it starts and the heading it turns to.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-attack`: the combat state (new requirements); the strike plateau
  requirement no longer states the duellist's interim walk.
- `combat`: the duellist AI follows the combat state.
- `world-viewer`: the still opponent's stance is the combat state's.

## Impact

- `exa_ai::combat` (new), `exa_ai::fight`, `exa_ai` (the duellist).
- Evidence: REA on the original's combat-state routine, its enter,
  practice, down, leave, estimate, set-target, bystander and keep-off
  helpers, the engine's quick arccosine, wrap and vector helpers, and the
  constants they load (`context/re/npc-combat-state.md`). No format field
  changes, so the corpus step is a skip.
- Out of scope: the per-contact position score (another component; the
  duellist feeds `exa_ai::place`), the ground cost, line walk, sight test
  and nav snap (no navigation queries in the running game: defaults), the
  chase route, the follow check, the cast choice and the casting stop,
  the stab aim call, the body's own step (the duellist walks a stand-in),
  the release of a swing in the running game, the contacts beyond the one
  foe, and the three fields the routine resets whose readers are
  elsewhere.

## Phase

Phase 6, the NPC combat AI.

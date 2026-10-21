# Proposal

## Why

The hand took hold on the left button's press, after a guess at the
interface's event masks. Tracing the interface's mouse code shows the
hand's handler answers the drag start (the pointer moved with the button
held), that a release without a drag is a click with its own handler
(which turns the player toward the cursor), and that a double click is a
third event. A plain click on the floor in Exarust grabbed, refused or
held where the original only turns the player.

## What Changes

- The hand acts on the drag start: the pointer beyond 2 pixels of the
  pressed point on either axis, or the early allowance 2 × ((200 − t)/20)
  within 190 ms; the pick is at the pressed point.
- A release before the drag is a click: outside combat mode the heading
  turns to the cursor's last direction when it is over 70 cm away.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: the left press becomes the drag start; the click turn.

## Impact

- `exa_game::hand` (`LeftPress`, `drag_threshold`, `starts_drag`,
  `click_turns`; the press system on the drag start; the hold system's
  click), `exa_game::player` (`PlayerBody::cursor`).
- Docs: `docs/PLAYER.md` (The hand). `docs/PLAN.md` is the integrator's.
- Evidence: REA (Ghidra 12.1.4) on the interface's mouse-move, mouse-down
  and mouse-up handlers (the event types 0x1000 press, 0x4000 drag start,
  0x2000 click, 0x8000 double click; the allowance and its constant 2 set by
  the desk's constructor) and the click handler's turn; notes in
  `context/re/player-control-hand.md` (GUI mouse event types). No corpus.
- Out of scope: the click's naming and use of what was clicked, the double
  click's interactions (inventory, loot, talk, containers): no window for
  them yet.

## Phase

Phase 3, bodies and control: M34, the hand's drag ("Still to do:
interact").

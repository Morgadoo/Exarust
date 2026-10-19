# Design

## Context

Close checks style bit 0x20 before the already-closed return. When the bit is set it searches the capture list from the end and, on a hit, drops that entry and clears the bit. A miss leaves the bit. The already-closed return is next, so a closed child can still release a capture and then stop.

If the child is the task's active form, the callback runs, the active pointer clears, and close returns. Otherwise a hover equal to this child clears. The task's children are walked from the first to the last. One that is not closed and names this child is closed with the same argument. Own children follow. A shown child hides, which asks for a redraw. The callback runs. Close asks for a redraw again. The redraw word gains bits 0 and 1 on each ask.

## Goals / Non-Goals

**Goals:**

- A captured shown child that is the hover releases the capture, clears the bit, clears the hover, closes the owned sibling, hides, and asks twice.
- An already closed captured child releases the capture and does not ask.
- A child that is not in the capture list keeps bit 0x20.
- The active form clears the active pointer and does not ask.

**Non-Goals:**

- The cursor value restored from a capture that remains.
- The focus call inside hide, and the root refresh after it.
- The call after the two dismiss lists.

## Decisions

1. Captures, the active name, the hover, the siblings, and the redraw counters live on `MatchGate`.
2. `redraw` is the word. `redraws` counts the asks, so one ask and two asks both leave the word at 3 and stay distinct.
3. `TGUIForm` moves from 1% to 2%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.

## Risks / Trade-offs

- Dropping a capture by name is the search and the compact. The 32-byte cursor record is not stored.

## Migration Plan

- The same close gains these steps. The mask, the own-child walk, and the leave stay as they are.

## Open Questions

- None for the bit, the hover, the sibling walk, and the redraw word.

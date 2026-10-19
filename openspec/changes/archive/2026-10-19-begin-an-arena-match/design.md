# Design

## Context

The main-menu click handler switches on the page, then on the command. Page 2 is in game. Command 4 compares the mode word: 8 is arena, 4 is story, and any other value is the remaining path. Only arena calls the specialist leave. It then asks for a save with the low-health abort clear, arms the match, stores session phase 0, and starts a transition.

The arm stores three values. The byte set to 1 is the hub-busy flag: closing the manage window stores 0 in the same byte. One qword cleared is the world click hook. A second qword is also cleared; the match start clears it again. Its role was not established.

## Goals / Non-Goals

**Goals:**

- `begin_match` calls `leave_before_match` only for in-game command 4 in arena.
- A hired physician at 699 is removed and the death count is 1.
- The hub is busy, the click hook is clear, the unnamed pointer is clear, the phase is transitioning, and a save was asked without the low-health abort.
- An empty roster still arms the match.
- Story, practice, command 1, and another page change nothing.

**Non-Goals:**

- Closing open windows before the leave.
- Writing the save package. The ask is recorded; the writer is not called.
- The transition that follows the phase store.
- The story and practice branches of the same command.

## Decisions

1. `MatchGate` holds the hub flag, the click hook, the unnamed pointer, the phase, and the save ask.
2. A miss returns false and does not call the leave.
3. `XA_Arena` stays 34%. The arm is 30 bytes, and one menu branch does not move the game-modes section off 0%. The runtime section stays 4%.

## Risks / Trade-offs

- The unnamed pointer is cleared without a role. The test only checks that it becomes clear.
- Recording a save ask is not a package write.

## Migration Plan

- A new `menu` module. The leave, hire, and the session file stay as they are.

## Open Questions

- None for the arena branch of command 4.

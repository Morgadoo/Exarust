# Design

## Context

When a session mark changes to 0, a flag is stored as 0 and a code is stored as 4. Side 0 then selects the first bank. Any other side raises a notice and selects the second. A select copies sixteen words only when the bank flag does not already match, then an install runs. The words between those sixteen are not copied. The install body was not identified. The tables are filled at run time.

## Goals / Non-Goals

**Goals:**

- Side 0 with a set flag copies the first bank, clears the flag, and sets the code to 4.
- Side 1 with a clear flag copies the second bank and raises a notice.
- A flag that already matches leaves the words and still counts the install.
- A mark that is already 0, and a mask with no focus, leave the code and the words.

**Non-Goals:**

- The install body.
- A world update of the side.
- The path taken when the argument is not 0.

## Decisions

1. Each bank is sixteen words supplied by the test, in the order they are written.
2. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.

## Risks / Trade-offs

- Counting the install is not its body. The body reads another table and calls through it.

## Migration Plan

- The mark clear now sets the flag and the code, then selects the bank.

## Open Questions

- What the install writes, and whether a world byte changes the side before the select.

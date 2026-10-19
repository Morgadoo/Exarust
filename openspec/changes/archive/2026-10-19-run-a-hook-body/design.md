# Design

## Context

The gate hook writes two zero words and sets a flag word to 1. When the mid byte is not 0 it counts an alternate hook and does not pick. The pick that follows a clear mid byte is not in.

The watch hook clears a stamp when bit 0x2000 is absent. With the bit, a target, and a ready byte, a held byte of 0 copies the clock. Code 1 clears bit 0x800000 when that bit is set. Code 2 sets it. A pending count above 0 counts a refresh. A leave with a focus counts the focus, clears it, and enters the session when the mark is not 0. The hook returns 1.

## Goals / Non-Goals

**Goals:**

- A gate hook with a set mid byte and an alternate hook counts that hook and leaves the flags at 1.
- A watch without bit 0x2000 stores a stamp of 0.
- Code 1 clears bit 0x800000. Code 2 sets it. A leave enters the session.

**Non-Goals:**

- The pick under the cursor.
- The focus method that runs before the hand words.

## Decisions

1. The ready byte, the held byte, and the watch code are results the test supplies. The checks that produce them are not in.
2. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- The three checks inside the watch hook are results, not their callees.

## Migration Plan

- `run_gate` and `run_watch` are the two bodies. A leave uses the session enter that a cleared mark already uses.

## Open Questions

- None for the flag word, the stamp, and the leave.

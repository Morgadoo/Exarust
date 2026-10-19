# Design

## Context

The platform step passes two runtime words and the shown value to an indirect call. Those words are filled at run time. The target is not in the image.

When a mark changes to 0 and a world is open, the side becomes 1 if the world byte is 0, and 0 otherwise. A lock leaves that side. Otherwise a force replaces the byte with 0, and a byte that differs is stored, selects its bank, and is written back. A non-zero side also clears a span and keeps the world flags masked by 0xfffffdbd. The select after that still runs.

A separate call passes 1. When the mark is not already 1 it stores 1, sets a flag to 1 and a code to 0, clears three notice bytes, installs index 0, clears a hold byte, sets view bit 0x10, and clears a view code. An open world also clears a quiet byte and the flags, and stores code 4 on the child.

## Goals / Non-Goals

**Goals:**

- Value 42 is drawn with owner 7 and extra 8.
- World byte 0 copies the first bank and installs twice. Byte 5 copies the second bank, raises the notice, and masks the flags.
- A force leaves byte 5 and selects the first bank once. A lock leaves byte 0 and selects the second bank.
- Raising the mark installs index 0, shows fallback 9, and does not copy the bank. A second raise changes nothing.

**Non-Goals:**

- Calling the indirect target.
- Callback registration.

## Decisions

1. The two platform words are fields the test supplies.
2. An index of 0 installs without copying either bank.
3. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- Recording the three words is not the platform cursor. The target pointer is empty in the image.

## Migration Plan

- A show now records the two words. A cleared mark reads the world byte when a world is open. `set_session_aside` raises the mark.

## Open Questions

- None for the three words, the side byte, and the call that passes 1.

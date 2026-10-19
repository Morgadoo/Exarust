# Design

## Context

The draw skin is 35 KB and is rated 31% through the wide right side. The next test reads bit 0x40 of the first flag word. When it is set, one ink dword is copied. The path then joins later code.

## Goals / Non-Goals

**Goals:**

- A wide form with style bits 0xc0 equal to 0x80 and bit 0x40 set counts one ink copy.
- A clear bit counts none. Style 0x10 without the wide path counts none.

**Non-Goals:**

- The join after that copy.
- The bit 0x4 compare that the other path uses on the same word.

## Decisions

1. The first flag word is the word that also holds the hold bit.
2. The copy is counted. Its stored dword is not drawn here.
3. `TGUIDrawSkin` stays 31%. Forty bytes do not cross the next point. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is a count, not the pixels of the copied ink. The join is not in.

## Migration Plan

- `dispatch_form` now returns the ink-copy count beside the right side. The UI plugin does not draw it yet.

## Open Questions

- The join after the copy is unread. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.

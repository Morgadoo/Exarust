# Design

## Context

The draw skin is 35 KB and stays 47% through the frame return. The helper that receives the frame releases one string slot. The dispatcher stores that slot as empty before any drawing. The release compares the slot with empty and returns without freeing it. The return jumps over a block that also names that slot. A normal dispatch does not enter that block.

## Goals / Non-Goals

**Goals:**

- The slot starts empty and is released once.
- An empty slot is not freed.
- A held band, a wide form, and a style that is not that band each do so.
- The skipped block does not run.

**Non-Goals:**

- Freeing a slot that is not empty.
- The calls inside the skipped block.

## Decisions

1. The slot is a managed string. Empty means the nil value stored before drawing.
2. The release call still happens. The nil compare is what skips the free.
3. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- The skipped block is real code. A normal return does not run it.

## Migration Plan

- `dispatch_form` now returns the empty slot, the release, and that the skipped block did not run. The UI plugin does not draw with them yet.

## Open Questions

- A slot that is not empty would free through the header before the text. No dispatch in this function stores one. No format page changes: this is draw behaviour, not a file format.

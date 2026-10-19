# Design

## Context

The draw skin is 35 KB and stays 47% through the call counted on the band. That call jumps to the only return. The return passes the frame to one call, restores the saved register, sets the stack to the frame, and pops the frame. A style that is not the band can skip the band quads and still reach this return.

## Goals / Non-Goals

**Goals:**

- Every dispatch counts one call that is passed the frame.
- The saved register is restored, the stack is set to the frame, and the frame is popped.
- A held band, a wide form, and a style that is not that band each do so.

**Non-Goals:**

- The helper body.
- The bytes the jump skips.

## Decisions

1. The count is 1 for every dispatch. The frame is restored on that same return.
2. The helper that receives the frame is not inlined.
3. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is the return. The helper body is not in.

## Migration Plan

- `dispatch_form` now returns the count and the restored frame. The UI plugin does not draw with them yet.

## Open Questions

- The helper that receives the frame is unread past its own frame setup. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.

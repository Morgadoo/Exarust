# Design

## Context

The draw skin is 35 KB and is rated 40% through the clear-tab right-edge quad. The four right-edge arms join, and bit 0x200 then draws one strip. Its x runs from the width minus four times the unit to the width minus twice the unit. Its y runs from six times the unit to the height minus that.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a strip from 80, 30 to 90, 50.
- A clear bit 0x200 stores none.
- A clear hold bit with that bit set stores the wider strip, from 70, 40 to 80, 60 at height 100.

**Non-Goals:**

- The drawing after that strip.
- The helper body on the clear-hold path.

## Decisions

1. The sample is 0.7, 0.5, 0.7, 0.7.
2. The inner x subtracts twice the unit from the outer x. The clear-hold path uses a slot of four times the unit and a drop of eight times the unit.
3. `TGUIDrawSkin` stays 40%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one strip. The drawing after it is not in.

## Migration Plan

- `dispatch_form` now returns the strip beside the right-edge quads. The UI plugin does not draw it yet.

## Open Questions

- The drawing after that strip is unread past its first vertices. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.

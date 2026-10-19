# Design

## Context

The draw skin is 35 KB and is rated 42% through the caps after the bar. A clear tab bit skips that bar and draws one strip on the right. A clear hold bit uses the wider scale.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a strip from 90, 30 to 100, 50.
- A set tab bit stores none.
- A clear hold bit with the tab bit clear stores the wider strip, from 80, 40 to 90, 60 at height 100.

**Non-Goals:**

- The next mask test.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the wide bar sample, 0.3, 0.1, 0.3, 0.2.
2. On a held form the strip runs from the width minus twice the unit to the width, and from six times the unit to the height minus that. A clear hold bit uses four times the unit and eight times the unit.
3. `TGUIDrawSkin` stays 42%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one strip. The next mask test is not in.

## Migration Plan

- `dispatch_form` now returns the strip beside the caps. The UI plugin does not draw it yet.

## Open Questions

- The next mask test is unread past its compare. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.

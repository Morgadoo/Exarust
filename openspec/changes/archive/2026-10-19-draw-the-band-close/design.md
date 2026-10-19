# Design

## Context

The draw skin is 35 KB and stays 46% through the two quads drawn when mask bits 4 and 8 differ. Both of those arms, and the quad they join from when the bits match, then reach one compare. A close mark of -4 draws one square. A clear hold bit is still on that band.

## Goals / Non-Goals

**Goals:**

- Unit 5 and width 100 store a square from 80, 0 to 100, 20.
- The earlier square remains.
- A mark of 0 or -3, a style that is not that band, and a wide bit store none of this square.

**Non-Goals:**

- The call after that square.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the close sample, 0, 0.8, 0, 1. The top edge is 0 and the side is four times the unit.
2. The square is drawn for a held form and for a clear hold bit. It does not depend on the tab bit or the edge mask.
3. `TGUIDrawSkin` is 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one square. The call after it is not in.

## Migration Plan

- `dispatch_form` now returns the square beside the earlier one. The UI plugin does not draw it yet.

## Open Questions

- The call after the square passes 0 and then the function returns. A later pass can take that the same way. No format page changes: this is draw behaviour, not a file format.

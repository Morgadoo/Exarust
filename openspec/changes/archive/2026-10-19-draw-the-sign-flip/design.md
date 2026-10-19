# Design

## Context

The draw skin is 35 KB and is rated 37% through the wider frame. When the two edge bits are not both set, the held form draws a different quad. Each corner that should sit on the negative side flips the sign of twice the unit. The far corner stays at six times the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 with an edge of 5, 5 stores a quad from -10, -10 to 30, 30.
- An edge of 4, 4 stores none, and the cap remains.
- A clear hold bit stores none.

**Non-Goals:**

- The mask test that follows that quad.
- The helper body on the clear-hold path.

## Decisions

1. The sample is 0.15, 0.2, 0.15, 0.6.
2. Negating twice the unit is the sign flip. Six times the unit is not flipped.
3. `TGUIDrawSkin` stays 37%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one quad. The next mask test is not in.

## Migration Plan

- `dispatch_form` now returns the quad beside the cap. The UI plugin does not draw it yet.

## Open Questions

- The next mask test is unread past its first compare. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.

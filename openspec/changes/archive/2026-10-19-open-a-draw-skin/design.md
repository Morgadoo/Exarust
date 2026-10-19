# Design

## Context

The draw skin is 35 KB and was rated 0%. Its constructor stores four spans from the GUI unit, four fixed pads, one colour copied three times, and a mark of 1. Argument 1 counts an allocation first. A null result skips the fill. Any non-zero argument counts the call after the fill.

## Goals / Non-Goals

**Goals:**

- Unit 5 stores spans 10, 5, 10 and 10, pads 8, 4, 8 and 8, and the colour 255, 250, 230, 100 three times.
- Argument 0 does not count the call after the fill. Argument 1 does, when the allocation succeeds.

**Non-Goals:**

- The base fill's body.
- The slider, the frame, the icon, and the widget dispatcher.

## Decisions

1. The colour is four bytes, copied as one dword twice more.
2. `TGUIDrawSkin` is 1%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- One constructor is about 300 bytes of 35 KB. The rest of the skin is not in.

## Migration Plan

- `open_skin` is the constructor. The UI plugin does not draw with it yet.

## Open Questions

- None for the unit spans and the colour bytes.

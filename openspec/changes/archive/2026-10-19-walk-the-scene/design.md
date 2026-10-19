# Design

## Context

Inside the view, sectors that share a camera bit are walked. A body needs flags 0x89, a shared camera bit, and no bit 0x4000000. The probe chooses the bits of 0.1 or of 5. A hit count above 0 stores the body and one less than the count. A later count replaces it. An empty scene leaves the supplied hit, so earlier tests still classify.

A new pane doubles the row one before the slot and caps it when greater. Bit 0x1000 uses the wide slot. The inner height is the unit times 6 plus the rows. The other inner sizes are twice the margin plus each span word times the unit. The outer width and height are the sizes passed to an empty pane. The place uses those sizes. An index of 0 does not read before the table.

## Goals / Non-Goals

**Goals:**

- The last positive body is the hit.
- A wide slot of 2 with a cap of 10 stores 8 rows.
- A place past the screen becomes the screen width minus the pane.

**Non-Goals:**

- The ray distance.
- Child layout.

## Decisions

1. An empty scene does not replace the supplied hit.
2. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- The triangle test is a supplied count. Distance does not reject a later body.

## Migration Plan

- `run_gate` walks a non-empty scene before it classifies, and a build stores the pane size.

## Open Questions

- None for the masks, the pads, and the size.

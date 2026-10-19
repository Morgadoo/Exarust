# Design

## Context

The pick converts the cursor to a float and rejects a point below 0 or at or past the view. Inside the view the scene walk is still a supplied hit. A marked item then releases: a count above 0 drops and becomes 0. A count of 0 or less clears a pointer, and byte 1 counts a close first. The open walks panes. The first tag 0xce1c2013 with that item is shown. A later pane is not visited. With no pane, a marked span counts a build. The builder is not in.

## Goals / Non-Goals

**Goals:**

- Cursor 8 on a view of 8 clears the hit and does not release.
- A count of 3 becomes 0 and counts one drop.
- Byte 1 on a pointer counts one close and clears the pointer.
- The first matching pane is shown.

**Non-Goals:**

- The scene walk.
- The pane builder.
- The close and drop callees beyond the count.

## Decisions

1. The view test uses the integer cursor the gate converts. A 1 by 1 view keeps cursor 0 inside, so earlier tests still hit.
2. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- Inside the view, the hit is still supplied. The ray against the scene is not in.

## Migration Plan

- `run_gate` tests the point before it classifies, and a kind mark releases and opens.

## Open Questions

- None for the view edge, the held count, and the pane tag.

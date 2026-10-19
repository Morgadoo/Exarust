# Design

## Context

The segment test measures a centre against a ray from the origin along z. The ray starts at length 4000. A centre past the end misses unless the end still lies in the radius. A taken hit shortens the ray to that centre's z. A later centre beyond the new length is left.

A pane's previous size is 0 on the first build. A stretch child adds the new size. An anchor child moves by the same change. A centre child is placed at half the spare. Nested children are not visited.

## Goals / Non-Goals

**Goals:**

- A centre at 100 is kept and a centre at 500 is left. The reach is 100.
- A centre at 5000 misses.
- A centre child of size 10 by 4 sits at 9, 27 in a pane of 28 by 58.

**Non-Goals:**

- A ray that is not along z.
- Nested children.

## Decisions

1. Centres and radii are whole numbers. The segment test uses the same compares as the ray.
2. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- The ray is along z from the origin. A free camera direction is not in.

## Migration Plan

- `run_gate` shortens the ray inside `walk_scene`, and `build_pane` lays out one level of children.

## Open Questions

- None for the length 4000 and the child flags.

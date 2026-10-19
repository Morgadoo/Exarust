# Design

## Context

The footprint stamp writes 63, 32 and 16 onto the penalty. The search adds that penalty to the cost of each cell it enters, so a body pays for its own neighbours. The mover keeps rectangles of floors that have to be raised again. Nothing queues those rectangles yet.

## Goals / Non-Goals

**Goals:**

- Lift the standing footprint for the search and stamp it again.
- Merge a rectangle when the union padded by six is strictly smaller, and keep sixteen.
- Grow each side by three cells, raise floors from samples, and rerun the cost passes when a floor changes.
- Cover a cell and a neighbour from a zero-extent box, inside the margin of three.

**Non-Goals:**

- The countdown that installs one rectangle over the whole grid. Its writer was not found.
- Walking a scene graph for the samples. The caller passes them.
- The rule that follows a path only past 150 cm. That already ships.
- Building the box from a rotated extent. The caller passes the half extent.

## Decisions

1. The plan calls the shipped removal, the shipped search, and the shipped stamp, in that order. The search cost is the cost of the goal cell, so a lifted footprint is cheaper when the path enters a stamped neighbour.
2. A rectangle's size is the inclusive cell count. The padded area is that size plus six, on each axis. The first existing rectangle whose union is strictly cheaper absorbs the new one. Past sixteen, the oldest is dropped.
3. The box is the cell of each edge, two and a half cells out from the half extent, clamped to the margin of three.
4. Working floors start as the stored floors. A sample from 1 to 254 raises the lower floor when it is 0 or the sample is less than fifteen above it, and otherwise raises the upper floor when the sample is higher. A sample at or under the lower floor and its underside raises an empty upper floor. A changed floor is written, and the cost passes run on that grown rectangle. A rectangle too small for a pass is left alone.

## Risks / Trade-offs

- One queue of sixteen stands in for the ring and the list. A seventeenth separated rectangle drops the oldest.
- The cost passes read neighbours of the grown rectangle. A rectangle on the outer cells can still do so, because each pass starts one cell further in.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- The countdown's writer was not found, so the full-grid refresh is not in this slice. The lift, the stamp, the merge, the three-cell grow and the floor raise were read before this change.

# Design

## Context

The dismiss walk calls close on each marked child and passes 0. Close returns at once when the child is already closed. A pinned child is marked closed and a notice is raised, and the close continues. Its own children are then closed from the first to the last, skipping one that is already closed, with the same argument. The close callback runs after those children.

A separate branch handles the task's active form, and another walk closes the task's other children that point at this form. A style bit releases a capture. The close then detaches and asks for a redraw. Those calls are not in this step. The call after the two dismiss lists is also not in.

## Goals / Non-Goals

**Goals:**

- `begin_match` closes a marked child before the specialist leave.
- A child named A, then an already closed child, closes A and then the parent. The already closed child's callback does not run.
- A pinned parent raises the notice and still closes A.
- An already closed marked child is still dismissed, and its child stays open.
- Each close that proceeds records argument 0.

**Non-Goals:**

- The capture release, the task-active shortcut, the task's other children, detaching, and redraw.
- The call after the two lists.

## Decisions

1. `closed` is the early-return byte. `pinned` is the byte that marks it and raises `notice`.
2. `close_log` records callback names in the order they run.
3. `TGUIForm` moves from 0% to 1%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.

## Risks / Trade-offs

- The close is a subset. A task-owned sibling is not closed.

## Migration Plan

- The dismiss walk calls close. The flag mask and the leave stay as they are.

## Open Questions

- None for the early return, the pinned byte, and the own-child walk.

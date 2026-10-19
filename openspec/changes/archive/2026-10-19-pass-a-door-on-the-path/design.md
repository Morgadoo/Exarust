# Design

## Context

A toggle already chooses open or close from how open a door is. The path step is what decides to send that toggle.

## Goals / Non-Goals

**Goals:**

- Hold the walker at a door whose openness is at most 0.7, and toggle it unless it is already toggling.

**Non-Goals:**

- The rest of the path.

## Decisions

1. No path, a non-positive waypoint count, a cursor at or past the end, or a missing door lets the walker continue.
2. Openness is the same measure the toggle uses. Above 0.7, and a non-finite openness, the walker continues and the door is unchanged.
3. At or below 0.7 the walker holds. The toggle runs only when the door's command is not already the toggle command.

## Risks / Trade-offs

- The walker is not yet called from a placed character. The function is the step's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The path step was read before this change.

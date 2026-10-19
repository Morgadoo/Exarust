# Design

## Context

The cloth tick falls, blows, pins and shortens edges. It does not meet a body or the level.

## Goals / Non-Goals

**Goals:**

- Push a vertex out of a sphere and a plane, sleep a still cloth, rustle a fast one, and read the mesh from the particles.

**Non-Goals:**

- Testing a swept sphere against a cloth triangle. The push is per vertex.

## Decisions

1. Padding is 2 cm, the shipped record.
2. Friction scales only the tangential part of the push. The three factors are available; the body class for each was not read.
3. An asleep cloth skips the fall. A push sets the countdown back to at least 1.

## Risks / Trade-offs

- The triangle sweep in the original is larger than this vertex push.

## Migration Plan

No data migration.

## Open Questions

Which of the three frictions belongs to which body was not read.

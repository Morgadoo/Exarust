# Design

## Context

A resting sphere and an infinite plane already push a vertex. A moving sphere can pass a triangle between ticks, and a vertex can pass a finite floor the same way.

## Goals / Non-Goals

**Goals:**

- Hit a cloth triangle with a swept sphere, and stop a vertex on a level triangle. A miss does neither.

**Non-Goals:**

- Pushing the body that owns the sphere, and the cloth render hooks.

## Decisions

1. The segment is tested against the triangle. The face normal points back toward the side the motion came from.
2. The sphere's contact centre sits one radius out from the hit, and the existing vertex push supplies the 2 cm padding.
3. An asleep cloth still receives sweeps, so a passing body can wake it.

## Risks / Trade-offs

- The original also broad-phases triangles against the body's box. This tests every listed triangle.

## Migration Plan

No data migration.

## Open Questions

None for this slice.

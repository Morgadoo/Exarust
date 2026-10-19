# Design

## Context

A hinged door already creaks, counts a pull, and runs a motor. Two leaves of one doorway are partners. While one is open, the other is not held against its frame.

## Goals / Non-Goals

**Goals:**

- Decide whether a shut leaf stays shut and whether it is driven toward closed.
- Say when a leaf counts as near closed.
- Mark a paired leaf so the two leaves pass through each other.

**Non-Goals:**

- The lock, the slam's random pitch, and moving the stop particle.

## Decisions

1. A single door leaves the shut state only when its angle is strictly more than 0.05 rad short of closed and it is not latched. It is still held.
2. A paired door whose partner is not near is not held. It leaves the shut state when its own angle is strictly short of closed. A near partner keeps it shut and held.
3. Near means within 0.1 rad on the short side, strictly inside that gap, or any angle past closed. A non-finite angle is not near.
4. A paired leaf ORs the pair bits `0x3000` into its layer. An unpaired leaf keeps the layer.

## Risks / Trade-offs

- The placed door does not yet call this each tick. The function is the partner's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The partner check was read from the hinged door step before this change.

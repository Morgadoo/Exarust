# Design

## Context

A signal of 0 asks for close-and-lock. The walk finds each door under the node and drives it.

## Goals / Non-Goals

**Goals:**

- Start the shut drive on a motorised hinge and on a fast sliding leaf.
- Latch a leaf that is already shut, and play the lock sound on an unlatched sliding leaf.

**Non-Goals:**

- The lock sound's random pitch.

## Decisions

1. Only a motorised hinge writes fraction 1. A slow one counts 300 ticks and a fast one 100.
2. A sliding leaf never writes the fraction here. It plays the lock sound only when it is not already latched, and only a fast one starts the 100-tick drive.
3. The latch is applied after the drive, so a shut fast slider both drives and latches.

## Risks / Trade-offs

- The placed door does not yet call this. The function is one leaf's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The close was read before this change.

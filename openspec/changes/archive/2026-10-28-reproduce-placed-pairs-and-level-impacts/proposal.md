# Proposal

## Why

Phase 3 (Bodies: skeleton, physics, control) of `docs/PLAN.md`: the
simulation sector's second pass (TSimSector) reproduced the placed pairs,
the impact's smoothed travel, the level impacts, the registration's impact
scales and kinds, and the slider links' smoothing. The review checked each
against the original's instructions and corrected the placed query, the
placed offset's weights, the level travel's weights, the registration kinds
and the slider smoothing's reach.

## What Changes

- A pair with exactly one placed body (placed = settling countdown above
  zero) runs the static query only, moves only the placed side and records
  the opposite push in its offset array; it never strikes.
- The pair impact's travel is the parts' smoothed velocity (a tenth of the
  way to each tick's displacement) at the contact weights.
- A part's level hits make a level impact record (gates 0.1 and 0.1).
- Every slider link, a root's to the world included, takes a 0.01
  smoothing at conversion.
- The registration walk: a body with the simulated-object bit joins the
  object list, a 0x89 mesh node with class bit 0x200000 the static list.

## Evidence

The original's placed pair pass and callback, the dynamic callback, the
level callback and detection, the integration's smoothing (0.1, read from
the executable's constant), the link conversion and the registration walk,
in assembly (`context/re/packs/simsector/`, `context/re/exe.asm`).

## Out of scope

Sphere-against-mesh contacts, the offset array's reader (a placed node's
controller), the level impact's consumer (sound and damage) and its state
bit 0x800 gate, the static grid's live use, simulated-object lists.

## Capabilities

### Modified Capabilities

- `physical-body`: placed pairs, smoothed impact travel, level impacts,
  slider smoothing, registration kinds.

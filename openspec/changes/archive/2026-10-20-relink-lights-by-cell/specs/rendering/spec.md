# Spec Delta

## ADDED Requirements

### Requirement: Moved lights relink nearby casters
A light that moves SHALL be relinked only against the casters whose sphere can reach it (found by the 6 m cell of their centre, the casters wider than a cell, and those already linked to it), with the same links as testing every caster.

#### Scenario: Carried torch
- **WHEN** the player walks with a torch in exanima01
- **THEN** each frame tests the torch's links against the casters near it, not every caster of the level, and the shadows are unchanged

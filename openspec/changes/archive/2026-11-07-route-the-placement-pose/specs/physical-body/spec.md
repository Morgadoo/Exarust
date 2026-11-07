## ADDED Requirements

### Requirement: Placement pose routing
A placement on the ground SHALL pose each part as the pose-only sampler routes it: the thighs' clip turns carrying the femur posture before the chain, the torso's midline parts but the head turned by the twist (the part under the root half way), the head on the untwisted chain, and the part with the support pins taking the thorax hunch when the feral level is over 0.1. No leg IK, look or foot pitch SHALL run there. A first spawn SHALL take the plain clip chain.

#### Scenario: A feral body re-enters
- **WHEN** a feral body with a hunch is placed on the ground on re-entering the scene
- **THEN** its thorax is placed with the hunch and its head and arms on their clip chain

#### Scenario: A body in combat
- **WHEN** a body whose behaviour is in combat and which stands is placed on the ground
- **THEN** its thighs take the combat femur turn

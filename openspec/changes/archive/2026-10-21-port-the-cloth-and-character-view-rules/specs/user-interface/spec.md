## ADDED Requirements

### Requirement: Character preview turn
The system SHALL turn a preview figure each tick toward the pointer: the target yaw is 0.7 of the engine's arc-cosine yaw of the horizontal direction to the pointer (product in double), clamped to 1.5 rad in magnitude; the gap to the figure's yaw is wrapped by 2 pi (compared in double); a gap of at most 1 rad moves the yaw by 0.05 of the gap and a larger one by 0.05 rad.

#### Scenario: Pointer to the side
- **WHEN** the pointer is straight to the figure's side and the yaw is 0
- **THEN** the yaw becomes 0.05

### Requirement: Character preview head
The system SHALL build the cursor direction from the new yaw with the fast sine and cosine and the stored pitch as its height, normalise it, blend the stored aim vector 40 % of the way toward it, zero the blend's height, ease the head's look 5 % toward the result and normalise the look.

#### Scenario: Aim to the side, pointer ahead
- **WHEN** the aim is (1, 0, 0), the pitch is 5, the yaw is 0 and the look is (0, 0, 1)
- **THEN** the look has no height and leans toward +x

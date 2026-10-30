# Spec Delta

## ADDED Requirements

### Requirement: Wake order
The knock-out wake SHALL run in the body tick after the behaviour update and before the posture decision, so the posture decision reads the condition the wake wrote.

#### Scenario: Revived and lying
- **WHEN** a revived body is within its knock-out's first 1.5 s
- **THEN** the wake writes down before the posture decision runs, and the body stays down

### Requirement: Held wake release
While the player's state flags hold the wake (0x40), the player's behaviour SHALL clear that bit alone when a mouse button (left or right) or a movement key is held, or when the move stick's squared length is over 0.1 in double precision.

#### Scenario: A click after the story revive
- **WHEN** the revived player's flags are 0x70 and the left button is held
- **THEN** its flags become 0x30

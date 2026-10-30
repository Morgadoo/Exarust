# Spec Delta

## ADDED Requirements

### Requirement: The step clear on entering combat
A duellist entering combat SHALL clear its queued step with its body's own speed along its heading (cm a tick), so a move point farther than 8 × speed + 20 cm is put that far straight ahead of the body.

#### Scenario: A walking body enters combat
- **WHEN** a body moving at 2 cm a tick with a move point 100 cm away enters combat
- **THEN** the move point is 36 cm ahead of the body

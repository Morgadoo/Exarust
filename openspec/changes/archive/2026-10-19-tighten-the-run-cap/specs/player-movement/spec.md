# Spec Delta

## ADDED Requirements

### Requirement: The sprint latch tightens the run cap
While the sprint latch is kept, the run blend SHALL stop at 1 minus 0.4 times encumbrance. The walk keys SHALL stop at 1 minus 0.3 times encumbrance. Encumbrance 0.5 with Shift and the cursor 4 m ahead SHALL reach 0.8 and keep the latch. Shift with the walk key and no cursor SHALL reach 0.85 and spend the latch.

#### Scenario: Cursor 4 m ahead, then the walk key
- **WHEN** a player of encumbrance 0.5 holds Shift with the right button and the cursor 4 m ahead for 40 steering ticks, then releases the button, presses the walk key, and is steered for 40 more ticks
- **THEN** the first tops the run blend at 0.8 and keeps the sprint latch, and the second tops it at 0.85 and spends the latch

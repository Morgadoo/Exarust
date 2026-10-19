# Spec Delta

## ADDED Requirements

### Requirement: Door unlock
An unlock SHALL clear the saved latch and start a wait of 1000 ms on the door and on its partner, while both stay latched. The partner SHALL keep its own saved latch. At the exact wait both SHALL stay latched. Once the clock is strictly past the wait, both latches SHALL open.

#### Scenario: A one second wait, then both leaves
- **WHEN** an unlock starts at time 0 on a latched door with a latched partner, the clock is then 1000, and then 1001
- **THEN** the saved latch is clear, the partner's saved latch remains, both stay latched through time 1000, and at time 1001 both latches are open

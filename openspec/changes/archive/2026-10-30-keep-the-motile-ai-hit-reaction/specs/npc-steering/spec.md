# Spec Delta

## ADDED Requirements

### Requirement: The grabbed flag's timeout
The AI tick's grabbed flag SHALL stay set while the body reports the grabbed condition, unless the brain is not in the combat state and the body's hook answers event 0x10 under zero, which SHALL release the grip and clear the flag. Outside that condition the flag SHALL clear when the grab time plus 500 ms, summed without a wrap, is below the clock.

#### Scenario: Exactly 500 ms
- **WHEN** a grab began at 1000 ms and the clock reads 1500 ms outside the grabbed condition
- **THEN** the flag holds, and at 1501 ms it clears

### Requirement: The investigation's goal
The investigation's goal callback SHALL take the pathfinder's nearest walkable point to the body as the goal, with motion mode 20 and a 100 cm radius.

#### Scenario: A walkable point
- **WHEN** the pathfinder answers (10, −5, 2)
- **THEN** the goal is (10, −5, 2) with mode 20 and radius 100 cm

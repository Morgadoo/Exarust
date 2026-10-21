# Spec Delta

## MODIFIED Requirements

### Requirement: Left press
The hand SHALL act on the left button's drag start, picking at the pressed point. A drag start SHALL be ignored while combat mode keeps the button for attacks (unless Ctrl is held or something is held), while the point under the cursor is out of reach, while the player is down, or when the pick hits nothing. Each drag start SHALL begin from nothing held.

#### Scenario: A busy hand
- **WHEN** a corpse is pressed in combat mode without Ctrl, out of reach, and with the player down
- **THEN** each press is ignored

## ADDED Requirements

### Requirement: Drag start
The drag SHALL start once the pointer, the left button held, has moved from the pressed point by more than 2 pixels on either axis, or, within the first 190 ms after the press, by more than 2 × ((200 − t) / 20) pixels with t the milliseconds since the press and the division an integer one.

#### Scenario: The allowance
- **WHEN** the pointer moves 20 then 21 pixels at once, 2 pixels by 2 then 3 pixels after 500 ms, and 1 pixel after 185 ms
- **THEN** 20 does not start the drag and 21 does; 2 does not and 3 does; 1 pixel after 185 ms does

### Requirement: Click turn
A left release before the drag starts SHALL be a click; outside combat mode a click SHALL turn the player's heading to the cursor's last direction when the cursor's last distance is over 70 cm.

#### Scenario: Turning
- **WHEN** clicks are made with the cursor 70 cm and 70.1 cm away
- **THEN** the first does not turn the player and the second does

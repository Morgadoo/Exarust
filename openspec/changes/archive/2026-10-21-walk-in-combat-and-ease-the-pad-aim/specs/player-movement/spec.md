## ADDED Requirements

### Requirement: Combat walk to the cursor
In combat mode, while the right button is held, the player SHALL ask the walk to the cursor however near it is, the cursor showing its chevrons. With the cursor 50 cm or more away the request SHALL keep only its sprint latch and its move, take the cursor's direction as heading and look, run when the cursor is beyond 3 m or turn the sprint latch into the run, and walk free; the footwork SHALL give way, and the release SHALL end the walk.

#### Scenario: Walking, running and the sprint latch
- **WHEN** the button is held with the cursor 2 m ahead, then 4 m to the side, then 2 m ahead with the sprint latch, then 20 cm away, then released
- **THEN** the body walks ahead with the move bit alone, runs to the side, runs ahead with the latch cleared, asks the walk without moving, and asks nothing

### Requirement: Combat walk look-ahead
While no foot steps, the combat walk SHALL cast the look-ahead 6·v² + 45 cm along the facing; a hit SHALL stop the free walk and, within 100 cm, the run and the sprint.

#### Scenario: Open ground
- **WHEN** the combat walk's look-ahead meets nothing
- **THEN** the walk stays free

### Requirement: Gamepad aim ease
A gamepad aim whose x² + z² is over 0.01 SHALL place the aim point 250·|aim| + 50 cm out along it, and the aim cursor SHALL move (0.4·AimSpeed + 0.1)² of the way toward it each behaviour tick (in double precision, then single), starting from the mouse's point; combat mode SHALL aim along it.

#### Scenario: The shipped AimSpeed
- **WHEN** AimSpeed is 0.29 and the point is 100 cm away
- **THEN** the cursor moves 4.6656 cm in a tick

### Requirement: Mechanism drag holds the hand
A drag start on a door, a wheel or cloth SHALL hold the hand until the release without keeping the movement request; a grabbed box SHALL be refused when a placed corner is higher than 280 cm or lower than −1000 cm, an unordered height passing.

#### Scenario: A door and a box
- **WHEN** the hand holds a door's drag, and boxes with corners at 280 cm, 281 cm, −1001 cm and NaN are tested
- **THEN** the request is not kept, and the boxes at 280 cm and NaN are taken, the others refused

### Requirement: Idle camera gates
A screen that blocks the controls SHALL turn the camera once the clock is ten seconds past the stamp whatever the pointer and the buttons do; the active view not being the game's (the in-game menu) SHALL only restamp the clock; a dialogue or the power bar SHALL keep the player busy.

#### Scenario: Blocked, a window and a dialogue
- **WHEN** at 20 000 ms a blocked frame comes with the pointer moved and a button held, a dialogue frame comes, and a window frame comes
- **THEN** the blocked frame turns the yaw by 2 rad and stops reading the cursor, the dialogue restamps the clock, and the window restamps it without touching the last frame's time

### Requirement: Behaviour aim lock
The aim height SHALL be a target point's within the pixel radius 80 cm make at its head's depth and blend by (d − r)/r within twice it; without a target a picked point above the plane's and under the ground plus 200 cm SHALL replace the aim point.

#### Scenario: The lock
- **WHEN** a bone is 15 px from the cursor with a 10 px radius
- **THEN** the aim height is halfway between the plane's and the point's

### Requirement: Player contact list
The contact list SHALL hold every character within 12 m, up to twelve, with closeness 1 − d·0.00083333335 capped at 0.99 and facing weight 0.75·(0.5 − 0.5·facing·dir) + 0.25·(0.5 − 0.5·cursor·dir)², each scoring half of both; a new character SHALL replace the lowest it outscores.

#### Scenario: Ahead and beside
- **WHEN** a character stands 100 cm ahead of the body and the cursor, and another ahead of the body with the cursor turned aside
- **THEN** the first's closeness is 0.9166667 with no facing weight, and the second's facing weight is 0.0625

### Requirement: Strongest engaged enemy
The strongest engaged enemy SHALL be the largest threat among the living characters within reach plus 70 cm, in front of the heading by more than 0.3 of their distance, whose own contact score for the player is over 2.

#### Scenario: An engaged and a calm enemy
- **WHEN** an enemy with threat 0.7 and a score of 2.5 for the player stands 100 cm ahead of a player with 60 cm of reach, and one with a score of 2
- **THEN** the first is found with threat 0.7 and the second is not

### Requirement: Attack timing
After a swing's tick the wind-up step SHALL be 1 − 0.05·√E (1 at no energy), at least 0.1, over the weapon's rate times 0.03, at least 0.01; the next attack SHALL come round(1000/(80·step)) ms on, at most the swing's length; the recovery SHALL end round((2f + 1)·(round(100·rate) + 100)) ms on, with f = (3 − 2b)·b², b = balance/0.7, while a swing's leg mode is on and the balance at most 0.7, else 1.

#### Scenario: Full energy and a poor balance
- **WHEN** a fighter of energy 1 swings a weapon of rate 0.1 at balance 1, and one of energy 0.5 at balance 0.5 in leg mode 5
- **THEN** the first attacks again 44 ms on and recovers 330 ms on; the second recovers 286 ms on

### Requirement: Item use values
A pour SHALL warm the character by 0.2, or 0.1 for a liquid of type 50, in double precision; a cleaning kit SHALL take a dirt byte over 0x20 to 0x20; a repair kit SHALL lower a wear byte by 255 over (rating + 5)/10 (at least 1), never under zero, and leave an unworn item; a kit's value SHALL move 0.0005 of its gap a millisecond, rounded to the nearest (ties to even), the timed list dropping an arrived entry by moving the last into its place.

#### Scenario: Kits over time
- **WHEN** a wear of 100 is repaired with a rating of 25, and a timed value runs from 100 to 0 for 1000 ms and 2000 ms
- **THEN** the target is 15, and the value is 50, then 0 and done

## ADDED Requirements

### Requirement: Combat-mode setter
The practice start SHALL turn the player's combat mode on right after it attaches the controls, each next practice fight SHALL turn it on without touching the controls, and the story revive SHALL turn it off before it raises the body.

#### Scenario: The next fight
- **WHEN** the next practice fight starts
- **THEN** combat mode goes on and no controls step is made

### Requirement: Lie-down to wake
A new story's unknown background SHALL lie down to wake 3000 ms after its first motion, and the story respawn SHALL lie down with no delay after it attaches the controls and before its get-up motion; a lie-down SHALL write the condition 0x200, replace the flags' wake nibble with 0x30 and set the wake clock to the game clock plus the delay.

#### Scenario: A lie-down at 1200 ms
- **WHEN** a player with flags 0x600 lies down with a 3000 ms delay at game clock 1200 ms
- **THEN** its condition is 0x200, its flags 0x630 and its wake clock 4200 ms

### Requirement: Story revive body
The story revive SHALL raise a dead player body (yellow health half the red, laid down at once), then set the held bit 0x40 and push the wake clock 2000 ms on, then enable the controls; a living body SHALL NOT be raised.

#### Scenario: A revive at 1200 ms
- **WHEN** a dead player with red health 0.5 is revived at game clock 1200 ms
- **THEN** its yellow health is 0.25, its flags 0x70 and its wake clock 3200 ms

### Requirement: Live wake tick
A body carrying a knock-out SHALL run the wake tick each character tick while its flags hold 0x20, and SHALL clear the wake nibble once its readiness reaches 1 unless it is held.

#### Scenario: A body ready to stand
- **WHEN** a knocked-out body's readiness reaches 1
- **THEN** its flags no longer hold 0x20 and its stance factor is the race's

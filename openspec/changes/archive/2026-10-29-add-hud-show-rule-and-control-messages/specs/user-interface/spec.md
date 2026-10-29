## ADDED Requirements

### Requirement: HUD bars show rule
The HUD's bars SHALL show in a frame where the yellow health differs from the stored value (a NaN on either side shows), the focus is strictly below the stored value (a NaN does not show), the power bar is open, the player is in combat mode, or the game clock is below the hold clock compared unsigned; the stamina gauge SHALL show while the player holds a shield in combat mode.

#### Scenario: A hold clock past 2^31
- **WHEN** nothing changed, the clock is 1000 and the hold clock is 0x80000000
- **THEN** the bars show

#### Scenario: A NaN focus
- **WHEN** only the focus became NaN
- **THEN** the bars do not show

### Requirement: Counter and bar control messages
A counter SHALL answer message 1 with its segments' draw, message 2 with the skin's control draw, and message 0x200 by setting its value, writing its percentage caption, running its callback and answering 3; a bar control SHALL answer message 1 with its frame and message 2 with the bar two pixels in, one down and four narrower; every other message SHALL answer 0.

#### Scenario: A value set
- **WHEN** a counter gets message 0x200 with 0.5
- **THEN** it answers 3 with the caption "50.0%" and runs its callback

### Requirement: Deferred inventory fill
The deferred inventory fill SHALL fill the inventory and leave the GUI job queue once the character generator has the character's node, and SHALL stay queued while it has none.

#### Scenario: No node yet
- **WHEN** the generator has no node
- **THEN** nothing fills and the job stays queued

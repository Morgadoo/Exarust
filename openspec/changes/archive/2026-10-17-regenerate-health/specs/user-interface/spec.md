## MODIFIED Requirements

### Requirement: Health bar fade
The system SHALL show the bars while combat mode is on and in a frame where the yellow health changes or the focus drops, fading in over 500 ms, then holding 200 ms and fading out over 2 s (game time); fills SHALL blend at the fade squared, frames and shadows at the fade. With a shield in combat mode the block-stamina gauge SHALL show above them by the same rule.

#### Scenario: Combat mode on and off
- **WHEN** Tab turns combat mode on and, later, off
- **THEN** the bars appear over half a second and, after combat mode ends, stay 200 ms and fade out over 2 s

#### Scenario: Exploring
- **WHEN** the player walks outside combat mode, unhurt
- **THEN** no bars are drawn

#### Scenario: Healing
- **WHEN** the player, wounded, turns combat mode off and its yellow health regenerates
- **THEN** the bars show while the yellow segment refills over the brown, and fade out (200 ms, then 2 s) once it reaches the red health

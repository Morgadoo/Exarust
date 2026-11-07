## ADDED Requirements

### Requirement: Story companion facing
The story companion SHALL face the story start's facing word taken as radians: the start passes -90 unconverted to a spawn record that takes radians.

#### Scenario: Derrin's facing
- **WHEN** a story begins with the first background and exanima01 is in
- **THEN** Derrin's yaw is -90 rad, not -90°

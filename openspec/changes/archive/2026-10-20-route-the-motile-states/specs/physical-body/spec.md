## ADDED Requirements

### Requirement: Motile route order
The system SHALL check the collapse gate, the shock bit, the posture split and the posture leaves before the clip branches.

#### Scenario: Gate and shock
- **WHEN** bit 0x800 is set
- **THEN** the route is Skip
- **WHEN** bit 0x40 is set
- **THEN** the route is Shock

### Requirement: Motile posture routes
The system SHALL route condition 0x300 to get-up, balance below 0.3 with motion below 25 to lying still, an upright lean to upright, the stagger window to stagger and otherwise to fall.

#### Scenario: Leaves
- **WHEN** balance is 0.29 and motion 0
- **THEN** the route is LyingStill
- **WHEN** balance is 0.59 and the lean is upright
- **THEN** the route is Upright

### Requirement: Motile clip routes
The system SHALL route clips 5–12 to attack, tone below 0.85 to weak, clip 0x0D to kick, parrying to parry, clip 4 to parry-end, clips 0x22–0x28 to power, clips 0–1 to gait, clip 0x1F to switch and the rest to standing.

#### Scenario: Branches
- **WHEN** the clip is 5
- **THEN** the route is Attack
- **WHEN** the tone is 0.8 and the clip 3
- **THEN** the route is Weak
- **WHEN** the clip is 0x0D
- **THEN** the route is Kick

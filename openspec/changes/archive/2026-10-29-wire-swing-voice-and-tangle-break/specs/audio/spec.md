## ADDED Requirements

### Requirement: Swing voice
The system SHALL play a fighter's swing on the voice of its first held item: restarted on a `SwingHA` variant, silent, at 0.4 × the primary striker's swing pitch on the first attack tick in the swing's region, and again only after leaving it. Every attack tick after the start in the region SHALL set its volume and pitch by the swing sound rule from the tracked speed. A fighter holding no item SHALL have no swing voice.

#### Scenario: A sword swing's voice
- **WHEN** a fighter holding a sword reaches 30 % of its attack clip
- **THEN** its voice starts silent at 0.4 × the sword's swing pitch and the same tick sets its volume and pitch from the tracker, with no second start in the region

#### Scenario: An unarmed swing
- **WHEN** an unarmed fighter attacks
- **THEN** no swing voice is cued

### Requirement: Swing voice fade and rustle
Every body tick that is not an attack SHALL fade a playing swing voice by 0.01 of volume and 0.0005 of pitch and stop it under 0.005. A swing voice's start SHALL rustle the fighter's clothing foley once its foley clock has passed, then hold the clock 400 ms plus a jitter under 100.

#### Scenario: The voice fades after the attack
- **WHEN** a fighter's attack ends while its swing voice still plays at volume 0.5
- **THEN** each following body tick lowers it by 0.01 until it stops under 0.005

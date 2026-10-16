## MODIFIED Requirements

### Requirement: Blow reactions
The system SHALL take the yellow and red health bars by a resolved blow, daze the victim, shove it on heavy impact, and make the body go limp when yellow health falls below 0.01. A blow that does not kill SHALL NOT start a reaction clip on the struck fighter; its stance changes come from its balance. A fighter struck during an attack SHALL stop that attack and return to its stance.

#### Scenario: Knock-down
- **WHEN** a fighter's yellow health falls below 0.01
- **THEN** its body goes limp and falls under the blow, it stays down, and it is no longer attacked

#### Scenario: Cut on the arm
- **WHEN** a bare forearm of a fighter in its stance takes a sword cut and the fighter stays up
- **THEN** it keeps playing its stance clip

#### Scenario: Struck mid-swing
- **WHEN** a fighter is cut during an attack
- **THEN** that attack deals no further hit and the fighter returns to its stance

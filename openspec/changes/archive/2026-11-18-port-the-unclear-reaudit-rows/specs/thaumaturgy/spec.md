## ADDED Requirements

### Requirement: Refused AI cast
An AI cast the power system does not start (any answer but 1) SHALL clear the duellist's casting flag at once.

#### Scenario: A refused cast
- **WHEN** a duellist's chosen power is refused
- **THEN** its casting flag is clear before its next combat tick

### Requirement: Shell break sound
A force shell that names a break set SHALL play it once when it is asked to close, at its body, volume 0.4 and pitch 0.4r + 0.6 with r the effect stream's draw; the Shell power's aura names `thmfrc_brf`.

#### Scenario: The Shell power ends
- **WHEN** the Shell power's aura is asked to close
- **THEN** `thmfrc_brf` plays once

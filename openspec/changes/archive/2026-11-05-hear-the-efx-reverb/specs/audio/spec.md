# Spec Delta

## MODIFIED Requirements

### Requirement: Reverb parameters
The system SHALL keep one global EAX reverb with the original's stored parameters and a decay of 1.49 s plus the level's decay offset, uploaded while the reverb is on; the effect SHALL keep its previous decay when the new one is outside 0.1 to 20 s.

#### Scenario: Level decay
- **WHEN** a level's decay offset is 0.51 s and the reverb is on
- **THEN** the reverb decays over 2 s

#### Scenario: Refused decay
- **WHEN** the reverb decays over 1.49 s and a level's offset is −1.45 s
- **THEN** the reverb still decays over 1.49 s

## ADDED Requirements

### Requirement: Audible reverb
The system SHALL play the reverb: every world sound bound while the reverb is on SHALL be sent to it, interface sounds SHALL not, and the reverb's output SHALL be mixed with the dry sounds.

#### Scenario: A room
- **WHEN** the player walks in `exanima01`
- **THEN** the footsteps' reverb is heard

### Requirement: Reverb send
A sound's send to the reverb SHALL be its gain clamped to its minimum gain and to 1, lowered past 4 m from the listener by 0.001 raised to the distance past 4 m (in centimetres, up to 26 m) over the decay time times 343.3, with the high band lowered over the decay times the HF ratio.

#### Scenario: A sound 6 m away
- **WHEN** a sound of gain 1 plays 6 m from the listener with the 1.49 s decay
- **THEN** about 7 % of it reaches the reverb

### Requirement: Outdoor arenas are dry
Loading an outdoor arena SHALL turn the reverb off; the indoor arenas and the story levels SHALL keep it on.

#### Scenario: An outdoor arena
- **WHEN** `arenao3` is played
- **THEN** no sound is sent to the reverb

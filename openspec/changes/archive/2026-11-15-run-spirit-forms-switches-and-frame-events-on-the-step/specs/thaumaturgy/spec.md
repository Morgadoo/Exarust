## ADDED Requirements

### Requirement: Spirit Form's switch runs in the step
Spirit Form's adoption SHALL keep only the held body, the spirit screen and the rooms' anchor on the frame. It SHALL post the switch to the start of the next step: the caster's behaviour held, the spirit's player behaviour, the caster's power caster, its mind list and status bits, and the player marker on the spirit's root. The frame SHALL NOT write those on its copies.

#### Scenario: An adoption in free mode
- **WHEN** the spirit is adopted while the simulation runs free and several steps reach the frame in one frame
- **THEN** the frame shows the player behaviour and the power caster on the spirit only, and the caster's focus never goes back

### Requirement: Spirit Form's way home is posted to the step
At the transition's black the frame SHALL forget the held body and the spirit screen and move the rooms' anchor back. It SHALL post the way home to the start of the next step: the power caster, the mind list, the status word, the player behaviour and the player marker back on the body. The spirit's removal SHALL be sent from there to the frame.

#### Scenario: Home in free mode
- **WHEN** the way home runs while the simulation runs free and the step changes the caster and the player behaviour every step
- **THEN** the body ends with its player behaviour, its caster and its one power, the spirit is gone, and no aim or focus goes back on the frame

## MODIFIED Requirements

### Requirement: Spirit list moves on the power walk
Behind the simulation bridge, Spirit Form's switch SHALL move the caster's power list to the spirit on the simulation's world, and the way home SHALL move it back there; neither SHALL write a mind's list on the frame. In one world the move SHALL run at the start of the next fixed step.

#### Scenario: A slow frame at the adoption
- **WHEN** the frame's copy of the caster's list still lacks the Spirit Form power the cast wrote
- **THEN** the spirit still takes the power from the simulation's list, and the later cast ends it

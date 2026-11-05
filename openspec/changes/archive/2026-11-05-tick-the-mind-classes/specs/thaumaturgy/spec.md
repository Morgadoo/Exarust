# Spec Delta

## ADDED Requirements

### Requirement: Timed Mind classes expire
The power walk SHALL clear Confusion's bit `0x4_0000` and Enrage's bit `0x8_0000` and end their entries once more than 15 000 ms have passed since their cast, and SHALL end Trust's and Suspicion's markers once more than 20 000 ms have passed.

#### Scenario: Confusion runs out
- **WHEN** an opponent is confused and 15 012 ms pass
- **THEN** its confused bit is clear and the entry has left its list

### Requirement: Sense Minds and Mask Mind ticks
The instant Sense Minds SHALL answer 4 once its end passes and leave the list 2000 ms later; the instant Mask Mind SHALL keep the masked bit `0x1_0000` for 15 000 ms and clear it as it leaves; the persistent forms SHALL run until ended and give their focus back.

#### Scenario: A mask wears off
- **WHEN** the player casts Mask Mind and 15 s pass
- **THEN** the player's masked bit is clear

### Requirement: Loyalty and Command Dead unlink
A Loyalty or Command Dead caster's end SHALL end the target's marker whose caster it is; a marker's end SHALL end its caster's power aimed at it and re-arm the body's AI, and Loyalty's marker SHALL play `ThmMnd_E01` at 0.15. The casts SHALL build the caster's power running.

#### Scenario: Loyalty cast again
- **WHEN** the player casts Loyalty on the body already bound to it
- **THEN** the marker ends, the caster's Loyalty ends on the next walk and the body's AI re-arms

### Requirement: Mind pick sight and ward
The mind pick SHALL skip a body wearing a mind ward and, without Sense Minds, a body the level's light-reach query does not join to the player's place; the dead powers SHALL pick with collapsed bodies allowed.

#### Scenario: Behind a shut door
- **WHEN** the opponent stands in another room behind a shut door and the player runs no Sense Minds
- **THEN** the pick finds no body

### Requirement: Spirit Form screen and sound
A Spirit Form cast SHALL play `ThmMnd_B02` at 0.2 and fade `Msc_SprtRealm` in over the level's track; while the spirit screen charges or holds its eased value SHALL be the post-processing's desaturation; the form's end SHALL play `ThmMnd_E02` at 0.1 and fade the overlay out, and the way home SHALL put the desaturation back to 0.

#### Scenario: Into the spirit realm and back
- **WHEN** the player casts Spirit Form and ends it
- **THEN** the scene greys over 1.2 s, the realm music plays, and after the way home the colour is back

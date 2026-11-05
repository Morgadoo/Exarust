# Spec Delta

## ADDED Requirements

### Requirement: Mind casts from the bar
The bar SHALL run a Mind word through the Mind front end and its cast on the caster's body at the body the mind pick finds under the pointer, and a Use on a body the player was handed SHALL cast nothing and end the control.

#### Scenario: Control Mind on an opponent
- **WHEN** the player casts Control Mind with the pointer on an arena opponent
- **THEN** the cast answers 1 and the binding is on the player's list aimed at the opponent

### Requirement: Control Mind handover
Once its binding has run 2000 ms Control Mind SHALL give the player the target's body: the caster SHALL keep the handover bit `0x10_0000` and its behaviour held still, the target SHALL gain the control bits `0x60_0000` and the possession marker, its AI SHALL be set aside and it SHALL take a player behaviour, and the camera SHALL follow it.

#### Scenario: Playing as the opponent
- **WHEN** the binding passes 2000 ms and the player then walks
- **THEN** the opponent's body walks and the caster's body stands where it was

### Requirement: Control Mind hand-back
The end of Control Mind SHALL open the transition screen and, at black, give the player its own body back: the target's AI SHALL return, its status SHALL become the caster's less the control bits, and the caster SHALL lose the handover bit.

#### Scenario: Back home
- **WHEN** the player uses the bar while in the opponent's body
- **THEN** after the fade the player is in its own body and the opponent has its AI again

### Requirement: Spirit Form body
A Spirit Form cast SHALL build a body of race byte 14 60 cm along the caster's facing, give it the player, a behaviour whose spawn snap is already counted, the caster's powers and list and the status `0x22_0000`, and leave the caster's body standing with the handover bit; the end SHALL open the transition screen and at black give the player its body, powers and list back and remove the spirit.

#### Scenario: Spirit and back
- **WHEN** the player casts Spirit Form and uses the bar again
- **THEN** the camera follows the spirit while the body stays, and after the fade the player is in its body and the spirit is gone

### Requirement: Mind break on a hit
A blow on a body that is not down, whose AI is not set aside, and whose status has the handover bit SHALL end the Spirit Form it is the body of, or else the first Control Mind it runs.

#### Scenario: The held body is struck
- **WHEN** the possessed opponent strikes the player's held body
- **THEN** Control Mind ends and the hand-back runs

### Requirement: Mind powers in the perception
Each body's Mind list SHALL be the effects its perception applies to attitude and trust, and a hold the perception breaks SHALL end its power.

#### Scenario: A broken binding
- **WHEN** the perception marks a loyalty marker broken
- **THEN** the marker is told to end after the brains' tick

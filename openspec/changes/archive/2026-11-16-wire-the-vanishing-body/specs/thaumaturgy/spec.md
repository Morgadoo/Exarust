## ADDED Requirements

### Requirement: Vanished body taken out
As a Vanish or a Long Door body vanishes, its state flags and its status word SHALL take the vanished bit 0x800, the body and the items its hands hold SHALL be disabled, and the body SHALL drop its first breakable grab that still holds.

#### Scenario: The fade-out starts
- **WHEN** a player's Vanish reaches its fade-out while the player holds an item
- **THEN** the state flags and the status word carry 0x800 and the body and the item are disabled

### Requirement: Vanished body brought back
At the come-back the bodies SHALL be enabled, the character's body SHALL lose its kinematic bit and settle for 1000 ticks before it is placed, and a Long Door whose body's condition has 0x800 SHALL place it where it stands. At the fade-in's end the vanished bit and the settling SHALL clear.

#### Scenario: The come-back and the show
- **WHEN** the vanish is let go and fades back in
- **THEN** the bodies are enabled and settling during the fade-in, and after it the bit is clear and the settling is 0

#### Scenario: A collapsing Long Door
- **WHEN** a Long Door with a destination 5 m away comes back while its body is collapsing
- **THEN** the body stays where it stands

### Requirement: Vanished target and footprint
A duellist SHALL drop a target whose state flags carry the vanished bit as it drops a dead one, and the player's footprint SHALL be neither stamped nor lifted while the player is vanished.

#### Scenario: A target vanishes
- **WHEN** a duellist's target in combat takes the vanished bit
- **THEN** with no focus the duellist leaves combat and forgets the target

#### Scenario: The player vanishes and moves
- **WHEN** a standing player vanishes and moves 60 cm, then shows
- **THEN** the footprint stays where it vanished, then follows the body

### Requirement: Vanish tendril images
A player's Vanish or Long Door over a shell of its own SHALL put the body's wisps up at the cast, an AI's door at its found place; the fade-out's end SHALL freeze and fade them, the come-back SHALL put a new set up and the fade-in's end SHALL fade it. A body image SHALL be 12 times the body's size.

#### Scenario: The images of one vanish
- **WHEN** a player's Vanish runs from the cast to its fade-in's end
- **THEN** one image is up from the cast and ended at the fade-out's end, and a second from the come-back is faded at the fade-in's end

## ADDED Requirements

### Requirement: Pain sounds
After the damage call the system SHALL derive the victim's pain sounds from the wound and the yellow (both before the bars' 0.75): above a wound of 0.3 the wound sound at 0.8 of the voice's volume and a groan; otherwise, when the wound passes the epsilon or the yellow 0.2, the wound sound past a wound of 0.1 and a groan over a yellow of 0.5, else a grunt.

#### Scenario: A deep wound groans
- **WHEN** a blow leaves a wound of 0.5
- **THEN** the victim's wound sound plays and its voice groans

#### Scenario: A bruise grunts
- **WHEN** a blow leaves no wound and a yellow of 0.3
- **THEN** no wound sound plays and the victim grunts

### Requirement: Mute victims
A victim whose voice level is under 0.01 SHALL make no pain line, and one without a wound sound SHALL play none.

#### Scenario: A mute victim
- **WHEN** a victim with voice level 0.009 takes a deep wound
- **THEN** its wound sound plays and it makes no voice line

### Requirement: Player jolt
A struck player SHALL feel 7 × the yellow plus 7 × the wound; a striker of scale above 1.3 SHALL make it twice its speed plus four times that, times 0.4 for an overhead swing. The camera SHALL jolt by its square root when it passes 1 and the pad rumble SHALL rise by it. A striking player's pad rumble SHALL rise by half the yellow it dealt.

#### Scenario: A hard blow on the player
- **WHEN** the player takes a yellow of 0.1 and a wound of 0.05 from a human
- **THEN** the camera jolts by the root of 1.05

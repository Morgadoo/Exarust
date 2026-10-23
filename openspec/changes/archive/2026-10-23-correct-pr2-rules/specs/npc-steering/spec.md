## MODIFIED Requirements

### Requirement: Presence in the scene
An AI character's presence SHALL be checked each body tick unless its condition carries bit 0x800 or its state flags carry 0x10. After a change it SHALL hold 80 ticks. Out of the activation range a present character SHALL leave the scene, and in range a dormant one SHALL enter it. A body out of the scene SHALL not be solved, ticked or drawn.

#### Scenario: A far character leaves and comes back reset
- **WHEN** a casting duellist stands 300 m from the centre of a 200 m area, and after it has gone the centre moves onto it
- **THEN** it leaves on the first tick and is out of the scene and hidden on the next; it stays dormant 80 ticks, then enters with its condition normal and an entry message, and is present and in the scene a tick later

## ADDED Requirements

### Requirement: Entering resets the body
A character entering the scene SHALL have its condition set normal, take its stance, and have its cast hook asked event −1 and cleared. Its body SHALL stop: every velocity zero; balance, tone and energy 1; the stepping episode, blow sag and posture wait cleared; the locomotion's moving, step, flags, free walk, goal and look cleared; the locomotor's speed, run and drives zeroed, its facing kept.

#### Scenario: A tired walker enters
- **WHEN** a dormant duellist with tone 0.2, energy 0.3, balance 0.4, moving, stepping, flags 0x4, a free walk, speed 3 and run 0.5 enters
- **THEN** tone, energy and balance are 1, it is not moving and has no step, its flags are 0, the free walk is off, and its speed, run, drive and going are 0

### Requirement: A hidden character
While an AI character is not present, its AI SHALL still run its combat state, choose its stance and start attacks, but its body SHALL neither turn nor move under it, and it SHALL advise, start and release no power.

#### Scenario: A hidden duellist
- **WHEN** a duellist out of the scene faces its target within reach
- **THEN** its combat state still decides, its body stays where it was, and no power starts

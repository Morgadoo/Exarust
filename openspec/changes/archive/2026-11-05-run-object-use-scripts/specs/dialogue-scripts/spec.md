# Spec Delta

## MODIFIED Requirements

### Requirement: StartDialogue
`StartDialogue(topic)`, whichever script calls it, SHALL start the named node between the pair of the roles unit's last encounter (its observer and the partner it met), not the calling script's actors; it SHALL be refused while that observer's arousal is above 10 or it fills an actor slot of any active conversation, whatever that conversation's state, and SHALL start nothing before the first encounter.

#### Scenario: Already talking
- **WHEN** a script asks to start a dialogue after an encounter whose observer is in an ended but still active conversation
- **THEN** no dialogue starts

#### Scenario: A used object's script
- **WHEN** a guard met a traveller and then a script whose only actor is the player asks for `StartDialogue(Hello)`
- **THEN** the guard's `Hello` node starts toward the traveller; before any encounter the same call starts nothing

### Requirement: Scripted device use
A scripted device's use (usable class 2, kind 3) that its 2.4 s clock allows SHALL first run the device's use script with the player as its one actor slot (none without a player), no running role and the device as the operative, then pulse the placed object with a 1200 ms fade and no hold and play the sound its usable section names (at most 15 characters) at the object at volume 0.3.

#### Scenario: The pedestal hand's use
- **WHEN** `exanima67`'s PedestalHand is clicked
- **THEN** its script `WitnessRole(Operative, $E1D1);` runs, it pulses for 2.4 s and its sound `DeviceA01` plays at 0.3

## ADDED Requirements

### Requirement: Operative witness grant
`WitnessRole(Operative, role)` in a used object's script SHALL add the role (with the parents it inherits) to every character within 18 m of the object's own place (squared distance at most 3 240 000 cm²) that the level's sight test sees from there. A `WitnessRole` in a script without an operative (a dialogue's or a role's) SHALL grant nothing.

#### Scenario: Measured from the device
- **WHEN** a device at the origin grants role 31 with characters 10 m and 20 m away and the player 500 m away
- **THEN** only the character 10 m away holds role 31

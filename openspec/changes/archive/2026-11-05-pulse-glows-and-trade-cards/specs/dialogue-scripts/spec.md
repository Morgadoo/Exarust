# Spec Delta

## ADDED Requirements

### Requirement: Scripted device use
A scripted device's use (usable class 2, kind 3) that its 2.4 s clock allows SHALL pulse the placed object with a 1200 ms fade and no hold and play the sound its usable section names (at most 15 characters) at the object at volume 0.3.

#### Scenario: The pedestal hand's use
- **WHEN** `exanima67`'s PedestalHand is clicked
- **THEN** it pulses for 2.4 s and its sound `DeviceA01` plays at 0.3

### Requirement: Live glow pulse
Each frame a pulse SHALL scale the emissive of every part of the object's own mesh, of the first part of each child mesh and the intensity of each child voxel light (its record intensity times the level), and SHALL leave them at level 0 when it ends. A level load SHALL cancel every pulse.

#### Scenario: The pedestal hand
- **WHEN** `exanima67`'s PedestalHand pulses
- **THEN** its voxel light rises from 0 to 40 over 1.2 s, falls back to 0 over the next 1.2 s, and its glow mesh is dark afterwards

### Requirement: Live crystal collect
At the end of a level load the player's two hand items, worn items and carried items SHALL be walked once: each socketed item whose first socket carries the charge flag is drained once and kept as a source, and a carried power crystal files the inventory source.

#### Scenario: A charged sword in hand
- **WHEN** a level loads with a sword whose first socket holds a charged crystal in the player's hand
- **THEN** its charge goes down by 2 and the sword is a drain source

### Requirement: Live crystal drain
Once every game second each source SHALL be drained and a source that changed nothing SHALL be dropped. A socket drain SHALL take 2 off the first socket's charge, to no less than 0, while the flag is set; the inventory drain SHALL take 2 off every carried crystal's charge above 2 and clear one at or below 2.

#### Scenario: A carried crystal
- **WHEN** a carried crystal of charge 10 is drained, and then one of charge 2
- **THEN** the first goes to 8 and counts as a change, and the second goes to 0 and does not

## ADDED Requirements

### Requirement: Ghoul chest pins
A ghoul's or hulking ghoul's body SHALL take its thorax pins scaled about the thorax's origin by half its character size, their weights taken again over the part's rest; every other race SHALL keep the stored pins.

#### Scenario: Half size
- **WHEN** the chest pins are scaled by 0.5
- **THEN** the chest point lies half as far from the thorax's origin

### Requirement: Status-effect broadcast
A body's active power list SHALL be passed event 2 once when the body starts to collapse or dies, and the player's list event 8 at a level change, every entry taking the event; each entry's answer SHALL be handled as in the tick's walk (bit 2 removes it, bit 4 marks the player's slot). The broadcast SHALL run before the next tick's own walk.

#### Scenario: Collapse with a running power
- **WHEN** a caster with Force Weapon running collapses
- **THEN** the power is passed event 2 and ends

#### Scenario: Level change
- **WHEN** the player changes level with a power running
- **THEN** the power is passed event 8 and keeps running if its control ends only on 2

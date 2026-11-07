## ADDED Requirements

### Requirement: Loaded player placement
Loading a story slot SHALL keep its save file, and SHALL place the loaded player's root and every part of its body at the saved place, in the running simulation as well as in the drawn world.

#### Scenario: A reloaded save
- **WHEN** a story save made at (−4.46, 0, 69.52) m in exanima06 is continued
- **THEN** the player stands there, does not fall, and the save file is still in the folder

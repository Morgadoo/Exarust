## ADDED Requirements

### Requirement: The debug weapon's blood in the list
The debug weapon blood (`--weapon-blood`) SHALL register the debug body's new weapon in the weapons' blood list and give its spot the blood at the start of the next fixed step, in the world that holds the list, and the weapon's drawn blood SHALL be its spot's amount.

#### Scenario: A sword drawn with blood 0.5
- **WHEN** the arena starts with `--weapon ub05ssworda --weapon-blood 0.5`
- **THEN** the player's sword holds 0.5 in a spot of the list, not in no spot

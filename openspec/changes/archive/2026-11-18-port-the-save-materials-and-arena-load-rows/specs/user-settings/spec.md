## ADDED Requirements

### Requirement: Steam Deck first-run defaults
The game's first run on a Steam Deck SHALL write CursorSize 150, GUISize 1 and GILowPrecision 1 into the user's settings file before it reads the settings, and a first run off a Deck SHALL write none of them.

#### Scenario: A first run on a Deck
- **WHEN** the game starts for the first time on a Steam Deck
- **THEN** the settings file holds the three defaults and the game reads them

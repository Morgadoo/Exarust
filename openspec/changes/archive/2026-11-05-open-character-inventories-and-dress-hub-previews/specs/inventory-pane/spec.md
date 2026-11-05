# Spec Delta

## ADDED Requirements

### Requirement: Another character's inventory
The inventory window SHALL open for a given character under the player's show rule (the player ready and not dead; an open window for that character focused, or closed when it is active and the call toggles). A new window SHALL be registered under the character's tag, its pack SHALL hold that character's carried list, its character view SHALL show that character, and its drops and drags SHALL move that character's items.

#### Scenario: A script's call on an active window
- **WHEN** a script asks for the open, active window of a companion
- **THEN** it stays open; the interaction's call closes it

### Requirement: Another character's inventory place
Another character's inventory window SHALL open at the screen width less the window width less four GUI units, at the constructor's y; only the player's own window SHALL remember its place when it closes.

#### Scenario: A companion's window
- **WHEN** the window of a character other than the player opens on a 1600-wide screen at unit 4 while the player's remembered place is (30, 40)
- **THEN** it opens at (1340, 8), registered under that character's tag; closing it leaves the remembered place at (30, 40)

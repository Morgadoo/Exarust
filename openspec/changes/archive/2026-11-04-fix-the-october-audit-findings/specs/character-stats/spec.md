# Spec Delta

## ADDED Requirements

### Requirement: Fallen player
The player that falls to its death SHALL leave the scene but stay the session's player, dead with no health, so the session's hook opens the end menu.

#### Scenario: The body stays
- **WHEN** the player dies on the 334th tick after it was lost
- **THEN** its body is out of the scene and hidden with its root, its condition is dead and its health 0

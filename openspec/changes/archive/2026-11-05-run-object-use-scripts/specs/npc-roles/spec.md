# Spec Delta

## MODIFIED Requirements

### Requirement: Roles script functions
The roles unit SHALL register `StartDialogue` and then `ChangeRole`; its encounter SHALL leave the observer, the partner, the running role and (for an entry matched on one of the partner's roles, kept otherwise) that partner role as the context `StartDialogue` starts its conversation from; `ChangeRole` SHALL report that the function does not exist.

#### Scenario: Registration
- **WHEN** the registration is listed and `ChangeRole` is called
- **THEN** the names are `StartDialogue`, `ChangeRole` and the error reads `Function "ChangeRole" does not exist.`

## ADDED Requirements

### Requirement: Character interaction
The player's Ctrl/double-click interaction on a living character other than itself SHALL do nothing while the character is collapsing (condition bit `0x800`) or the player possesses another body; otherwise a passing interaction condition, or else a Loyalty (`0x10010A`) or Command Dead (`0x10010D`) marker the player cast on the character, SHALL open that character's inventory window.

#### Scenario: A loyal character
- **WHEN** a character whose condition toward the player fails carries the player's Loyalty marker
- **THEN** its inventory window opens; with another caster's marker nothing opens

### Requirement: Interaction condition on use
The interaction's condition SHALL be the character's interaction condition toward the player: none fails, an empty one passes, else its text runs with the character as `Actor[0]` and the player as `Actor[1]` and no running role, and passes on a result above 0.

#### Scenario: The golem boss
- **WHEN** a character's condition toward the player is `result = Actor[1].HasItem($1DE);` and the player lacks the item
- **THEN** the condition fails

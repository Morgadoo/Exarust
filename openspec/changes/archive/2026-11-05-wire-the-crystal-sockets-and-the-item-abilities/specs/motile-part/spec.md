# Spec Delta

## REMOVED Requirements

### Requirement: Grip mode store
**Reason**: The two-hander's turn of mode 2 into 1 was read as a half-sword thrust grip. It tests the item's ability word 3 (the burning blade) and its holder's lit-blade byte.
**Migration**: See Grip mode setter.

## ADDED Requirements

### Requirement: Grip mode setter
The grip mode setter SHALL store the requested mode before running the grip's callback, and SHALL mark the links as relaxing only when the grip has a callback. The two-hander's callback SHALL turn a requested mode 2 into mode 1 when its item's weapon section names ability 3 (the burning blade) and its holder's lit-blade byte is set, so the lit blade rests in both hands; that turn SHALL be local to the callback.

#### Scenario: Lit burning two-hander
- **WHEN** mode 2 is asked for a two-handed grip whose item is a burning blade held by a body with the lit-blade byte
- **THEN** the links take mode 1's arrangement and the stored mode is 2

#### Scenario: A duellist's lit greatsword
- **WHEN** `exarust grid "play arena --ability-item 3,npc-only --opponent --still-opponent …"` runs
- **THEN** the duellist's "Sword GSF 01" lights and its grip logs the turn to mode 1

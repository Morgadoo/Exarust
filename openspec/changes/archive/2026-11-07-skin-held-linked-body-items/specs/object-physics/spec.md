# Spec Delta

## ADDED Requirements

### Requirement: Held skinned items
A held item whose model has a skinned linked-body object SHALL draw that object's meshes skinned over the item's own node entities, which its grip and bodies move, and SHALL leave the grip and the bodies unchanged.

#### Scenario: A held flail
- **WHEN** the player holds `flail a01` and swings it
- **THEN** its five meshes are skinned over its five nodes and no Rebalance is logged

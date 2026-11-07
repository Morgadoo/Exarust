# Spec Delta

## ADDED Requirements

### Requirement: Skinned linked-body objects
A model root mesh whose render word has `0x400000`, that owns its data and is not marked skinned, SHALL be drawn as one skin over its mesh nodes, the root as bone 1 and each node's marked mesh children (class `0x89`, `0x400000` without `0x800000`) taking the next bones in child order before the walk enters them. Each bone's inverse bind SHALL be its node's file frame turned back; the bodies SHALL be unchanged.

#### Scenario: The shipped objects
- **WHEN** every Objlib and Objects model is loaded
- **THEN** 22 objects with 105 member meshes are skinned, each member's stored weights naming only the walk's bones and its own among them

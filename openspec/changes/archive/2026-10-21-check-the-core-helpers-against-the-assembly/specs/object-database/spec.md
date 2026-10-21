# Spec Delta

## ADDED Requirements

### Requirement: Object manager scene trees
A record with a model name SHALL get the factory weapon its `@` names or the model; without one, the container volume of its volume section (radius the length of its extents, x and z squared first) or, with an operative section, a 50 cm placeholder of radius 86.60254; otherwise no tree.

#### Scenario: A volume
- **WHEN** a record without a model has a volume section of 30, 40 and 120
- **THEN** its tree is a volume of radius 130

#### Scenario: A placed operative without a model
- **WHEN** a level places an operative record with no model name
- **THEN** the running game spawns its placeholder node at the placement

### Requirement: Object manager flags and materials
Every tree the manager makes SHALL carry node flag 0x4000 and each child 0x5000 with the parent's hidden bit. A weapon record SHALL hide its `*` materials; an item-body record SHALL hide `*` and paint `&` materials, and a painted tree SHALL not be kept as its model's master. A release that finds another tree in the slot SHALL report the mismatch and still hand the slot on.

#### Scenario: Materials
- **WHEN** an item-body record's model has materials `*mask` and `&cloth` and a plain record's has `*glass`
- **THEN** the mask is hidden, the cloth painted and the glass drawn

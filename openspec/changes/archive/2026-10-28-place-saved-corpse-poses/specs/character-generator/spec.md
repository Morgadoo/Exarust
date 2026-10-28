# Spec Delta

## MODIFIED Requirements

### Requirement: Merge order and poses
The system SHALL merge a garment's chosen nodes along the slot tree and restore a saved bone tree by matching each record to its parent bone's child whose rest local offset is nearest the record's origin in the parent's restored frame, only under 20 cm (the first nearest kept), skipping a record's subtree when none matches. A matched child SHALL take the record as its world frame and re-derive only its local rows.

#### Scenario: Merge order
- **WHEN** every slot has a chosen node
- **THEN** they merge in the order 1 to 13, 17, 14 to 16, 18

#### Scenario: Twenty centimetres
- **WHEN** a child record lies 19.99 cm, then 20.01 cm, from its parent's child rest offset
- **THEN** the first is placed on that child and the second is skipped with its subtree

#### Scenario: Siblings near one child
- **WHEN** two sibling records lie 15 cm either side of one child's rest offset
- **THEN** both match it, and the child keeps the second record's frame and its rest offset

## ADDED Requirements

### Requirement: Flat saved poses
A flat saved pose SHALL map slots 1 to 18 by the saved part table (pelvis, belly, chest, right arm, head, left arm, legs) and ignore later slots; each part SHALL take its slot's world frame and re-derive only its local rows.

#### Scenario: Flat slots
- **WHEN** the 13 shipped `exanima05_sw` flat poses are mapped by the slot table
- **THEN** each body's pelvis–belly, shoulder and thigh distances are rigid

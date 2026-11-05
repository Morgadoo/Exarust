# Spec Delta

## ADDED Requirements

### Requirement: Loadout form drags
A left press over an entry of the loadout form's kit list or arsenal box SHALL start its drag, its icon following the pointer. Released over the preview, an arsenal entry SHALL be taken as the equip rules say; released over the arsenal box, a kit-list entry SHALL be returned; released over its own view, the entry SHALL only move there; released anywhere else, it SHALL stay where it was.

#### Scenario: Equip by drag
- **WHEN** a rank 0 weapon is dragged from the arsenal box onto the preview at tab 0
- **THEN** the entry is lent and the kit list holds the weapon

#### Scenario: A drag let go elsewhere
- **WHEN** an arsenal entry is released over the form's records
- **THEN** nothing is taken and the entry keeps its place and flags

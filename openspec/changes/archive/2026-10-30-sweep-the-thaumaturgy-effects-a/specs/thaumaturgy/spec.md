# Spec Delta

## ADDED Requirements

### Requirement: Spirit spawn
The spirit body's spawn SHALL take the caster's frame, replace its place's height with the caster's ground height, step 60 cm along the frame's facing row in single precision, keep the caster's basis, flag the new node `0x1000`, and give the body the status bits `0x220000` and behaviour mode 4.

#### Scenario: On the ground ahead
- **WHEN** a caster at (10, 20, 30) with its ground at 5 faces +z
- **THEN** the spirit spawns at (10, 5, 90)

### Requirement: Deferred power calls
A posted power call SHALL run on the unit's next pass through its domain's update (Force, Energy or Displacement by the id's domain byte); a call whose domain has no update, or whose update does not answer 1, SHALL clear the caster's ending flag.

#### Scenario: An unknown domain
- **WHEN** a call for a Mind power id is run
- **THEN** no update is called and the ending flag is cleared

### Requirement: Throw sub-nodes
The throw's lift SHALL collect the thrown item and every child node depth first, give them all the thrown shell and keep the item's own, which every collected node takes back when the throw ends.

#### Scenario: Depth-first order
- **WHEN** an item 1 has children 2 and 3 and node 2 has child 4
- **THEN** the collected order is 1, 2, 4, 3

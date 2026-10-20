## RENAMED Requirements

- FROM: `### Requirement: Light, body and track records`
- TO: `### Requirement: Light and body records`

## MODIFIED Requirements

### Requirement: Export walk and numbering
The exporter SHALL skip owned nodes (flag 0x1000) and nodes with neither flag 0x40000 nor class 1 with their subtrees, walk through a top-level group, and sort the others into characters, props, placed objects and scene nodes. A character's subtree SHALL NOT be walked. Scene nodes SHALL be numbered from 1 in walk order, and props and placed objects from the last scene node's number.

#### Scenario: A prop after two nodes
- **WHEN** a scene has nodes A and B, then a prop holding node N
- **THEN** N's stored parent is 3, the number the prop takes after the last scene node's 2, which N also holds

#### Scenario: A node under a character
- **WHEN** a scene has a character holding a plain node, then a plain node holding another
- **THEN** only the last two are scene nodes, numbered 1 and 2, and the second stores parent 1

### Requirement: Light and body records
A light body SHALL be 40 bytes with 1.0 as its fifth word, plus two spot values when flags2 & 0x300 is 0x200 and a zero word with three values under body flags 0x30000. A rigid body's links SHALL stop at the first link marked 0x400000, with the count patched, and weighted links SHALL store the weights over the rest points and the parent point less the centre.

#### Scenario: Weighted link
- **WHEN** weights 0.5, 0.25, 0.125, 0.125 over rest points (0,0,0), (10,0,0), (0,20,0), (0,0,40) and a parent point (3,4,5) with centre (1,1,1) are written, followed by an unsaved link and a third
- **THEN** the link count is 1, the body point is (2.5, 2.5, 5) and the parent point (2, 3, 4)

#### Scenario: Light with spot and extra values
- **WHEN** a light with body flags 0x10010 on a node with flags2 0x200 is written
- **THEN** its body is 64 bytes, the fifth word is 1.0 and the word at byte 48 is 0

## ADDED Requirements

### Requirement: Keyframe track records
A keyframe track SHALL store 0x3AE2, mode, duration, the last key's index and the keys; a track without keys SHALL store the index 0xFFFFFFFF and no keys.

#### Scenario: A track without keys
- **WHEN** a track with mode 2, duration 3 and no keys is written
- **THEN** the chunk is 16 bytes of payload ending in 0xFFFFFFFF

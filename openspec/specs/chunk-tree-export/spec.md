# chunk-tree-export Specification

## Purpose
Writes a scene back into a chunk tree as the original does when a save stores a visited level: the chunk framing, the walk that sorts and numbers the nodes, and every chunk it writes.

## Requirements

### Requirement: Chunk framing
The writer SHALL open a chunk with its tag and a zero size word, close it by storing the number of bytes written after the size word, and SHALL be able to take an open chunk back out whole. Closing with no chunk open SHALL be an error.

#### Scenario: Nested chunks
- **WHEN** chunk 0x10 holds the word 5 and chunk 0x20 holding the half-word 1
- **THEN** the outer size is 14, the inner size is 2 and the output is 22 bytes
- **WHEN** a chunk is opened, written into and taken back out
- **THEN** the output is as before it was opened

### Requirement: Export walk and numbering
The exporter SHALL skip owned nodes (flag 0x1000) and nodes with neither flag 0x40000 nor class 1 with their subtrees, walk through a top-level group, and sort the others into characters, props, placed objects and scene nodes. Scene nodes SHALL be numbered from 1 in walk order, and props and placed objects from the last scene node's number.

#### Scenario: A prop after two nodes
- **WHEN** a scene has nodes A and B, then a prop holding node N
- **THEN** N's stored parent is 3, the number the prop takes after the last scene node's 2, which N also holds

### Requirement: Stored parent and data source
A node's parent SHALL be stored only when exported, with its local matrix; otherwise the parent is 0 and the world matrix is stored. A node SHALL store only a reference to the exported node sharing its data with the lowest number below its own.

#### Scenario: A group child
- **WHEN** a plain node sits under a top-level group
- **THEN** it is stored with parent 0 and its world translation

#### Scenario: Shared data
- **WHEN** node Y shares node X's data and both are exported
- **THEN** Y stores X's number 1 and nothing after its header

### Requirement: Exported chunk order
The exporter SHALL write the sanity half 0xAFCF and version 0x3D23, then the tile grid, the material list, the scene nodes, the props, the local database, the characters, the environment and the caller's chunks, each only when it has content. The environment SHALL be written only for the whole scene and when its first word lacks 0x100.

#### Scenario: Everything present
- **WHEN** a scene with a grid, a mesh with a material, a prop, a placed object, a dead character and an environment is exported with one extra chunk
- **THEN** the root tags are 3DED, BA00, 3D000000, 3DE10000, DBC0, CAC00001, EC00DC00 and the extra chunk, and Exarust's reader reads them back without errors

#### Scenario: Empty scene
- **WHEN** an empty scene is exported
- **THEN** the output is the four bytes CF AF 23 3D

### Requirement: Mesh encoding
A mesh body SHALL store its render word without the low byte, the number of saved streams less one (format bit 0x1000000 leaves a stream out), each stream's elements by format byte, the topology with parts named `_Default` when they have no material, 16-bit indices below 65 535 vertices and 32-bit from there, and the per-triangle sub-chunks unless switched off.

#### Scenario: A quad
- **WHEN** a quad with three streams, one unsaved, and render word 0x200FF is written
- **THEN** the stored stream count is 1, the render word is 0x20000, and the reader gets four positions, the UVs, two groups (`Wood`, `_Default`) and the triangle flags
- **WHEN** one triangle is written for 65 534 and for 65 535 vertices
- **THEN** the second is 6 bytes longer

### Requirement: Material list and texture renames
The material list SHALL hold each mesh part's material name once, lowercased, unless a shared library holds it; its count SHALL include names the scene's library lacks, which are not written. Renaming SHALL turn a kind-0x10 layer's texture into `<stem>_rfi` with kind 0x40 and record the lowercase `<stem>.rfi` once; the `.rfi` switch SHALL change only the extension.

#### Scenario: Shared and missing materials
- **WHEN** meshes use Wood, STONE, WOOD and Iron, stone is shared and only Wood is in the library
- **THEN** the list stores count 2 and one `wood` record

#### Scenario: Renaming a texture
- **WHEN** a layer of kind 0x10 names `Brick01.dds` and renaming is on
- **THEN** it names `Brick01_rfi` with kind 0x40 and one rename `brick01.rfi` is recorded, not repeated by a second export

### Requirement: Light, body and track records
A light body SHALL be 40 bytes with 1.0 as its fifth word, plus two spot values under flags2 0x200 and a zero word with three values under body flags 0x30000. A rigid body's links SHALL stop at the first link marked 0x400000, with the count patched, and weighted links SHALL store the weights over the rest points and the parent point less the centre. A keyframe track SHALL store 0x3AE2, mode, duration, the last key's index and the keys.

#### Scenario: Weighted link
- **WHEN** weights 0.5, 0.25, 0.125, 0.125 over rest points (0,0,0), (10,0,0), (0,20,0), (0,0,40) and a parent point (3,4,5) with centre (1,1,1) are written, followed by an unsaved link and a third
- **THEN** the link count is 1, the body point is (2.5, 2.5, 5) and the parent point (2, 3, 4)

#### Scenario: Light with spot and extra values
- **WHEN** a light with body flags 0x10010 on a node with flags2 0x200 is written
- **THEN** its body is 64 bytes, the fifth word is 1.0 and the word at byte 48 is 0

### Requirement: Props and placed objects
A prop and a placed object SHALL store their flags under 0xB80000, name or ids, local matrix, parent number and tail word, then part records for owned children (body placement, cloth particles); a child whose attachment is neither SHALL end its siblings' records.

#### Scenario: A prop with three parts
- **WHEN** a prop with flags 0x80234 besides its prop and saved bits owns a body part, an emitter part and a cloth part, in that order
- **THEN** it stores flags 0x80000 and only the body's part record

### Requirement: Characters
A character SHALL store its record id, (body x, floor y, body z), heading and condition, with the pose tree when dead or posed. In a save it SHALL store the AI home (character AI, record kind 0x1000, skip byte clear) and, in AI mode 1, the pose and the motion state.

#### Scenario: A dead character
- **WHEN** a dead character at body (500, 90, −200) with floor height 12 is exported
- **THEN** the reader places it at (5, 0.12, −2) m with a two-bone pose

#### Scenario: A saved character
- **WHEN** a living character with a character AI in mode 1 and record kind 0x1000 is saved
- **THEN** its sub-chunks are CACE0F77, CAC0EB00 and CAC0EA00

#### Scenario: The save game's placement
- **WHEN** a saved character's attached objects write nothing
- **THEN** no CAC0EC00 chunk is left, and when two write, it holds their count 2 and their bytes

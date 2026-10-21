# Spec Delta

## ADDED Requirements

### Requirement: Placement transform records
The system SHALL apply a placed prop's or object's transform records before it spawns and registers its bodies: each replaces its node's local transform and applies its nested records to that node's children. A record SHALL find its node from the node it applies to (the object's root at the top level): that node for an empty name, else its first direct child whose name matches exactly; a record whose node is missing SHALL be skipped with its nested records.

#### Scenario: First story level
- **WHEN** `exanima01.rfc`'s props are read against their objects
- **THEN** every record finds its node and some transform records carry nested ones

### Requirement: Placement position records
The system SHALL apply a placement's position record to a mesh node when its count is the node's position-stream count, rebuilding the object's model with the new elements and normals recomputed from them.

#### Scenario: Stored banner drapes
- **WHEN** `exanima01.rfc`'s position records are read against their objects
- **THEN** all 14 match their meshes' position-stream counts

### Requirement: Node kinds and instances
The system SHALL read camera nodes (the node they follow and one word) and sector nodes (no body), keep the header's instance word, and draw an instance node with its owner's meshes, lights and emitters at its own transform; an instance node stores no body or sub-chunks. The mesh body reader SHALL replace a mesh's deprecated fading bit (bit 31) by bit 24; other node kinds keep the word.

#### Scenario: Instanced beams
- **WHEN** a level whose nodes instance other nodes is shown
- **THEN** each instance draws its owner's mesh where the instance stands

### Requirement: Level strings and database-only read
The system SHALL read a level's own string table with its local database, report a table that does not decode while the rest of the database loads, and read a level's local records and strings on their own without building its scene.

#### Scenario: Database only
- **WHEN** only a level's database is read
- **THEN** its object records and string table are returned and its placed objects are not read

### Requirement: Character locale and material order
The system SHALL create a world-database character only in the level whose locale its record stores (a level's own records always), and SHALL shade a surface with a library material over a file's own record of the same name.

#### Scenario: Locale filter
- **WHEN** a level places a world-database character whose stored locale is another level's
- **THEN** that character is not created there

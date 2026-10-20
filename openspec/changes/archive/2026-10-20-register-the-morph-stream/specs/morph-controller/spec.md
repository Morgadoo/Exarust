## ADDED Requirements

### Requirement: Morph stream registration
Stream registration SHALL run only when the controller has weights and the owner carries bit 0x200000. It SHALL reuse the stream whose low flag bits are 0x1000, or grow the owner's list by one with flags 0x04000001 and type 0x02a41000, and SHALL zero the stream's V·32 bytes.

#### Scenario: Guard, reuse and creation
- **WHEN** there are no weights or the owner bit is clear
- **THEN** nothing is registered
- **WHEN** an existing stream is found
- **THEN** it is reused and V·32 bytes are zeroed
- **WHEN** none is found
- **THEN** the list grows with flags 0x04000001, type 0x02a41000 and the vertex count

### Requirement: Morph target list
Target collection SHALL add the remapped corners of every triangle whose flags carry 0xC00000, in the order 1, 2, 0, without duplicates.

#### Scenario: Flagged and repeated triangles
- **WHEN** the first and third triangles carry the flag, are identical, and the second does not carry it
- **THEN** the list holds the first triangle's remapped corners once, in the order 1, 2, 0

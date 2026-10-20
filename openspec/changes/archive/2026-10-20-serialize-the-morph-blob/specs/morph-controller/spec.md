## ADDED Requirements

### Requirement: Morph blob bytes
Serializing the morph data SHALL write little-endian the magic, the size, the base, the V rest points, the part count, then each part's pivot and V deltas. Parsing SHALL accept only the magic 0xEDC0C589 and a size matching both the bytes and the vertex count.

#### Scenario: Round trip and rejects
- **WHEN** one part with two rest points is serialized
- **THEN** the bytes are 84 long, start with 89 C5 C0 ED and 84, and parse back to the same data
- **WHEN** the magic is changed or the vertex count is wrong
- **THEN** parsing returns nothing

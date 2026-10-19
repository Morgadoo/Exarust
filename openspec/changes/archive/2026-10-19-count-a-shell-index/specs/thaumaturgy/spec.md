# Spec Delta

## ADDED Requirements

### Requirement: Shell index room
A skinned shell index room SHALL add one for each triangle of a section with bit 2, and nothing without that bit. A hidden section with bit 2 SHALL still count. The room SHALL ask for 12 bytes plus 8, plus 4 bytes a triangle. The kind SHALL be 3 and the base 0. The end SHALL be one less than the triangle count, and the vertex end one less than the kept count. A range one early SHALL add nothing. A range two early SHALL subtract one.

#### Scenario: Three triangles and a short range
- **WHEN** a skinned range of two triangles, a plain range, a hidden skinned triangle, and a range that ends one early are counted for two kept vertices, and a range that ends two early is counted for none
- **THEN** the room has 3 triangles, 32 bytes, width 4, one channel, kind 3, base 0, end 2 and vertex end 1, and the short range has -1 triangles, 16 bytes and vertex end -1

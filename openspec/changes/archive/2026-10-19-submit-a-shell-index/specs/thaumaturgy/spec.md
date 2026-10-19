# Spec Delta

## ADDED Requirements

### Requirement: Shell index submission
A play draw of a skinned shell SHALL count its room and, when the upload flag is set, SHALL submit that upload. The mesh SHALL hold one index per triangle of the payload. A name above 0 SHALL unmap. A clear flag SHALL keep the counted room and SHALL not unmap.

#### Scenario: Three triangles, uploaded and kept
- **WHEN** two skinned spans are submitted with the flag set and a name of 1, and the same spans are submitted with the flag clear and a name of 1
- **THEN** the upload has a header of 28, a payload of 12 and 3 indices, and it unmaps, and the clear draw keeps a room of 32 bytes and does not unmap

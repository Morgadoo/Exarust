## ADDED Requirements

### Requirement: Editable part assembly
Appending an editable part SHALL join its groups to the destination's group with the same material or a new group, its channels by low key, and its streams by whole format word, skipping the normal stream (low key 2); the position stream SHALL be placed by the rigid `dst⁻¹ · src` in the original's float order, a stream gap filled with 0 (255 for colours), missing face flags filled with 1 and edge bits with 7.

#### Scenario: A turned destination
- **WHEN** a part is merged into a destination 1 m along +X and turned a quarter about Y
- **THEN** the part's (1, 0, 0) lands at (0, 0, 0), its (1, 0, −1) at (1, 0, 0), and no normal stream is merged

### Requirement: Editable material regroup
After an assembly the faces SHALL be sorted into material runs, stable within a group, with every channel, the face flags, the edge bits and the extra per-face word permuted alike.

#### Scenario: Interleaved groups
- **WHEN** faces 0, 1, 4, 5 belong to "a" and 2, 3 to "b"
- **THEN** face 4 becomes the third face and face 2 the fifth

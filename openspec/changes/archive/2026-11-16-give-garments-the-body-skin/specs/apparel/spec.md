## ADDED Requirements

### Requirement: Skin group stack
The skin build SHALL keep the parts' group centres on a stack of sixteen frames of room for eight centres each: a push past sixteen frames SHALL be refused with "Skinning group stack overflow." while its pop still counts down, a garment root SHALL start on an empty frame that tests no limit, and centres past the eighth SHALL run on into the next frame, where a child's push overwrites them.

#### Scenario: Ten centres
- **WHEN** a part pushes ten centres and its first child pushes one
- **THEN** its second child reads the first eight centres, then the first child's count and centre in place of the ninth

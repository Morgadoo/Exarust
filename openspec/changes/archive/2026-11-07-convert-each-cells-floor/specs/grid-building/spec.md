# Spec Delta

## ADDED Requirements

### Requirement: Converted cell floor
The system SHALL build a cell's `+Floor` from each entry's stored mesh, turned by its side with no translation, `+` materials taking the surface: appended, regrouped, given planar UVs, welded and converted with the vertex-cache reorder. The run holding triangle 0 SHALL be marked for the Ground Query. The centre variant's children SHALL be their own nodes at the cell centre.

#### Scenario: Triangle 0 of a merged floor
- **WHEN** exanima06 is built
- **THEN** each of its 1133 floored cells has exactly one merged run marked as holding triangle 0

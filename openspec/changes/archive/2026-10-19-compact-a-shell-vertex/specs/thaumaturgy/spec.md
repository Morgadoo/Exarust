# Spec Delta

## ADDED Requirements

### Requirement: Compacted shell vertices
A shell mark list SHALL start at 255. A skinned section SHALL write its index over each triangle from the first through the last. Any other section SHALL write 255. A vertex SHALL take the next compacted id when a link in its half-open range names a mark other than 255, and stop at the first such mark. A vertex with none SHALL be absent. An index SHALL become that id. A short list or an absent id SHALL be an error.

#### Scenario: Two kept vertices and a gap
- **WHEN** marks are built for a skinned range, a plain range, and a later skinned range, three vertices are numbered, the kept indices are applied, an absent index is applied, and a short mark list is built
- **THEN** the marks are 0, 255, 2, the vertices are 0, absent, and 1, the indices become 1, 0, 1, the absent index is an error, and the short mark list is an error

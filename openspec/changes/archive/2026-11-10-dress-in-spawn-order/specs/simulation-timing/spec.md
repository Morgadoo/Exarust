## ADDED Requirements

### Requirement: Lockstep runs dress their characters alike
Under `--fixed-frames` the characters SHALL be dressed in the frame their parts are attached, every one of them, in spawn order (entity index, ascending), with their garments' fits and paints built in that frame, so the garments, the held items and those items' bodies take the same entity ids in every run. Every bridged resource type SHALL be registered in the frame's world when it is bridged, so the frame's resource hold covers it.

#### Scenario: The armed duel
- **WHEN** four identical `play arena --opponent --weapon ub05ssworda --frames 1500 --fixed-frames --trace-all` scenes run, two to a grid
- **THEN** their traces are byte-identical, the `body` column included, and every run reads the first clash alike

#### Scenario: A resource only the fixed step names
- **WHEN** a resource is bridged and the frame's world holds its resources
- **THEN** its first insert in the frame's world takes no new entity id

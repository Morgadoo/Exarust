## ADDED Requirements

### Requirement: Run-time body clone
A body's clone slot SHALL copy no link carrying the code flag or the breakable flag, which leaves every origin pivot behind, SHALL remap the indices of the links it keeps (a leg's limit link following its link, each part's link range keeping its surviving links), and SHALL share the template's collision-sphere array instead of copying it.

#### Scenario: A clone of a two-part body
- **WHEN** a two-part body with its stored link, its origin pivot, an added breakable link and an added plain link (the leg's limit) is cloned by the slot
- **THEN** the clone keeps the stored and the plain link, the leg's limit points at the second, the part's link range spans both, and both bodies hold one sphere array

# Spec Delta

## ADDED Requirements

### Requirement: Dropped weapon owns itself
A collapsing body's dying tick SHALL give a dropped weapon itself as owner and copy that owner onto every child. The weapon leaves its parent's list. The weapon and each descendant take the weapon as their root. The iteration count is 2 plus the parts visited, minus 1. The link count is 0 and the awake byte is 0. A grip at or above 0 becomes -1. A part that has a child, an object type whose low twelve bits are 0x89, and object bit 0x400000, sets its mark to -2.

#### Scenario: A held weapon returns to itself
- **WHEN** a collapsing body holds weapon 3, parent list 5 then 3 then 8, grip 4, child 7 of object type 0x1089 with flags 0x400001 and grandchild 8, and weapon 4 with grip -3, mark 4, parent list 9 and no child, and the dying tick runs
- **THEN** weapon 3, child 7 and grandchild 8 have owner 3 and root 3, iterations are 4, the child's iterations stay 11, links are 0, grip is -1, the awake byte is 0, the parent list is 5 then 8, child 7's mark is -2, weapon 3's mark stays, and weapon 4 keeps grip -3, mark 4, parent list 9 and iterations 2

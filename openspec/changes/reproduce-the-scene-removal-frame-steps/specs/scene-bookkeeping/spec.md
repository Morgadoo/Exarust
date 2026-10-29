## ADDED Requirements

### Requirement: Removal drain
Each frame SHALL first move every queued removal, oldest first, to the end of a pending list and hang its node from the global world. At the frame end a finished entry SHALL be replaced by the last pending one, which is looked at next.

#### Scenario: Swap order
- **WHEN** deletions A, B, C and D are pending and B's subtree is owned
- **THEN** A, D and C are taken out in that order and B stays pending

### Requirement: Removal kinds
At the frame end a hidden node SHALL get its owner back. A deletion SHALL wait while its subtree is marked owned, otherwise get its owner back, be reported when that owner is the global world, and be taken out with its subtree, or deferred to the free list while another owner holds it. A free entry SHALL skip the wait.

#### Scenario: A hidden node
- **WHEN** a node in a room is queued hidden
- **THEN** it hangs from the global world during the frame and belongs to the room again after it

#### Scenario: A held free entry
- **WHEN** an owned node and a held node are queued as free entries
- **THEN** the owned node is taken out at once and the held node goes to the free list without getting its owner back

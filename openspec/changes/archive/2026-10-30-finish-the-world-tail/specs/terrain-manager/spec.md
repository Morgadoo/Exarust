## ADDED Requirements

### Requirement: Block descriptor
The terrain manager model SHALL give a block descriptor a vertical half extent of half the block's height span plus one centimetre with no other minimum, and SHALL walk a block tree as a node's descriptor followed by its four children's in order.

#### Scenario: A flat block
- **WHEN** every bounds cell of a block is at 5.0
- **THEN** the descriptor's vertical centre is 5.0 and its vertical half extent is 1.0

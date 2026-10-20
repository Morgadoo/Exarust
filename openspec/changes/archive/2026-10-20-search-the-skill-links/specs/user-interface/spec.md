## ADDED Requirements

### Requirement: Skill link first pass
The search SHALL test the two slots beside a positive hit node, wrapping into 0 through 4. A reachable slot SHALL place its satellite at 18.3 units and 0.2 of a turn, scale the y offset by 4 when the direction's y magnitude is below its x magnitude, normalize the offset and keep the largest dot above 0.1. A kept slot SHALL be written and returned.

#### Scenario: The adjacent slot
- **WHEN** the hit is `0x0202`, the node is at 10, 10, the point 0, 0, the direction 1, 0 and the unit 1
- **THEN** the returned code is `0x0201` and the point 28, 10

### Requirement: Skill link scan
Without a kept slot the search SHALL scan every node's five slots, requiring a squared offset of at least 1 and a dot of at least 0.3, and keep the smallest `length / dot²` from a one-million start. The winner SHALL be written and returned.

#### Scenario: The five-slot scan
- **WHEN** the hit is 0, one node is at 0, 0, the point 0, 0, the direction 0, 1 and the unit 1
- **THEN** the returned code has low slot 5 and the point is 6, 17
- **WHEN** no code is reachable
- **THEN** code 0 and the input point are returned

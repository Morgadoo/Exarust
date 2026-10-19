## ADDED Requirements

### Requirement: Marked window dismiss
A match begin SHALL walk two child lists from the last child to the first, before the specialist leave. A child whose flags masked by 0xce100000 equal that mask SHALL be dismissed. Any other child SHALL stay. A count of 0 or less SHALL dismiss nobody, and a child past the count SHALL stay. A command that does not begin SHALL dismiss nobody. The method on a dismissed child, and the call after the two lists, are not part of this step.

#### Scenario: Marked children go before the physician leaves
- **WHEN** in-game command 4 in arena walks a first list of an unmarked child, a child flagged 0xce100000 and a later marked child with count 2, then a second list of a fully flagged child and a child flagged 0xce000000, and the same lists are offered to story mode, and a marked child is offered with count 0
- **THEN** the dismissed names are the 0xce100000 child and then the fully flagged child, the later marked child and the 0xce000000 child stay, the physician is removed, story dismisses nobody, and count 0 dismisses nobody

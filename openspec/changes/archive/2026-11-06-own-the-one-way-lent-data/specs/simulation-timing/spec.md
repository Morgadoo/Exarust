## ADDED Requirements

### Requirement: One-way resources across the bridge
A resource that only the frame writes SHALL cross to the simulation only, and one that only the simulation writes SHALL cross to the frame only (and in when the frame inserts it); a write on the other side SHALL not cross, alike inline and on the simulation's thread.

#### Scenario: A rogue write
- **WHEN** the simulation writes a resource the frame owns, or the frame writes one the simulation owns
- **THEN** the other side never sees that write, and without such writes both sides see what one world sees

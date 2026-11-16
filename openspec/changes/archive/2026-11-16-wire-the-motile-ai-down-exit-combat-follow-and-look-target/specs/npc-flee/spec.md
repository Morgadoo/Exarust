## ADDED Requirements

### Requirement: The flight's escape and run
A flight that ends because its threat is elsewhere SHALL soft-stop a body its walk moves (the route and its mode word end, the run bit clears, the stopping bit is set); while it runs, the body's cursor and wanted directions SHALL take its heading.

#### Scenario: Escaped while running
- **WHEN** a fleeing brain on its locomotion runs with its threat in its own room, then with its threat in another room, then in its home room
- **THEN** it runs on with its cursor and wanted direction on its heading, then it soft-stops at level 0, then it flees on

## ADDED Requirements

### Requirement: Wisp strip end
A wisp strand's strip SHALL add one point past its last drawn point only when the strand goes on past its visible end; at the end itself the last drawn side SHALL be the tip's, three quarters toward its own raw side.

#### Scenario: A strip that ends at its end
- **WHEN** a strand's strip ends at its visible end
- **THEN** its last side is the tip rule over the last two raw sides

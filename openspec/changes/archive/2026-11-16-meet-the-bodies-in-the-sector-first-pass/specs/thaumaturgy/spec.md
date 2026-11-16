## ADDED Requirements

### Requirement: Blast and wave meet the bodies first
A thrown blast or wave SHALL strike the other bodies' parts only in the next sector step's first pass, before the character ticks; its own tick SHALL keep its level and cloth contacts and strike no body. The caster's own parts SHALL be skipped.

#### Scenario: A blast at a body ahead
- **WHEN** a duellist's blast is thrown at a body 0.35 m ahead
- **THEN** every tick the body moves, it has moved by the end of the first pass and nothing later in the tick moves it

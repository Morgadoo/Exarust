## ADDED Requirements

### Requirement: The chase walk's arrival
A chasing fighter's walk SHALL stop following, with the soft stop, once its ground point is within 100 cm (3-D) of its route's last point; it SHALL also stop following when it has no route or the fighter leaves combat, so the next combat tick runs on past the chase.

#### Scenario: Arrival
- **WHEN** a chasing fighter's ground point comes 99 cm from its route's last point
- **THEN** the walk soft-stops, its route is cleared and the next combat tick runs on past the chase

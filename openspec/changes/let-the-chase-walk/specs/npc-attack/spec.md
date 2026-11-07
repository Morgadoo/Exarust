## MODIFIED Requirements

### Requirement: The chase route
A fighter stuck past 21 ticks SHALL run the chase route: with no route followed and no fresh foe it SHALL drop its foe, focus and level and leave combat; with a fresh foe, or following a route whose end lies more than 100 cm (3-D) from the estimate, it SHALL plan a route to the estimate on the foe's ground through the navigation grid. The combat tick SHALL end at the chase only while a route is followed. The walk SHALL stop following, with the soft stop, once its ground point is within 100 cm (3-D) of the route's last point; it SHALL also stop following when it has no route or the fighter leaves combat.

#### Scenario: Fresh foe
- **WHEN** a fighter on a flat grid chases a fresh foe 5.1 m away, then chases again
- **THEN** it follows the route and its tick runs on the first time, and ends at the chase the second time without a new plan

#### Scenario: Arrival
- **WHEN** a chasing fighter's ground point comes 99 cm from its route's last point
- **THEN** the walk soft-stops, its route is cleared and the next combat tick runs on past the chase

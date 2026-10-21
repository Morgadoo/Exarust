# Spec Delta

## ADDED Requirements

### Requirement: The plan turn
Only one brain SHALL plan a route a frame: another brain asking SHALL record its wait (frames since its last plan) when it is longer than the longest so far, and the longest waiter SHALL plan next. A plan SHALL be followed when the route has at least two points and is forced or its ends lie more than 150 cm apart across the ground.

#### Scenario: Waits and spans
- **WHEN** two brains ask on frames 1 and 2 and a third asks with an equal wait; and a route 5.1 m long and one 30 cm long are planned
- **THEN** the second brain plans next, the long route is followed and the short one only when forced

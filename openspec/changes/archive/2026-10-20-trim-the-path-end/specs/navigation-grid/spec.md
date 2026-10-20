## ADDED Requirements

### Requirement: Path endpoint fix-up
The endpoint fix-up SHALL drop up to eight points from the path's end whose penalty byte is above 64/(quality+1), always keep the first point, and report how many remain.

#### Scenario: High penalties and the eight cap
- **WHEN** the last three of five points carry penalty 100 at quality 0
- **THEN** two points remain
- **WHEN** eleven points carry high penalties in a row
- **THEN** only eight are dropped and four remain
- **WHEN** the quality is 1
- **THEN** a penalty of 40 is dropped

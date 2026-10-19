# Spec Delta

## ADDED Requirements

### Requirement: Season round trip
A save SHALL restore the day, which weeks were won, the seasons and tournaments won, the reward count, and the reward item ids. A career saved on day 8 with the first two weeks won SHALL still grant season 0 when the remaining weeks are swept.

#### Scenario: Two weeks, then the season
- **WHEN** a company has swept two weeks, is saved on day 8, loaded, and then sweeps through day 22, and that result is saved and loaded again
- **THEN** the first load is on day 8 with the first two weeks won, two tournaments won, and no reward ids, and the second load is on day 22 with one season won, four rewards, and the ids 1175, 1174, 1177 and 1176

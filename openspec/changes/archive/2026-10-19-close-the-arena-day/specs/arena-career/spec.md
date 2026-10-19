# Spec Delta

## ADDED Requirements

### Requirement: Day and season
Ending a day SHALL add 1, starting from 0, and subtract each employee's wage. A merchant's wage SHALL be 2, a physician's 12, a trainer's the sum of six skill levels, and a combatant's half that sum, at most 5, plus 1. A day whose number mod 7 is 1 SHALL give the week to a company with at least 3 of the 6 events and no tie. The company's own id is 1. A day that is a multiple of 7 SHALL drop an entry whose health is under 3276. Winning all four weeks SHALL grant four season rewards.

#### Scenario: Wages, a dropped entry, and a swept season
- **WHEN** a company with one unskilled combatant, entered and at health 3275, ends days through a sweep of every week, and a second company ends day 1 on a 3–3 tie
- **THEN** the first night costs 1 copper and takes week 0, day 7 drops the entry, day 22 grants four rewards, and the tie takes no week

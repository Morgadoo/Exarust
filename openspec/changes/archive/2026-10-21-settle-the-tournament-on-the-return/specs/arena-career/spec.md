## ADDED Requirements

### Requirement: Tournament settled on the return
Back at the hub after a match on a day that is a multiple of 7, each event before the first one the company entered that has no winner and no entrant SHALL take the company `2 + high(roll × 5)`, the roll stepped by `× 0x08088405 + 1`, after the day's match count.

#### Scenario: Roll 1 on the return
- **WHEN** the events are open, won by 5, open, entered, open, open, and the roll is 1
- **THEN** the winners are 2, 5, 6 and the rest stay open, and the roll ends at 0xdc6dac1f; six open events with roll 1 take 2, 6, 3, 3, 5, 3 where the start's pass gives 2, 7, 3, 3, 6, 3

## MODIFIED Requirements

### Requirement: Hub arrival
Arriving at the hub SHALL, when a physician is on the staff, remove the weapon-kind object lying within 10 cm in x and z of (−2100, 0, 900) cm, as the physician's hire does, and SHALL build the tournament board when it holds no prize.

#### Scenario: A physician and an empty board
- **WHEN** the staff holds a physician and the board's prize is 0, then a merchant and a prize of 7
- **THEN** the object is removed and the board built, then neither

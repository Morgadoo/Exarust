# Spec Delta

## ADDED Requirements

### Requirement: Shell size fade
A shell SHALL store a size of 800. Opening SHALL divide the elapsed time by that size times 1.5. Above 1 it SHALL become 1 and leave, or enter the hold state when held. The hold state SHALL move on and stamp the size times the remainder, rounded to the nearest even integer. Closing SHALL store 1 minus the elapsed time divided by the size. A value not above 0 SHALL become 0 and finish. A held close SHALL store 3 times that value, then show 1. Any other state SHALL stay.

#### Scenario: Open, hold, and close
- **WHEN** the stored size opens for 0, for 1200 and for 1201, a held open of 1201 is stepped, a hold at 0 and a half are stamped, a close of 400 and of 800 are stepped, a held close of 400 is stepped, and a resting fade is stepped
- **THEN** the size is 800, the opens stay at 0, stay at 1, and finish at 1, the held open enters the hold state, the stamps are 200 and 10, the closes are one half and finished at 0, the held close shows 1 with a gain of 1.5, and the resting fade stays

## ADDED Requirements

### Requirement: Free loop zero wait
When the step driver asks the free simulation loop to wait zero milliseconds, the loop SHALL look at its inbox and yield its time slice instead of sleeping.

#### Scenario: A step due within the millisecond
- **WHEN** the driver answers a wait of 0 ms
- **THEN** the loop's wait is zero and the thread yields

### Requirement: Free loop timed wait
Any other wait SHALL be the driver's milliseconds in clock time, divided by the clock's speed and capped at the driver's longest wait.

#### Scenario: A slowed clock
- **WHEN** the driver asks for 10 ms at half speed
- **THEN** the loop waits 20 ms of real time

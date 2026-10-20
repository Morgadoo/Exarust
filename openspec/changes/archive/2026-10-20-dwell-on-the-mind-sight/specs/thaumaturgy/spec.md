## ADDED Requirements

### Requirement: Sight tick throttle
A sight tick SHALL be due at least four milliseconds after the last one.

#### Scenario: Three and four milliseconds
- **WHEN** three milliseconds have passed
- **THEN** the tick is not due
- **WHEN** four milliseconds have passed
- **THEN** the tick is due

### Requirement: Sight dwell
The dwell window SHALL be 3000 ms when the node's dwell value is above 1, 5000 ms when its body is marked, and none otherwise. The fade SHALL be the elapsed time over the window, and the node SHALL be lost once the clock passes seen + window.

#### Scenario: Window and fade
- **WHEN** the dwell value is 2
- **THEN** the window is 3000 ms
- **WHEN** 1500 ms of a 3000 ms window have passed
- **THEN** the fade is 0.5 and the node is kept
- **WHEN** the clock passes seen + window
- **THEN** the node is lost

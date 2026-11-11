## MODIFIED Requirements

### Requirement: Periodic timers
The system SHALL run the timer table once a frame on the app's real clock and report each record that is not free and is due, in record order, re-arming it `interval × 100` from the frame time. A new timer SHALL take the first freed record, and a reschedule SHALL arm a timer `(interval − elapsed) × 100` from the frame time.

#### Scenario: Re-armed from now
- **WHEN** a 20-unit timer added at 1000 ms runs at 3000 ms
- **THEN** it runs next at 5000 ms

## ADDED Requirements

### Requirement: GUI timer
The timer table SHALL hold the GUI timer as its first record, one unit (100 ms) for the app's life. The GUI screens SHALL show a requested tooltip, hide an expired one, step held scroll arrows and delete closed forms only on the frames the GUI timer comes due.

#### Scenario: A delayed tooltip
- **WHEN** a hinted record stays under the pointer past the 400 ms delay
- **THEN** its tooltip shows on the next frame the GUI timer comes due

### Requirement: Arrow pause
A press that starts a scroll arrow's repeat SHALL reschedule the GUI timer by −6 units, so it next comes due 700 ms after the press.

#### Scenario: Holding an arrow
- **WHEN** an arrow is pressed at 130 ms
- **THEN** the GUI timer next comes due at 830 ms

### Requirement: Auto rematch record
With the arena's auto switch on, the return to the hub SHALL register a 40-unit timer record, which SHALL be freed when it comes due and the last match started again.

#### Scenario: Four seconds after the return
- **WHEN** the record is registered at 1000 ms
- **THEN** it comes due at 5000 ms and not again

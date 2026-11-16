## ADDED Requirements

### Requirement: The staging freeze holds the states
While the staging freeze is set, every AI tick SHALL run its presence, contacts and target pass (which may enter combat and set the target) and then, in place of the current state's call, the idle stand: a present body SHALL take the out-of-combat turn toward its look with the idle sway, and a body following a route SHALL soft-stop. No combat state, flight, idle routine or idle walk SHALL run.

#### Scenario: A followed route while frozen
- **WHEN** a brain following a route runs one frozen body tick, and another the same tick unfrozen
- **THEN** the first's route has stopped with the request's stopping bit set and its turn's heading is its look; the second still follows

#### Scenario: A duellist while frozen
- **WHEN** an engaged duellist 5 m from its foe is ticked under a fresh freeze
- **THEN** it takes its guard on the first tick, its combat mind does not change and it does not step for the freeze's ticks, and it steps in later than a duellist never held

### Requirement: The staging freeze's span
The staging freeze SHALL hold for the transition screen's 500 ms hold and its 600 ms fade back in, counted as 92 body ticks from the first tick after it is set, and SHALL clear on the 93rd; setting it again SHALL start the count again. Frames that run no body tick SHALL not count.

#### Scenario: The release
- **WHEN** the freeze is set and the brains' chain runs 92 body ticks, one frame without a tick, and one more tick
- **THEN** it holds through the 92 ticks and the frame, and the 93rd tick runs the states

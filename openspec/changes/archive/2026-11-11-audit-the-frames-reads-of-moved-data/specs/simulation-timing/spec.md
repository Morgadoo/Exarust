## ADDED Requirements

### Requirement: Frame readers of data the frame does not hold
Once the bridge has started, every frame schedule SHALL be checked as it is built, and again when its systems change, for systems whose access names a type the frame's world does not hold behind the bridge: a moved resource, a type only the simulation's systems touch, or a lent resource while the simulation runs free. The start-up schedules, which run before the bridge starts, SHALL be skipped.

#### Scenario: The types listed
- **WHEN** play mode starts with the bridge
- **THEN** the log names the types the frame's world does not hold, the navigation grid, the bodies' physics and the weapons' blood list among them

### Requirement: Such a reader is reported
A frame system found reading such a type SHALL be logged as an error, and a build with debug assertions SHALL stop, unless the reader was allowed with a reason; an allowed reader SHALL be logged with its reason.

#### Scenario: A frame reader of a moved resource
- **WHEN** a test app's frame system reads a moved resource through an optional resource parameter
- **THEN** the audit names that system and that type, the fixed step's own reader is not named, and a debug build stops unless the reader was allowed

### Requirement: No frame reader of moved data in play
The game's frame schedules SHALL hold no reader of a type the frame's world does not hold behind the bridge: work on such data is posted to the next fixed step, and data the frame reports is a snapshot of the step's.

#### Scenario: Debug options on
- **WHEN** play mode runs exanima01, exanima04, exanima08 and the arena with the profiler, the traces, the digest and the debug views on
- **THEN** the audit reports no frame reader

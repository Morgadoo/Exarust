## ADDED Requirements

### Requirement: Brains' time in the profiler
The profiler SHALL report each frame's time in the brains' tick, its runs and its parts as the difference between the brains' clock's totals and the last frame's; the clock is the fixed step's and the frame reads it as a snapshot, so the report also holds behind a free-running simulation.

#### Scenario: Brains in the log
- **WHEN** exanima01 runs with `--perf-log 5`
- **THEN** the `brains` line reports the time and the runs of the brains' ticks of the last frames, not zero

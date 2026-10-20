# Spec Delta

## ADDED Requirements

### Requirement: Frame profiler
The system SHALL, with `--profiler` or `--perf-log`, measure each frame's main-thread time per main schedule (with the number of fixed ticks), the render thread's schedule time and the GPU time of Bevy's passes and Exarust's own passes, and SHALL show it in an overlay toggled by F10 or log it at the given interval.

#### Scenario: Where the frame goes
- **WHEN** play mode runs with `--profiler`
- **THEN** the overlay shows the frame time percentiles, the time of each main schedule, the render thread's time and the GPU passes, with a graph of the last 120 frames

#### Scenario: Off by default
- **WHEN** neither flag is given
- **THEN** no render diagnostics are recorded and no probes run

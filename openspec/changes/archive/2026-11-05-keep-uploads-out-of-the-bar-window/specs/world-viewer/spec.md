# Spec Delta

## ADDED Requirements

### Requirement: Upload readouts in the profiler log
The profiler's logged report SHALL count the image, mesh and material assets added or changed since the previous report, with the bytes of the images and the image changed most often, and SHALL time a CPU write of 8 MiB into a freshly mapped upload buffer, keeping the fastest of four passes.

#### Scenario: Uploads in the log
- **WHEN** play mode runs with `--perf-log 5` after the load
- **THEN** every report has an `assets:` line with the added and changed images, meshes and materials since the last one, and a `staging write:` line in MB/s

### Requirement: Silent grid host
The grid host SHALL open no audio output (its games play the sound), and SHALL leave rodio's stream errors out of its log.

#### Scenario: A stalled host frame
- **WHEN** the grid host's frame stalls while its games load
- **THEN** its log gains no `alsa::poll()` lines

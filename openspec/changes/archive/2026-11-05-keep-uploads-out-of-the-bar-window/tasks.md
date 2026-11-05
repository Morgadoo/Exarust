# Tasks

- [x] Reproduce the stall in a grid and profile it (`perf record` on the
  stalled child: 72 % of its user time in `memmove` on the compute pool).
- [x] Count the uploads per report in the profiler (`assets:` line): the
  stalled game uploads no more than the others.
- [x] Measure the staging memory (`staging write:` line) and map the games'
  `/dev/nvidia0` mappings: the first two games hold a 64 MiB BAR block;
  the stalled one writes at 0.5 GB/s, and each stall starts with the
  kernel's BAR1 VA failure.
- [x] Patch gpu-allocator's upload memory choice; check every game's maps
  hold no BAR block and its staging writes run at 8-11 GB/s.
- [x] Drop the grid host's audio plugin and filter rodio's stream errors.
- [x] Test the profiler's churn line.
- [x] Grid checks: four exanima01 games with the fix, and side by side with
  the unpatched build in both launch orders.
- [x] Update the exarust README and docs/PERFORMANCE.md.
- [x] fmt, clippy -D warnings and tests for exarust; play build.

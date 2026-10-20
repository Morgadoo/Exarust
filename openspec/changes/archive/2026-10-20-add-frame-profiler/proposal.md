# Proposal

## Why

Play mode runs at 26–32 FPS and nothing in Exarust said where a frame's
time goes: the FPS readout gives a number, Bevy's render diagnostics were
off, and a CPU sampler is not available in the sandboxed development
container. Engines ship a frame profiler for this.

## What Changes

- `--profiler` turns on a frame profiler with an overlay (F10 toggles it);
  `--perf-log <seconds>` logs its report. The report splits the main
  thread by schedule (First, PreUpdate, the fixed ticks and their count,
  Update, PostUpdate, Last, the rest), gives the render thread's schedule
  time, Bevy's per-pass GPU timestamps and Exarust's own GPU spans
  (`exa_cube_shadows`, `exa_gi`), with frame-time percentiles and a graph
  of the last 120 frames.
- Cargo features `trace-chrome` and `trace-tracy` on `exarust` turn on
  Bevy's tracing spans, for Perfetto or Tracy; `context/tools/trace_summary.py`
  (research tooling) summarises a Chrome trace per span.
- A `profiling` build profile: the play build with line tables and symbols.
- `docs/PERFORMANCE.md`: the tools, the 2026-10-20 play-mode measurements
  and the order of work they give.

## Capabilities

### Modified Capabilities

- `world-viewer`: the frame profiler.

## Impact

- `exarust::profiler` (new), `exarust::main` (flags), `exarust` features.
- `exa_render::cube_shadows`, `exa_render::gi`: a GPU timing span each,
  recorded only when render diagnostics are on.
- `Cargo.toml` (`profiling` profile).
- Evidence: not applicable (Exarust's own tooling).
- Out of scope: the optimisations the measurements point to (each its own
  change); per-camera GPU timing, which Bevy 0.20's split command buffers
  do not allow from outside its passes.

## Phase

Phase 8, performance.

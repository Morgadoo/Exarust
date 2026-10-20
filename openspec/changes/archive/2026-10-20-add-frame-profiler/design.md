# Design

Main thread: probe schedules inserted into `MainScheduleOrder` before First
and after First, PreUpdate, RunFixedMainLoop, Update, PostUpdate and Last
record an `Instant` each; at the next frame's first probe the differences
make a `FrameRecord` (240 kept). A `FixedFirst` system counts the ticks.

Render thread: systems in `RenderSystems::ExtractCommands` and
`RenderSystems::PostCleanup` time the render schedule into an atomic shared
with the main world.

GPU: `RenderDiagnosticsPlugin` records Bevy's pass spans; Exarust's cube
shadow and GI systems open a `time_span` on their encoder when the recorder
exists. A span opened around each camera's schedule was tried and dropped:
the passes' command buffers are submitted after it, so it measured nothing.

The overlay (bottom left) refreshes its text four times a second and its
120-bar graph every frame. Everything is added only with `--profiler` or
`--perf-log`.

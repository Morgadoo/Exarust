## ADDED Requirements

### Requirement: Renderer finished before exit
In the frame whose end exits the run, the render world SHALL wait for every pipeline compile still in flight and for the GPU work submitted, before the renderer is dropped, so no compile runs inside the graphics driver while the process's exit handlers run.

#### Scenario: A run ends right after a new material variant
- **WHEN** a run's last frame asks for a pipeline that is still compiling
- **THEN** the compile finishes before the render world is dropped and the exit trace logs both stages

## Why

Roadmap task B4, lane `camrun3`: the port queue's lane Q13 (camera,
navigation and runtime) and the unclear re-audit's port items for the main
tick's clocks, the scene task's clock, the gamepad's simulation-clock
deadlines and the mode switch's per-mode words. The scene clock's arithmetic
was ported but never ran; the lights' flicker timed itself on the frame's
own delta; the pad's step repeat and Powers hold ran on the real clock; the
mode switch's level cap was a fixed 15 everywhere; a switch to no mode left
the camera running; a shot did not reach the cursor pin's saved view; and
the player's own arena entrant got no start shot.

## Evidence

REA (Exanima.exe 0.9.5): the main tick's clock update (real clock, paused
accumulator under the pause counter, game clock), the scene task's start and
tick (stamp from real less paused, the double compare against 1/60 s, the
scale and the audio mark) and the light flicker reading the scene's step;
the gamepad gameplay reader's two deadlines on the simulation clock; the
mode switch's flag words and their readers; the mode-record switch's
controller detach; the shot routines' copy of the camera node into the
cursor pin's saved view; the arena match start's two-point shot.

## What changes

- `exa_game::frame_timers::GameClock` and `SceneClock` run in `First`; the
  scene's step is `exa_world::scene_frame::SceneFrame`, which the flicker
  reads.
- `PlayPad::update_at` runs the step repeat and the Powers hold on the
  simulation snapshot's clock.
- `exa_game::flow::ModeWords` follows the flow's mode; the skill screen, the
  HUD's ready alert and the company report take its level cap.
- `ChaseCamera::detached`, the shot counter for the cursor pin, and the
  first round's shot for the player's own entrant.

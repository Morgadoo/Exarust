# Design

- The screen model is `exa_ui::transition`'s (already reviewed against the
  run routine). The flow owns it so the action, which is flow state, runs
  inside the same call that sees the callback. `transition_tick` takes a
  `TransitionInput` (real clock, the music voice's gain, the current audio
  fade, whether a level change is in progress) and returns a
  `TransitionFrame` (the action's steps, the alpha byte, the music gain to
  write, the stop, the audio fade, the world fade reset, the close).
- The screen is created on its first frame so its start is on the real
  clock, as the original stamps it with the frame clock, while the flow's
  own rules keep the 12 ms game clock (which stops during loads and pauses).
- The audio fade is seeded from the engine's value: the screen writes it
  only below 1 or at black, so a screen without the music fade never writes
  it.
- The original's action loads a level synchronously, so its hold starts
  after the load. Exarust loads in stages over frames; an action whose steps
  load or tear down marks the transition as waiting, and the screen's clock
  is held while `LevelChangeReady` or the loading screen is up. A host with
  no loader releases it after `LOAD_WAIT_FRAMES` (30).
- The long hold comes from a flow flag the level exit's fade bit sets,
  consumed by the next open, and a running screen's flag carries over to a
  screen opened over it (the original reuses one form and only its hold
  clears the byte).
- `run_transition` sits in the flow's chain before `apply_steps`, so the
  action's steps apply in the frame it runs; the overlay is a UI node at
  z 960 drawn with `linear_opacity` as the death fade is.

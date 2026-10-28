# Design

- `exa_game::footwork`: `StepRecord` (started, landed, start clock, long,
  queue count, queued vector, target, and Exarust's give-up clock),
  `StepGate`, `dispatch` (returns `Nothing`, `Queue(word)`, `Start(word)`
  or `Relay(word)`), `goal_direction`.
- `steering::MoveWord` gains the raw word, the queued word, the last laid
  word and `take_queue`; its expiry flag now clears the queue.
- `player::combat_steps` runs: take the queue, tick the word, dispatch,
  lay the goal (`lay_goal`: last word, long flag, `goal_stride`,
  `Steering::step_target` with the grid snap), then land the step
  (`land_step`, after the dispatcher, as the body's tick follows the
  behaviour's) and set the stepping velocity toward the target.
- `player_behaviour::combat_mode` returns the mode and the button word;
  `steer_player` keeps both on `PlayerBody`.

## Open questions

- The point the under-way test measures from: Exarust uses the chest
  (answer by reading the motile's step routines with REA).
- The grid snap's height: Exarust keeps the clamped height (answer with a
  capture of a step onto a slope).
- The meaning of the mode ladder's kept action (number 45): read the
  action table with `cargo exa`.

Formats page: none (behaviour only); docs/PLAYER.md, Combat mode.

# Design

- `player::Footwork` carries `crouched_step_cm`, built by
  `crouched_step_length(reach, extension)`: the double 0.7 product
  narrowed, then the extension, as the original orders it.
- `steering::key_sum(facing, word, side_share)` is the shared key sum;
  `key_goal_direction` uses the 0.8 share and drops the height,
  `key_step` the full side and drops the height of either branch.
- `steering::goal_stride(reach, extension, clock, encumbrance, tap)`.
- `player::cursor_ground` reads the player's body child and the physics
  resource optionally, so either missing only skips the lock or the raise.

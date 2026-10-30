# Design

The fixes keep the lane's structure (pure functions over plain records)
and only correct what the instructions show.

- The scissor switch is kept as a per-entry list of `Option<bool>` over
  the concatenated groups, because the original's remembered bit lives
  for the whole list call, across groups, and starts at zero; the sector
  routine turns the test off before each list so that start is right.
- The particle masks live in `passes` and `PassState::particle_mask`
  returns them, so the schedule and the pass share one constant.
- The resolve weight is computed from the integer `4·pass + 1` converted
  to single and divided into 1.0 in single, as the instruction does; its
  use inside the resolve is not interpreted.
- The pass's sub-view count follows the instruction (`min(count, 4)` for
  every pass); for the levels Exarust models it equals the leftover count.

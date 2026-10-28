# Design

The review keeps the lane's structure and corrects the rules in place.

- `steering::MoveWord` takes the keys as `[forward, back, left, right]`,
  builds the word in the key reader's order (`of_keys`), compares the
  analog length and the crouch in double precision, and keeps its clocks
  in u64 milliseconds so the 200 ms sums never wrap. The movement target
  is not modelled; `target_expired` reports the tick the original lets
  it go.
- `player_behaviour::aim_lock_bones` keeps the first bone outright and a
  later one only when strictly nearer, lerps the reference point's height
  toward that bone by its fraction (the original lerps the view-space
  points and maps back) and hands the result to the existing
  `aim_lock_height`.
- `camera_follow_yaw` and `look_snaps` use `exa_ai::combat::{gap, wrap}`,
  which compare in double precision and step by the single full turn, as
  the engine's helper does.
- The live lift translates each sphere by its offset from the ragdoll's
  grab point to the sweep's start; the start stays no nearer than the eye.
- `item_cursor::segment_meets_sphere` is the engine's segment test,
  measured from the start, the end or across; `marker_id` the hexadecimal
  reader.
- The cloth's drag plane is stored as a normal and offset at the press.

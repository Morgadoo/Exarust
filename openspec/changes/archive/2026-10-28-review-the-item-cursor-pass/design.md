# Design

- `item_cursor::TurnRamp::frame(now_ms, frame_ms, stick, keys)` takes the
  pad's raw move stick (`PlayFrame::move_raw`, zero without an active pad)
  and the bound Forward/Back/Left/Right keys; `hand::hold_hand` passes the
  virtual clock's last frame time in ms.
- `place_point` takes the hit point's height in cm; `place_frame` returns
  the frame and the box answer. The hold picks with
  `Physics::fixed_pick_except`, skipping the held prop's stand-ins, and
  keeps the last point when nothing is hit (Exarust has no navigation mesh
  behind every pixel and no sweep).
- `release(cancelled, hover)`: GUI slot → `ToGui`; plain or world hover →
  `Drop`; otherwise `PutBack`. `hand` puts the parts back at the saved
  frames for anything but `Drop`.
- `Ragdoll::grab` is the lift's hold again; `drag_pull` has no slack.
- `Frame::orthonormalized` follows the engine's order (y, then x projected
  off y, then x × y normalised).

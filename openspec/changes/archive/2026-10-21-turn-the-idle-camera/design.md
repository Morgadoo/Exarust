# Design

The watch keeps the original's three fields (the busy stamp, the last
frame's clock, the pointer) in a resource and runs as one pure step
(`IdleWatch::frame`) fed by a system that gathers the frame: the game clock
(`Time<Virtual>`, which pauses as the original's game clock does), the
pointer in physical pixels (the original's desk coordinates are window
pixels), the player's condition word, the drive's step, the hand's state
and the mouse buttons. It runs after the hand's systems and before the
chase camera, so the turn reaches this frame's camera tick.

The cursor flag (`follows_cursor`) stands for the behaviour's flag that
gates its cursor read: `steer_player` passes no cursor to the steering
while it is off, so the steering keeps the cursor's last direction.

## Open questions and how they were answered

- The game clock or real time: the frame reads the clock that pauses with
  the game (REA notes on the frame clock), so `Time<Virtual>`.
- The stepping byte the test reads is taken as the drive's running step
  [inf]; the original's field is the stepping foot's.
- What the cursor flag gates: the behaviour tick's cursor ground read, in
  both branches (REA, two readers).

No format page changes; the page extended is `docs/PLAYER.md`.

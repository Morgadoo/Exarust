# Design

The idle turn reads what the hand holds this frame, as the original's idle
test follows its drag and request in the controls' frame, so it stays after
the hand; the hand picks through the chase camera, so the turn now comes
after the camera too, and the camera follows the turned yaw on the next
frame (one frame, under 17 ms, against a turn of 0.1 rad a second).

The pick's limits widen the single-precision value and compare it with the
double constant, as the instructions do; an unordered compare (NaN) passes.

## Open questions and how they were answered

- Whether the combat request reads the right button before the reach: the
  per-frame request's instructions (REA) take the combat branch, measure the
  reach and return before the walk keys and the right button are read.
- Whether the classification's limits are single or double: CVTSS2SD then
  COMISD against 8-byte constants 0.08 and 0.9 (REA read_bytes).

No format page changes; the page extended is `docs/PLAYER.md`.

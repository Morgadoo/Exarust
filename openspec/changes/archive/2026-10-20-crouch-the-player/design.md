# Design

## Amount

`PlayerBody.crouch` (0..1). `crouch_tick(crouch, held, moving, down)` runs
once per fixed tick in the steering, before the helpless early return so a
downed body still crouches: first the body tick's decay (in f64,
`crouch − 0.02` while `crouch > 0`), then the controls (`1.0` when `held`
and (`!moving` or `down`)). The original's body tick decays before the
behaviour update sets it, so a held key keeps exactly 1.0. "Moving" is the
controller's locomotion request from the last tick (`desired_velocity`
non-zero), the counterpart of the original's locomotion-active byte;
"down" is the condition's off-feet mask (0x200), the original's own test,
or Exarust's `Fallen`.

## Consumers

- `limp::bearing` sets `Bearing.crouch` from the player; NPCs stay 0.
- `limp::clip_frames` multiplies the locomotion row's rate by
  `crouch_clock(crouch)`: `1 − 0.1·crouch` above 0.1 (f64), else 1, before
  the `0.8·speed + 1` pace, as the original scales the gait record before
  the clock uses it.
- `body_balance` multiplies the footwork's `step_cm` by
  `steps::crouch_stride(crouch)`: 0.7 above 0.5, else 1.
- `request_attacks` passes `crouch > 0` to `Attacker::pick`, which already
  forces index 0 when crouched.

## Tests

The tick holds 1.0 standing, decays while walking, holds while down, and
reaches 0 or below in 50 ticks and then stays; the clock factor is 1 at
0.1 and 0.9 at 1; the stride is 0.7 above 0.5 only; the bearing carries
the player's crouch.

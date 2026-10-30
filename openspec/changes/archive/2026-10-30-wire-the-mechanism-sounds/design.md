# Design

The steps stay where they are (`exa_physics`, after the solver's tick);
each tick's report becomes messages the mixer reads in its frame chain,
in the order the ticks sent them. The physics crate cannot reach the mixer
(the audio crate depends on it), so the request carries set names and the
mixer resolves them; the random shares are drawn on the mixer side, pitch
first, as the original draws them.

The original's creak is not a loop: the step restarts the body's voice on
a fresh variant whenever it is not playing (flags restart and allocated,
no loop bit) and only re-levels a playing one, whose point stays where it
started. A quiet tick zeroes the gain, sets the stop bit and frees the
voice. The lever's and the plate's creak and grind are the same code. The
lever's return tick is the one real loop: a file loaded by name, restarted
on a fresh voice at 0.1 and pitch 1 with the loop bit; the step fades it
by 0.01 a tick while returning and frees it below 0.01, and the report of
0 frees it as it is.

Points: the creak, grind and click play at the body's centre; the knock,
slam, unlatch, unlock and rattle at the handle point the step measured
last. A signalled open runs before the tick measures the handle, so its
`opn` plays at the previous tick's point.

The original's single-literal levels (0.1, 0.05, …) are doubles; Exarust's
`Sound` keeps them as singles, and `levels` widens each by its shortest
decimal spelling so the narrowed result matches the original's.

## Open questions

- Which random stream the one-shots use: the runtime library's own
  (traced: the draws call it); its generator and seed are not traced, so
  the engine generator stands in, as for the existing lock sound.
- Some door calls (the unlock, the rattle) pass a second sound environment
  global; whether it differs from the world's was not traced.
- The journey bubble: the original's bubble effect steps itself once a
  tick (its own step, read in REA); Exarust's journey steps its bubble and
  the plain tick excludes journey bubbles, so the report did not
  reproduce; a test pins it.

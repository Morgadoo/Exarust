# Design

## Decisions, as traced

- The show/hide helper walks the parent's records from 0 up to and
  including the stored last index, so iterating every control is the
  original's loop. Its answer is the state's low byte with bit 0x40
  cleared, read before the lit bit is recomputed (the same byte after).
- Hit-and-focus loads a constant 0 for the cursor on the no-form branch;
  the hit test's -1 is not used there.
- The press widens the 32-bit clock and stamp before the subtraction and
  compares with 450 using a signed jump, so a clock behind the stamp counts
  as inside the window.
- The viewport's release callback is called but its answer is dropped:
  the routine answers 0 and never calls the form's release. The hit
  routine stores the port's cursor code on the form's -1 whether or not
  the callback exists.
- The draw slot compares the client's left, right and bottom edges with
  the view's bounds (not the top), then sets the refresh flag
  unconditionally; `DrawInput::refresh` carries that flag, so only a
  caller passing `false` reaches the progressive branch.
- The menu cues: the audio call takes the gain in its fourth argument
  (XMM3, single 0.06), the pitch and the flags 0x4060 on the stack. The
  pitch is the run-time library's `Random` (a double) times 0.4 plus 0.8
  in double precision. Exarust plays them through
  `AudioEnvironment::play_buffer` at the origin with those flags; the
  twister's seed is a guess (the original seeds it from the clock).
- The message-box callbacks store 1 (first button) or 0 (second button,
  close mark) and send the box virtual messages whose form behaviour is
  named "close" and "destroy" as a guess.

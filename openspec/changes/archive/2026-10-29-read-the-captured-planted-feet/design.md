## Context

Formats page: `docs/formats/RSQ.md` (motion playback; captures and the
muscles-off sampler). The rules sit beside the clip clock
(`exa_anim::clock`), the muscles-off sampler (`body_limp.rs`,
`exa_game::limp::sample_limp`) and the swing (`exa_combat::swing`).

## Decisions

- Every one of the original's captures (whole, plain, and the switch
  capture of the current targets) copies each foot's touching bit into its
  bone's flag; other bones keep theirs (zero from the buffer's
  allocation). Exarust keeps the flags as a per-slot vector on the
  animator, written after each capture the clip clock takes.
- The muscles-off sampler uses the capture as its source while the
  bracket aims at the capture's key; leaving it zeroes the blend only (the
  living sampler also takes the blend off the clock and resets the ease).
- The feet: not walking, every untouched foot anchors where it is;
  walking, only with support: a foot the flags refuse lifts (anchor
  re-taken), one they ask for plants unless touching, writing three times
  the scale (cm) as its anchor height and sounding a step. Walking and
  support are the condition and the feet pass's flag; a body without
  muscles runs no feet pass in Exarust, so both are kept from the moment
  the muscles go (labelled in code).
- The held-voice fade runs after the swing routine in the body tick, so
  a swinging voice is heard 0.01 below the value the swing set; it is
  skipped only while the condition is attacking.
- The foley's volume and base are the swing rustle's and the walking
  rustle's factors respectively (the foley set-up writes both; the golem
  race gives 1 and 1).

## Open questions

- Which field is the walking rustle's pitch divisor (named `size` here);
  answered by a capture of the body record in the running game.
- Whether the anchor height of three times the scale is a world height
  or a height above the ground in the original's frame; answered by a
  capture of a muscles-off walking body (none is known to occur).

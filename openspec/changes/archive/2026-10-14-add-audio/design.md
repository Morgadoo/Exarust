# Design

## Decisions

- Entries are wrapped as Bevy `AudioSource`s from their bytes (no copy to
  disk), cached by name.
- Positional effects use Bevy's spatial audio with the listener on the
  camera; reverb waits for the environment data.
- Variety comes from a tiny xorshift generator (no extra dependency).

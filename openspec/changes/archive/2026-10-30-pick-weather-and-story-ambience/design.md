# Design

- The stand-in voice is a `Voice` field of `AudioEnvironment` addressed by the
  id `DUMMY_VOICE` (`u32::MAX`), so `voice`/`voice_mut` reach it and the scan,
  ranking and binding, which walk only the pool, never see it.
  `allocate_or_dummy` wraps `allocate`; `release_voice` frees a pool voice
  and drops the stand-in, returning the cleared holder.
- `one_shot_direction` is split out of the scheduler so the epsilon and
  unordered fallbacks are testable. The original sums `(y² + x²) + z²` with
  `y = 0`; that equals `z² + x²` exactly.
- `ambience_for_level` reuses `music::story_music`'s byte table rather than
  duplicating it.
- `arena_ambience` returns the area and the optional wind bed override;
  `apply_arena_ambience` sets the area and then writes the override into the
  bed voice when one is held, as the setup does.

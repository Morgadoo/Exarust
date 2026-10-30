# Proposal

## Why

`TAudioEnvironment` (docs/PLAN.md Phase 5, visual and audio fidelity) still
returned no voice to the music and ambience callers when the 512-voice pool
was full, ported the ambience one-shot direction and the Music setting's
volume test without the original's unordered compares, chose no ambience for
the story levels and had no arena weather rule.

## What Changes

- The music and ambience callers take a shared stand-in voice when the pool
  is full; its writes are never heard and freeing it only clears the holder.
  A voice handed a slot outside the source table is parked on source 0.
- The Music setting skips an unordered remembered volume; the ambience
  one-shot direction falls back to `x = 1` when its squared length is at or
  below the `f32` epsilon or unordered, and is scaled in centimetres before
  the metre conversion.
- A story level takes the ambience its name's bytes pick at full volume
  (`AmbUWBA` for the fourth level, whose files do not ship); every other
  story level clears it.
- `arena_ambience` holds the outdoor arena setup's weather rule: rain above
  0.01 clears, daylight below 0.5 plays `NtrAN` at 0.3, a calm arena `NtrA`,
  wind above 0.1 `NtrW` with bed gain `wind² × 0.2` and pitch
  `0.6 + 0.5 × wind`, otherwise none.

## Evidence

The stand-in voice's static record (source −1, everything else 0), the two
allocators and the free routine's early exit for it; the Music setting's
`CVTSS2SD`/`COMISD` against the double 0.01 with the parity branch; the
scheduler's `COMISS` of `(y² + x²) + z²` against the epsilon with the parity
branch; the bind routine's out-of-range branch; the story level hook's
ambience string and volume 1.0; the arena setup's double thresholds 0.01,
0.1 and 0.2, the single 0.5 and the names `NtrAN`, `NtrA`, `NtrW`.

## Out of scope

- Wiring `arena_ambience`: the per-arena calm table and its index come from
  the arena progression, which is not ported; the setup's clear for arena
  indices below 7 is also the caller's.
- The story hook's gate that sets the ambience only when the track changes.
- Audible EFX and the three lock helpers.

# Proposal

## Why

docs/PLAN.md, Phase 8 (polish and release), the frame-loop paragraph
(change `reproduce-the-frame-loop`) left the base library's name, string,
colour and vector helpers, the hook lists and the deferred ring's
cancellation out. Other units call them for asset names, settings text,
packed colours, headings and hook registration.

## What Changes

- `exa_formats::text`: the fixed sixteen-byte name (`Name16`: length,
  lowering, C-string and ANSI forms with the 0x85 ellipsis, offset writes,
  appends, extension replacement, byte and word compares, equality), the
  bounded, ANSI and case-insensitive compares (the ANSI case-insensitive
  compare falls back on the length difference), the zero-string compare,
  byte searches, the splice, the byte-reversed counter seed, the public
  truncating fraction, and the ANSI lowering that never reaches the last
  byte.
- `exa_formats::colour`: the packed RGBA8 store (unpack × 1/255,
  round-to-even saturating packs, scales and blends in byte units).
- `exa_formats::data_stream::latin1_len`: the UTF-8 to Latin-1 length walk
  beside the existing converters.
- `exa_physics::engine_math`: 2D/3D/ground lengths, component min/max with
  the SSE NaN rule, projection onto and removal along a unit (dot summed x
  and z, then y), the signed XZ heading (the body twist's `heading_of` now
  calls it) and the yaw and elevation directions.
- `exa_game::hooks`: the priority method list and the procedure list.
  `exa_game::frame`: tagged deferred calls with cancellation by data or
  procedure, and the performance-counter millisecond clock.
- `exa_render::passes::blend_layer`: bits 4–7 of a blend word.
- Review: the case-insensitive ANSI compare, the ANSI lowering and the
  projection order were corrected against the assembly; duplicate
  UTF-8 converters, frac and heading were folded into the existing ones;
  raw record offsets were removed.

## Evidence

The base library's string, name, colour and vector routines, the two hook
lists' insert/remove/dispatch, the deferred ring's removal walks, the
performance-counter clock and the blended list sort's blend-word read,
each checked against its assembly (SSE horizontal sums, `MINPS`/`MAXPS`
operand order, `CVTPS2DQ` with `PACKSSDW`/`PACKUSWB`, the `REPE CMPSB`
flag paths).

## Out of scope

Win32 glue (clipboard, message box, error text, cursors, WGL), thread
priority and suspend, the MXCSR modes, the second per-channel colour scale
(its registers are mixed up in the original and its result depends on an
unset register), wiring the hook lists or the millisecond clock into the
frame loop (Bevy schedules and `Instant` already pace it; the main loop is
unchanged).

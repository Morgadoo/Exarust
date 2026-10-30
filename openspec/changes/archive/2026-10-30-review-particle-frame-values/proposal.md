# Proposal

## Why

A lane extended the particle emitter with the original's per-frame pass
(light intensity, peak, full-update and emissive values, the sound walk
once a frame), each fire emitter's detail value, a one-shot ring deadline,
the fixed camera quad, a level-of-detail scalar, an arc texture scroll and
a looping fire sound. A review against the original's assembly found that
the level of detail divided by the plain distance where the original's
distance helper returns the squared length, the camera quad used the
camera's forward axis where the original uses its back axis, the arc
scroll dropped back by two instead of one past 2, the spawn pull had been
moved to the wrong source axis, the sound volume's bounds turned a NaN
volume into the low bound and rounded the bound in single precision, and
the fire sound started a new looping voice every frame (filling the voice
pool) and never freed a despawned flame's voice.

## What Changes

- The per-frame light values: intensity density·((1 + 0.05x)·base) and
  emissive 0.8·density·(0.9 + 0.2x) in double precision, a new peak of 1.1×
  a strictly brighter intensity, a full light update on the first frame,
  a one-shot template's every frame and a brighter frame.
- The sound walk runs once a rendered frame; its volume bounds are double
  products, compared unordered (a NaN is kept).
- Each fire emitter's detail value (the small and medium flames' size,
  twice it for the medium-small and large flames, four times the fire for
  every bed) and the level of detail detail·1000/d², d² the squared camera
  distance held at one or more.
- The fixed camera quad from 0.3 of the camera's back axis.
- A one-shot template's ring deadline.
- The arc's texture scroll: +0.015 a tick in double precision, minus one
  once past 2, on a material of each arc's own.
- The spawn pull along the source's second axis.
- One looping voice per flame for its template's sound, on a handle of its
  own (`exa_audio`'s `play_buffer_attached`), freed with its buffer
  reference when the flame goes away.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: per-frame particle values, fire detail and loop sound, arc
  scroll.

## Impact

`crates/exa_world/src/{particles,fire,flame,arc}.rs`,
`crates/exa_render/src/electricity.rs`, `crates/exa_audio/src/environment.rs`,
`crates/exa_game/src/fire_sound.rs`, `docs/RENDERING.md`, crate READMEs.

## Phase

Phase 5, visual and audio fidelity (Effects: particles: fire, sparks).

## Evidence

The particle system's derived-values routine (deadline), its per-frame
pass (light, emissive, sound walk, level of detail and the camera quad's
corners), the engine's squared-distance helper, the fire preset builders
(the detail field), the tick (the pull along the source's second row), the
ring builder (the source layout) and the tendril emitter's tick (the
scroll) in the shipped executable's disassembly.

## Out of scope

Binding the level of detail and the camera quad to a material, applying
the emissive scale to the parent mesh, body-attached emitters other than
the carried torch, the arc ribbon's strip geometry and per-branch scissor,
and the original's recreation of a source whose buffer arrives late.

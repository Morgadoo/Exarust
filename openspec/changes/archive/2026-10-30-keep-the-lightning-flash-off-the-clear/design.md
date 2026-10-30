# Design

## Context

The frame apply has a branch that, when a scene byte is set, clears with the
scratch colour times a three-float tint, halved, and moves the fog to
−0.8…0.1 of its span. Exarust tied that byte to the lightning strike.

## Decisions

- **The strike does not raise the switch.** A search of the executable's
  full listing finds one access to the byte (the frame apply's test) and
  none that stores to it; the scene object is zero-filled at creation, so
  the branch never runs in the shipped game. The strike routine's stores
  are its own fields only.
- **What a strike changes.** Only the record's fog colour, by
  I / (I + 2) · 3 times the lightning blue, along the existing flicker:
  first step 0.2 + 0.1·r from the run-time generator, then ±(0.1…0.3) every
  50…89 ms, scaled by the attenuation (1 − reach · 0.0002)², intensity held
  at zero or above, the strike ending 100…599 ms after it started. The
  clear, the sun, the ambient and the fog's reach are untouched.
- **Keep the switch modelled.** Its rule and its default-tint scenario stay
  in the spec; only the strike's link to it goes.

## Open questions

- The reader of the lightning-tinted record fog colour in the renderer is
  not identified (the shader's fog uniform comes from the scratch colour).
  Answer with REA by following the scene object's record-fog field from the
  renderer's draw set-up; until then Exarust draws it as the outdoor fog
  colour, which is where a strike shows.

Docs: docs/RENDERING.md (scene environment, clear colour and sky).

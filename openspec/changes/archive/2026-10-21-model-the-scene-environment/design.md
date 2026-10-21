# Design

The plug-in is modelled as a plain struct, `SceneEnvironment`, so every rule
is testable without an app; `environment::level_lighting_with` builds it from
the level record and the preset, places the sun, applies one frame and turns
the result into `Lighting`. Formats page: [docs/formats/RFC.md](../../../docs/formats/RFC.md)
(Environment, Grid building); rendering notes: `docs/RENDERING.md`
(Environment and weather).

Arithmetic follows the instructions: compares against 0.44, 0.4, 0.47, 0.54,
0.15, 0.9, 0.3 and 0.01 are double precision on single-precision values;
several products are formed in double and stored as single; bounds that
the pseudo-code shows as `min`/`max` are compare-and-store, so NaN passes
through as the original's does. The daylight factor uses the engine's cubic
arc sine; renormalisation uses the approximate reciprocal square root
(the result differs by up to about 1e-4 between processors, so tests allow
for it). The engine's fast sine, logarithm and exponent are replaced by the
library's (differences in the last bits).

The weather takes two generators: the engine's linear congruential one,
and the run-time library's, passed in as a closure, for a strike's first
step and the thunder's pitch.

## Open questions

- Which of the two fog colours the shader's fog reads (the record's, or the
  horizon colour times a renderer factor): to be answered from the
  renderer's constant upload with REA. The model uses the record's under a
  sky and the level's own without.
- The writer of the flag that turns a lightning flash on: REA cross
  references; the strike stands in.
- The sky object's creation (which gates the low-sun fog narrowing): REA in
  the environment unit; the sky flag stands in.
- The day record table's contents: decode the constant table and its
  set-up routine with REA, then feed the track.
- Where the level record's sun colour goes: the environment unit's loader;
  meanwhile it colours the sky-less sun light as before.

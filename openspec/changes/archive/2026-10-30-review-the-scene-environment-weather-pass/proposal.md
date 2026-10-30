# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the scene environment and grid
environment paragraphs: the sceneenv3 lane ported the rest of the scene
environment's arithmetic (engine trig, the engine's sRGB curve, the cloud
dimming's precision, the sun placement's tail, the sky switch's light, the
weather's rain effect and sound, the paused and view-led tick, the strike
placed in front of the view), mapped the grid's floors in centimetres and
turned its sides with the engine's trig, and ran the environment live in
the game. Review against the assembly found four rules ported wrong.

## What Changes

- `exa_world::scene_env`: the engine's fast sines and sine/cosine pair in
  the sun direction, the orbit and the three-angle rotation; the engine's
  sRGB curve (the presets' helper) for every linearisation; the cloud
  dimming's factor in double and `f⁴` squared twice; the sun placement's two
  bodies, light colour and intensity (3 / 4.5), the scene's `k` scale and
  follow-up flag; the sky switch puts the level's light back; the rain
  request creates the rain effect and sound, the tick drives the sound, a
  paused scene does not tick, a view-led one takes the view's azimuth; a
  strike is moved 200 cm in front of the view; `clear_weather` returns the
  stop events.
- `exa_world::environment`: `SceneEnvRuntime` and a `PostUpdate` system that
  ticks the loaded level's weather and republishes the fog, clear colour,
  ambient and sun light when the frame changes.
- `exa_world::grid`: the floor mapping as the original's centimetre multiply,
  `side_rotation` with the engine's pair, the merged cell nodes' words.
- Review fixes: the luminance sums x, y, then z (the lane reversed it); the
  legacy grid build's 0.6275 grey goes to the ambient setter (the lane sent
  it to the sun colour and invented a linear sun colour); the frame keeps a
  scratch colour apart from the clear, so under a sky the clear stays the
  level's background (the lane cleared with the horizon); the rain sound and
  effect exist only after a rain request.

## Evidence

The luminance helper's instruction order (x·w0, then y·w1 added onto it,
then z·w2 added onto that); the ambient setter's store, which the
constructor seeds with 0.314 and the frame copies to the renderer's
ambient; the frame apply's three clear calls (the background under a sky,
the scratch only for a clearing fog or a flash), the 3-wide multiply that
leaves the product's alpha at the constructor's 1; the rain request's
object creation and the tick's sound gate; the side rotation's row layout
from the pair helper's two outputs; the light node matrix's third row taken
from the body direction.

## Out of scope

The sky cube's masked tint upload, the rain particles, the rain and thunder
sounds, the lightning bolt, the merged `+Floor` / `+Ceiling` meshes and
their 20 cm subdivision, the writer of the flash tint (zero as the
constructor leaves it), and a driven time of day.

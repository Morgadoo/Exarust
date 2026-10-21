# Design

## Behaviour

**Clock.** The effects read the original's game clock: whole milliseconds of game time that stop while paused, a 32-bit word. Exarust follows the fixed clock (`Time` in `FixedUpdate`), so a tick moves it by 12 or 13 ms and it never drifts. The original widens the clock and a stamp to 64 bits before subtracting, so a stamp ahead of the clock gives a negative span; deadlines are summed in 64 bits. The domain wheel also reads the real clock (hover, label, choice flash).

**Engine helpers.** The sine-cosine helper writes the sine to its first output and the cosine to its second. The squared distance sums (x² + y²) + z²; the distance sums (x² + z²) + y² before its square root. A point through a 3×4 frame pairs (x·px + y·py) + (z·pz + t). Lerp is a + (b − a)·t; add-scaled is a + d·s. The maximum helper is the processor's (a NaN gives the second operand). Float-to-integer conversions in this group use the default rounding (ties to even).

**Wheel.** Icon i sits at angle (i + 2)·2π/6 (single precision): x = cx − r·sin, y = r·cos + cy, rounded; r = round(76·scale). The hit compares dx² + dy² < 433 (32·0.65 squared in double, rounded). The label uses the angle (i + 2)·π/3 in double, unrounded, a 4 px text unit: 40 px drop, 16 px height, 8 px padding around half the text width. The label fade is 0.005 a millisecond of a signed span, inverted when nothing is highlighted, and drops the last-highlighted icon after 200 ms. The open ease is (3 − 2x)·x² over the time left over 300 ms.

**Shell and scan.** Shell fade (1 − shown²)²; texture offset (t mod 40 000)·2.5e-5 in double; detail 900 over the squared camera distance raised to one. Both shaders take the fade plane: the node frame's world-up row and offset at.y − (centre.y − e), divided by e and scaled; e is the skinned geometry's half height or the rigid box projected on up, capped at the radius. The scan's size is the float 800; its shader has no texture position.

**Bubble.** Before the shrink the fade is max(1 − value, 0); while shrinking the value itself (0 to −1) is uploaded, the square-root curve the original computes being discarded. The swell s = 5·(charge − 0.8) from a strict double compare; scale 1 + 100·(s²)². The texture period is the integer 500 − round(350·(2 − c)·c), used as a 32-bit divisor of the signed span.

**Flame row.** Fades ±0.02/0.015 in double; lag points close 0.2 of the gap to their end plus 20 cm on y; speed 0.1·move + 0.9·speed in double; walks, swing (0.3 double), jitter (0.125 double, limits compared in double) only with a light; intensity fade·((swing·0.06 + 1)·base) in double; volume clamp(speed·0.001) · fade · 2; pitch walk 0.005 in double, limits compared in double, then ·0.8 in double, only with a sound; removal when the double fade is under 0.01, flags becoming the removed flag. Attaching sets every remembered point to the ends. The pair shift, interpolated ribbon ends, bounds, indices, UVs and vertex colours are modelled.

**Portal.** Grid steps round(half/6); cells placed in double; per cell the gate (2 in 5), life and look draws; the record's first point is the cell through the other frame, its second the cell through the current frame moved halfway to it, jittered ±1.5 cm (three unit draws, double) and moved by −0.015 of its gap to the body's root node position. The burst box is min + (max − min)/2 with half extents 5 cm past the max. The marker (`0x3DD0E004` emitter) is replaced only in the game's scene; Exarust hangs its light under the marker node when the game plugin sees it.

## Model

Plain functions and data per effect module; Bevy systems step `BubbleEffect`, `ScanEffect`, `ShellEffect` and `Flame` components, and `light_portals` lights `exa_world::PortalMarker` nodes.

## Open questions

- Which of a node's two stored frames is this tick's in the dissolve seeding: answered by reading which one the particle system integrates as the position (REA on `TParticleSystem`, outside this group).
- The flame row's pair-shift order relative to its tick, and its initial softness jitter: answered by REA on the fire factory and the scene's tick driver (outside this group).
- Whether the bubble's 120 cm geometry and the shader's `Scale` both apply: answered by the `effecttele` shader source in the packs.
- The domain wheel's button-flag clearing: answered by REA on the button class.

## Docs

`docs/formats/RFC.md` (the emitter records) gains the portal emitter.

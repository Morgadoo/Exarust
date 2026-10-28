# Design

The renderer stays on Bevy's WESL surface program with run-time flags; the
original's per-draw decisions are modelled as plain functions returning a
`DrawPlan` so they can be compared with the executable and wired later
without touching frame time. `DrawState` owns the six light slots and their
stale flag, as the engine's drawing state does.

Precision follows the assembly: the light colour's `2/π / taps` and the
filter steps are double and narrowed; `FogPos` and the time pair are single;
the refraction opacity compare widens to double. The material factory's
choices read the plug-in-masked feature word, not layer presence, except
where the factory itself checks the layer pointer.

The height-map rule is the only live change: the material loader already
held the texture size, so the switch-off test and the double scaling cost
nothing per frame.

# Design

The light is a level light made from a light record (`exa_world::
light_bundle`), so it shades, shadows and lights the GI as the level's
own lights do. A marker component keeps its intensity; the frame's system
raises it by the original's ramp and sets the Bevy light's intensity only
while it changes (about 0.8 s), then moves it each frame. The switch runs
when the player's possessions or hands change, from the item catalogue's
kinds, and makes or removes the light.

The ceiling probe is rapier's shape cast of a 10 cm ball up from the head
against the fixed colliders other than the props' stand-ins, which stands
for the original's sweep of the scene's static meshes.

## Open questions and how they were answered

- Where the switch's byte lies in Exarust's item body: the original reads
  the byte 0x21C past its section pointer; the installed database has no non-zero
  byte at data offset 0x21C and exactly the three night-vision helms at
  0x214, so the pointer includes the 8-byte header (corpus scan).
- The head the light hangs over: the part whose bone is 10, by the writer
  of the body's head pointer (REA).
- The frame time the ramp reads: the scene's frame milliseconds; Exarust
  uses the virtual clock's frame delta.

The format page extended is `docs/formats/OTHER.md` (Object database, the
item body's special kind).

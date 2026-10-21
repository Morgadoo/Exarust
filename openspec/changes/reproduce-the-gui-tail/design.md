# Design

## Behaviour

Each class is a plain type that takes what the form base and the
renderer would give it (client points, base answers, skin colours) and
returns what the original writes or answers, so it can be tested
without the GUI. Colours go through the engine's own helpers: the RGB
to HSV conversion and the HSV to RGB the palette already uses, the
saturating byte pack, the fast logarithm and exponential, and the sRGB
decode. The tone picker shares the skin tone's routine with the skin
job. The trade window builds on the career's basket totals and the
company's trade lines.

## Open questions

- What the hair material's shading routine writes for the picker's
  inputs. Answer: the character generator's change ports it; the picker
  hands over its three inputs.
- Which level meshes the wall probe sweeps (the original's mesh flag
  filter). Answer: REA on the probe's sweep and the level loader's mesh
  flags, then a capture of the retail camera against a wall.
- The skin colours of the swatch frames and the list band, and the
  second draw alpha's use. Answer: REA on the skin's ink table readers.

## Docs

`docs/UI.md` (Item views, Skill screen, Layers, pickers and the
remaining original widgets) and `docs/PLAYER.md` (camera).

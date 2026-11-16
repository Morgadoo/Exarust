## Why

Roadmap task B4, lane `itemsmat` (the triage's L09b: items, materials and
texture paint). The worn effects stepped their crystals' bytes on a copy
and dropped their switches, so a spent golem suit never weighed on its
wearer; a copy's bulk was guessed from its own byte; the material lookups
asked the libraries in the wrong order and some read the file's record
first; the CPU painter sampled magnified
masks nearest; the skin maps ignored the texture detail.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The worn tick's switches: on writes the golem-suit byte 1 and the body's
  standing stance factor 0.75 (a constant of the image), off writes 0 and
  the race's stance factor; both rerun the armour pass, the step reach and
  the view's encumbrance dots. The garment switches rerun the part values,
  the holding pass, the armour pass, the step reach and the dots. The
  apparel tick compares the stored weightless share. Every tick writes its
  byte in the item's own apparel body.
- The two bulk routines: a tagged id reads its header's template word and
  that record's model byte, one step, 0 without one.
- The material get-or-load asks its providers from the last to the first.
- The extension copy: no dot, a trailing dot and an over-long extension
  leave the same empty field, which every caller looks up.
- The character's skin job: side 1024 when `TextureDetail` is 3 or more.
- The character view's displaced value reads the body's weapon class word.

## What changes

- Worn effects write their bytes back and drive the suit's stance factor
  and the equipment refresh.
- Copies weigh as their template.
- Material lookups: libraries last to first, the library's record first
  for bodies, GI thinness and split flags.
- The painter filters magnified region maps bilinearly.
- Skin maps follow the texture detail.
- The view's displaced value reads the body's class word.
- The level, world-tier and archive migrations run the item factory's
  pass on factory weapons (the migration routine takes the factory; its
  callers passed none).

Not changed: a material slot naming an image no pack holds still binds no
map. The shipped models name 259 such images outside reflections, and some
may be images the engine builds at run time, which Exarust does not
register; the original's grey placeholder waits for that registry.

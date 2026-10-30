# Design

The sweep extends the existing modules; every rule is a tested function
with the original's numbers, and the GI lights run in the live effect
entities through `thaum::effect_light` next to the blast's, torrent's and
teleporter bubble's (docs/RENDERING.md, Effects and GI).

## Decisions

- **GI frames.** An emissive's `bMat` is the inverse of its mesh node's
  frame times the volume's own transform. Exarust passes world
  centimetres, so `bMat` is the inverse of the node's frame: the Barrier's
  cap frame (its place and flattened facing; the cap's apex is the shell's
  +z pole, the program's 100-unit shell the cap's sphere) and the wall's
  axes at the node's place. The barricade's frames write the node's half
  size (the drawn half width and height), which its `bExt` and box read;
  the wall now keeps that size and place.
- **Precision.** The flame ribbon's 0.2 share of the line, the domes' hum
  products by 0.2 and 0.4, the electric tone's level term, blends and
  intensity are double; the peak opacity's `curve·0.375` is single before
  the double difference.
- **Reuse.** The flame ribbon uses `thaum::engine::{length, normalize}`;
  the flame material takes the three `Anim` values from
  `fire_row::texture_animation`; the throw's voice fade stays
  `ThrowSound::fade`; the flame light's flags stay `fire_node::flame_light`.

## Open questions

- The Barrier's node basis orthogonalises its up against an unflattened
  facing; the cap frame flattens it (they agree for a level facing).
  Answer: REA on the frame's basis helper when a pitched facing matters.
- The flame material's blend state is not in the draw path read here;
  additive is a guess. Answer: a capture of a burning item.
- The deferred call clears a byte on the caster's controller that the
  existing code calls the HUD's ending flag. Answer: REA on its readers.

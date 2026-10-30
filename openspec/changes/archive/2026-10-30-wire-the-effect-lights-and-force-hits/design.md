# Design

## Shapes as entities

Each emissive is its own entity carrying `GiShape`, `ShapeVolume` and a
`LightOwner` back to the effect; the effect carries `EffectLight`. The
render crate's `GiShapes` lists shape entities by their component, so the
original's append and swap removal hold. A light whose owner is gone is
despawned (the effect's destroy frees its emissive).

## Units

The original maps each multi-point and gradient-line position into the GI
volume's texture space before uploading it, and its bodies measure
`d²·Atten` there: a blast's 5000 reaches √(1/5000) of the block's width,
72 cm in a 51.2 m block. Exarust's shapes keep world centimetres in the
component; the pass scales the positions and the voxel by one over the
width in centimetres (the vertical axis takes the same unit, as the
original's `ZTrn.x` does).

## Open questions and how they were answered

- Where the blast's light points come from: the frame's last write per live
  chunk, the bent spine's second sample mapped back to the world (REA,
  the blast's frame). A charging chunk whose spine is undefined takes its
  own place (inferred).
- What the "5 s recoil" is: the character field the hit sets to 5 is the
  balance's landing momentum weight (fading 0.05 a tick), the same field a
  landing sets (REA, both hits; the feet notes).
- The torrent's level: the torrent's charge (its frame's level).
- The teleporter bubble's radius: read from its mesh node in the original;
  Exarust uses 120 cm times its swell scale (guess, labelled).

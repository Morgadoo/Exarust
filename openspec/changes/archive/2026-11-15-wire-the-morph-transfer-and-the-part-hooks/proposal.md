## Why

Roadmap task B1 (wiring lane `wiremorph`): the open rows of the character
morphing, skinning and motile-part families, and findings F249, F250 and
F283. The morph transfer solved its barycentric weights with a clamped
region test where the original solves them on the triangle's plane, and it
skipped neighbours at the same place where the original weighs them 1/0.
The second wind's heal sound never played, the swing voice's looping
sources had no life cycle, and the record forgot a dropped weapon only when
a collapse started, not when the weapon came loose.

## Evidence

REA (Exanima.exe), checked against the assembly where order or constants
matter:

- The transfer's per-element weights: the kd-tree visit keeps the closest
  point of the nearest triangle within 10 cm; the weights come from that
  point projected along the face record's normal onto the plane through the
  first corner, then a 2x2 solve chosen by `|x0+x1|` against `|y0+y1|`
  (`comiss`, NaN takes the second branch), the third weight one less the
  others. The face record's normal is the edges' cross product scaled by
  `rsqrtss`.
- The transfer's diffusion weighs each set neighbour `1/sqrtss(d²)`
  (`divss`); the engine masks every floating-point exception (its MXCSR
  set-up ORs in `0x8fc0`), so a neighbour at the same place weighs +inf and
  the mean is NaN. No shipped garment reaches that case (tested over all
  8002 garment pieces of both races).
- The race model loader takes the morph set from the morph file's first
  root; only a file whose root has no set reaches the child search and the
  numbered-target fallback. Both shipped morph files carry it on the root.
- The second wind calls the heal sound at the character's position
  (`StmHeal01`, 0.1, 1).
- The weapon drop calls the holder's came-loose hook; the shield and torch
  drops do not. The unlink writes the freed body's link-iteration count as
  the global 2 plus its subtree's node count less one.

## What Changes

- `exa_formats::morph`: `face_normal` and `plane_barycentric` (the
  original's solve); `MorphBody::transfer` uses them and keeps the 1/0
  weight. `exa_assets::morph_controller::morph_barycentric` delegates to it.
- `exa_audio`: `second_wind_heals` turns `SecondWindFired` into
  `HealSound`; `swing_voices` keeps a `LoopSources` per fighter, reserved
  at its first swing and freed at its death.
- `exa_physics::grip_solve::release_freed_grips` sends `CameLoose` for a
  weapon whose holder is still in the world and sets the freed body's link
  iterations; `exa_game::equip::forget_loose_hands` runs the record's
  `forget_loose` on it (replacing the collapse-only `forget_collapsed_hands`).

## Not done

- The transfer's skin-weight branch (garments whose render word has
  `0x200000`, most of the shipped garments, take skin records from the
  body through the morph), and the body records it needs (B4).
- The creator's live re-morph (F41) and the GPU build's morph-delta stream
  (F8).

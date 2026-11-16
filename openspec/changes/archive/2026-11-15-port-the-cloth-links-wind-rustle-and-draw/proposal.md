## Why

Roadmap tasks B4 and B1 (lane `cloth`), component TVCCloth, findings F308,
F309, F310 and F311 of the physics audit. The cloth's links were the mesh's
unique edges and opposite-vertex bends in triangle order, and its edge pass
was the square-root-free branch the game never takes (about 12% softer a
pass). The wind hook and the tile sample had no caller, so no cloth was
ever blown; the rustle was computed and never heard; the mesh write-back
took only the newest positions, with the normals of the rest pose.

## Evidence

REA (Exanima.exe), checked against the assembly where float order, NaN
branches or constants matter:

- The set-up's link builder walks out from the pins (vertex 0 without
  any) over each element's corner fan in the mesh's incidence order. The
  pick from the open list compares each entry with the first one only and
  keeps the last that is smaller; the taken slot is filled from the end.
  A taken vertex lists its fan's neighbours less the far end of each
  face's longest edge (squared distances `(x² + y²) + z²`, `comiss` with
  unordered results taking the other branch), links it to each neighbour
  not yet settled, and pairs its neighbours unless a settled vertex
  between them, or a face about the first joining the second to a settled
  vertex, shows the pair was linked from there; the pairs go to a list by
  the cosine of their arms (`> 0.7` in double) or to the other. The set-up
  appends the walk's links, then the in-line pairs, then the bent ones,
  each storing half its rest distance (`(x² + z²) + y²` under the root,
  times 0.5 in single).
- The constraint pass calls the full-correction edge pass, because the
  switch that selects the other one is 1 in the data and nothing writes
  it: `d = b − a`, `f = 0.5 − rsqrtss(|d|²)·h`, `a += f·d`, `b −= f·d`.
- The integrate loop adds `((v + (v̄ − v)·drag)·keep + g) + x`.
- The wind hook, installed into every simulation and run once a step:
  strength = scene preset wind × 0.3 (double); each of the 256 tile samples
  draws once from the engine generator and walks (`(r − 0.5)·s` in range,
  `∓ s·0.5·r` outside, NaN in range), `s = strength × 0.2 × 4` (double);
  then the direction `(0.6, 0, 0.3)` is normalised in place (`rsqrtss`)
  and the tile's frame is built from it (up less its part along the wind,
  the cross product, each normalised by `rsqrtss` without a zero mask).
- The integrate slot blows each particle, while the strength is above
  0.001 in double, by `dir·((0.25 + n)·strength)`, `n` the tile's bilinear
  noise at the particle's last place in the tile's frame (cell fraction of
  `x·0.005` in double, wrapping at 16).
- The rustle: only for a record with a sound; above 400 squared the voice
  is retriggered when its active, restart and loop bits are clear (pitch
  0.5), then placed at the centre with volume `0.5·tail + 0.0001·|Σv|`
  (the sum in double) and pitch `minss(0.5·v + 0.5, 2)`; else the stop
  bit; the tail decays ×0.9 in double each awake tick.
- The draw update lerps the last two particle buffers by the frame
  fraction (`a + (b − a)·t`) and rebuilds the normals with the mesh's flat
  normal builder.

## What Changes

- `exa_physics::cloth`: `incidence`, `walk_links`, `Edge::half_rest` and
  `Edge::at_rest`, `Cloth::from_mesh` (walked links) and
  `Cloth::from_mesh_linked`, the full-correction `correct_edge`, the
  integrate order, `Wind` (`advance`, `in_frame`, `sample`), `WindAt` and
  `step_before_level_in`, the rustle math, `ClothRustled`. The cloth's own
  wind, noise and tile fields and `walk_noise` are gone.
- `exa_formats::rfc`: `Mesh::element_incidence`, `cloth_normals`.
- `exa_assets::ClothMesh`: `links` (the walk over the whole mesh at load)
  and `part_keys`.
- `exa_world::cloth`: particles in element order, links from the file's
  walk; `SceneWind` (the frame's preset wind, copied in) and `ClothWind`
  (the simulation's, its hook before the cloth stage); the rustle sent from
  the stage; the draw update between the last two ticks with the normals
  rebuilt (`ClothShading`).
- `exa_audio::cloth_rustle`: the rustle voice.

## Out of scope

- The body side of the cloth contact (impulse to the body, its wake) and
  the contact's addition to the rustle tail: the body is not mutable in the
  cloth stage.
- The tangent rebuild of a normal-mapped first part (row stays missing):
  the per-part tangent buffer's fourth lane is not read yet.
- The re-placement reset from the body placements (TMotileDynamics, another
  lane), the draw countdown and its sleep flag, the two-pass draw callbacks
  (their slots' passes are not traced), the grab records.
- The positions the walk reads are the file's centimetres through metres
  and back (`× 0.01 × 100`), which can differ in the last bit.

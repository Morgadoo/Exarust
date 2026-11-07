# Design

Rules: docs/RENDERING.md (Cut-away fade); docs/formats/RFC.md (node
header, `flags2`).

## Reverse-engineering questions and answers

- How does the main pass draw a fading mesh? REA on the view's draw
  sequence: after the opaque, decal and blended lists a dedicated pass sets
  the view mask to fading (and settled, without supersampling), lays the
  meshes' depth with colour writes off (opaque and alpha-tested lists),
  then draws their colour with depth writes off, the material's diffuse
  alpha set to one less the fade amount and its blend word to source
  alpha, one minus source alpha (restored after). A blend, not a dither.
- What does the settled state do? Without supersampling it joins that
  pass; with 2–4 sub-frames the opaque pass includes it and the
  supersampled draw hook draws it into half of the sub-views; with more,
  only in the first round.
- What sets the fade target? The mesh loader: 1 with `flags2 & 0x08000000`,
  else ½ (the mesh constructor's default is ½). It also drops the parent
  carry of a candidate whose parent is not a mesh.
- What are the mesh callbacks? The static mesh installs a ray callback
  (one-sided ray–triangle, nearest within reach) and a segment callback
  (a sphere swept along the segment; the detector passes 50 cm). The
  original stores each triangle's plane and uses approximate reciprocals
  for the nearest point; Exarust derives the plane from the winding and
  divides exactly (documented deviation).
- Do shadows fade? No: the shadow passes do not read the cut-away bit; the
  dithered caster belongs to state bit 4 (a dissolve effect).

## Live wiring

The detector runs in `PostUpdate` after the visibility pass, so a mesh is
"in a drawn sector" when one of its parts is in view (a partless fade
plane when its node is not hidden). The material swap is per part: a
cloned `ExaMaterial` asset per fading part, released when the mesh is
solid again.

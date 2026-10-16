## Why

At the original's main-menu camera, with shadows on, the two wall torches lit
only small patches of their walls and floor. The lit patches had straight
edges. With `--no-shadows` the scene looked close to the original's capture.
The menu's cobwebs (`cobweb01`) hang around both torches: big two-sided quads
with an opacity map, drawn with alpha blending at opacity 1. Exarust cut
shadows only for alpha-tested materials, so these quads cast solid shadows
and walled each torch in.

The original decides when to cut a caster's shadow by the material record,
not by how the surface blends. It casts through its alpha-tested caster when
the blend word is alpha-tested (`0x4000`), and also when the material has
the opacity flag and an opacity above 0.75. The flag is `0x400000` in the
record's flags, and the loader also sets it when an opacity map loads, so a
record can have the flag without an opacity layer. That caster turns culling off and
discards texels whose texture alpha is below ½. The texture is the opacity
layer, or else the diffuse layer. The cut does not apply the material's
opacity or square the map.

A second fault came out of the same logs. Faces scheduled on the renderer's
first frame were marked done without being drawn. A face that was never
drawn reads as wholly shadowed.

Phase 5 (visual fidelity), the main-menu scene.

## What Changes

- Materials carry `cut_shadow`: an alpha-tested blend word, or the opacity
  flag (from the record or a loaded opacity map) and an opacity above 0.75.
  It is set whatever the blending.
- The cached cube pass cuts the shadow of alpha-tested materials and of
  materials marked `cut_shadow`. The cut is the opacity map's alpha (or else
  the diffuse map's) below ½, as the original does. Culling is off for these
  casters, with or without a texture to cut with.
- Faces the renderer could not draw now stay dirty. This covers the first
  frame, a pool or pipeline not ready yet, and the frame's draws missing.
- `docs/RENDERING.md`: the caster rule, the face rule, and the menu check
  that replaces the "menu renders darker" open question.

Evidence: REA/disassembly of `Exanima.exe` 0.9.5.2g (the mesh-part setup
that sets the part's cast and alpha-test flags, the 0.75 constant, the
material loader that assigns the cut texture, the cube caster loop and its
alpha-tested program). Corpus: the menu's `cobweb01` material record (blend
word `0x0203`, flags `0x400017`, opacity 1, opacity layer `CobWb01O`), and
a scan of the corpus's 1584 material records: the opacity flag and an
opacity layer differ in 61 of them (`harhps1` has the flag and no layer).
Captures: Exarust renders of the menu against the original's capture.

Out of scope: the original's other no-cast rules. Additive materials (blend
destination ONE) cast nothing in the original, and refractive glass casts at
any opacity; Exarust keeps its opacity ≤ 0.8 / extra-flag `0x4` rule. Also
out of scope: which texture cuts when a material has both a diffuse and an
opacity layer (the original takes the later layer).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: shadow casters, cached cube shadows, alpha-tested casters (new requirement).

## Impact

- `exa_render` (`material.rs`, `cube_shadows.rs`, `cube_shadow.wesl`),
  `exa_assets` (`model.rs`), `docs/RENDERING.md`.

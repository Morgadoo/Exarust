## Why

Roadmap task B4 (lane `lighting`, triage lane L07 "lighting, GI & shadows").
Two of the lane's PORT items: an unranged light still fell off over 500 m in
Exarust, and the cube shadow pass never culled a caster's front faces.

## Evidence

REA (Exanima.exe 0.9.5):

- The light activation (class slot after the live update) sets the
  attenuation to 0.8/radius only with node flag 0x40000 and a radius above
  0; otherwise the attenuation is 0 and the shadow far plane the float
  maximum. The surface programs' falloff is `(1 − attenuation·d)⁴`, so an
  unranged light reaches everywhere at full strength.
- The per-face cube caster draw switches the cull face by the mesh node's
  render flag 0x800 (front for a set bit, back otherwise) through the cached
  cull-face setter.

The retail comparison of the exanima01 start room (the gap the brief names)
is traced in `context/artifacts/2026-11-17-lighting/README.md`: most of it is
the comparison pose, not a lighting port.

## What changes

- The surface shaders give a light at Bevy range 500 m (`UNRANGED_RANGE`) no
  falloff (`exa_render::lights::falloff`).
- Meshes with render flag 0x800 get `ShadowCullFront`; the cube caster pass
  culls their front faces.

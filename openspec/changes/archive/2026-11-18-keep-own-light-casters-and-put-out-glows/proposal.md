## Why

Roadmap task B4, lane `gi` (triage2 lane Q15: lighting, GI and shadows).
Rounds 308 and 312 left exanima01's near-flame excess (1.3-3x retail within
about 1.5 m of a floor torch) in the GI share and named three candidates: the
injection into the light's own voxel, the surface's GI sample point near the
floor and the cube caster set of the light's own object. The fixture glow
writes of the put-out and switch-off were read but not ported.

## Evidence

- The original's injection program (`gi_inject_f`, decoded from
  `shading.flb`) and its surface GI sample (`ambient.glsl`) match
  `gi.wesl` and `shading.wesl` term for term: the voxel holding the light
  takes the same fall-off, shadow and shaping as any other, and the sample
  sits 20 cm out along the normal with the half-voxel read offset.
- The cube caster walk (asm) marks the light's parent's mesh children, the
  parent and the grandparent unless a mesh's render word has `0x1000`, and
  stops at the object node; the fire's light hangs under the emitter node
  that takes the marker's place under the host mesh, so the torch head is
  excluded exactly as before. 38 shipped meshes carry `0x1000` (corpus).
- Picture pairs on the worker (exanima01, retail pose): the documented
  `--floor-torch` pose floated the torch head 6 cm above the floor (light at
  11.7 cm). With the head resting (light 6.8 cm) the floor rings beyond
  100 px match retail within 0.9-1.2; the excess left is 1.2-1.7x at
  30-84 px below the flame.
- The put-out (asm) zeroes a mesh node's emissive scale and its first part's
  (flag `0x10`, `+0xf4`), and its mesh children's; the switch-off every part
  of the node, each mesh child's first part, and a voxel-light child's
  intensity without the colour update. Neither writes a point light.

## What changes

- `exa_world::self_shadowless` keeps meshes with `KEEP_OWN_LIGHT`.
- `exa_world::mechanisms::put_out_glow` and `switch_off`; the daylight path
  darkens the mesh the first fire marker hangs from.
- `exa-tool corpus render-words` lists the keep-own-light meshes.
- `docs/DEBUGGING.md`: the exanima01 recipe rests the torch's head.

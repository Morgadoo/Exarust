# OpenPBR for Exarust rendering — analysis

[OpenPBR Surface](https://github.com/AcademySoftwareFoundation/OpenPBR)
(ASWF, a MaterialX subproject, Apache-2.0) is a layered surface shading model
for production rendering: base (diffuse + metal), specular (GGX, IOR,
colour, anisotropy), coat, fuzz, subsurface, transmission, thin film,
emission, geometry (normal, opacity, thin-walled). The reference
implementation is a MaterialX graph; there are WebGL viewers and an Adobe
BSDF implementation. Spec and parameter reference:
<https://academysoftwarefoundation.github.io/OpenPBR/>.

## What the original renders

From the data and the executable (`docs/formats/RFC.md`,
`docs/research/engine-classes.md`): a forward OpenGL renderer with many
dynamic lights, cube shadows and voxel GI. Materials are a legacy
**specular workflow**: diffuse colour, specular colour, and texture slots for
diffuse, normal, specular, gloss, height, reflection and detail. GLSL is
generated from fragment libraries (`shading.flb`, obfuscated). Items and
characters get procedural materials (steel grades, iron, copper, leather,
cloth types, skin) from material classes in code.

## Can Exarust use OpenPBR?

**Yes, as a subset, and it is a good fit for the material *model*, but it
must not replace the original look by default.**

- **Feasible in real time on Bevy/wgpu:** base, specular (colour, IOR,
  roughness, anisotropy), metalness, coat, fuzz (sheen), emission, opacity,
  thin-walled. Bevy's `StandardMaterial` already covers part of this
  (specular tint, clearcoat, anisotropy, transmission), but a custom
  `Material` with a WGSL implementation of the OpenPBR lobes gives exact
  parameter semantics.
- **Only approximations in real time:** subsurface (skin: screen-space or
  pre-integrated), volumetric transmission, thin film (cheap analytic
  approximation possible).
- **Not available:** MaterialX in Bevy. The lobes are ported to WGSL by hand,
  and the MaterialX graph and OpenPBR viewers serve as references for
  validation.

## Mapping the original data

| original | OpenPBR |
|---|---|
| diffuse colour × diffuse map | `base_color` |
| specular colour × specular map | `specular_color` (`specular_weight` from its luminance) |
| gloss map | `specular_roughness` (curve to be fitted against captures) |
| normal / height maps | `geometry_normal` (height for parallax or detail) |
| reflection map | environment / `specular` IBL |
| metal procedural materials (steel, iron, copper) | `base_metalness` = 1, `base_color` from the grade |
| polished/painted metals, lacquered wood | `coat_*` |
| cloth classes (velvet, silk, padded, fine) | `fuzz_*` + `specular_anisotropy` for silk |
| skin (`skinhuman`, decomposition masks) | `subsurface_*` approximation |
| emissive effects (thaumaturgy) | `emission_*` |

## Recommendation

1. **One material type, `ExaMaterial`,** whose parameters follow OpenPBR
   names and units. The loader maps the original fields as above, so data
   flows through one well-specified model and can be exported to
   MaterialX/USD for offline comparison.
2. **Two shading modes for that material:**
   - **Original (default):** reproduces Exanima's lighting model. It is
     derived from the GLSL captured with RenderDoc/apitrace (`docs/PLAN.md`,
     Phase 5) and compared with side-by-side screenshots. Fidelity is the
     project's goal.
   - **Enhanced:** the OpenPBR subset in WGSL, as a user option for a modern
     look.
3. Phase 5 order: capture the original shaders, build the parameter mapping,
   implement the Original mode, then the Enhanced (OpenPBR) mode.

Until then the runtime uses Bevy's `StandardMaterial` with the mapping above
(base colour, normal map, roughness defaults).

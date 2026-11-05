# Proposal

## Why

The sun map's caster pass and its receivers' filter were the parts of the
render pipeline (backlog system S8) still on Bevy's terms: the fit computed
the polygon offset and the softness the original writes into the sun light,
but Bevy's directional shadow pass has no caster-side offset and its filter
no softness input, so the softness source (round 56) reached only the
caster depth nudge, and the shadow edge stayed Bevy's 2–6 cm whatever the
sky. Bevy's shadow pipeline also culled no faces and cut every masked or
blended caster by the colour pass's alpha rule.

Phase: the rendering milestone of docs/PLAN.md (render pipeline, S8); no
PLAN.md edit in this change.

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly): the
sun map's render routine (depth function less, `GL_POLYGON_OFFSET_FILL`
with factor `1/(h·0.001) + 1 + (c'·20)/(h·0.01)` and one unit, the caster
nudge, back-face culling with the node's front-cull bit, and per alpha-cut
part the alpha-tested program, its cut texture on unit 0 and
`GL_CULL_FACE` disabled around the draw), the caster program builder and
its two shader texts (the alpha-tested fragment is `if (texture(Tex0,
vrTex).w < 0.5) discard;`), and the decoded `lights` template (the sun's
`smp.xy = vrSdw.xy + rs.xy · Light[3i+2].x` over the `SdwSmp` taps, turned
per pixel, `smp.z = vrSdw.z` unbiased, summed and divided by the tap count
through the light colour).

## What Changes

- `exa_render::sun_fit::SunShadowFilter`: the receivers' filter radius
  (`softness · 0.5 · 0.005` of the map) and the casters' polygon factor and
  unit (2⁻¹⁶ of the window depth), from the fit of the active camera drawn
  first; carried to the shaders in the light records' first element
  (`exa_render::lights`, binding 130).
- `shading.wesl`: `sun_shadow`, the original's sun filter on the fitted
  map: the unbiased map position, the fragment's disc taps (the point
  lights' taps, split over sub-frames as theirs are) at that radius, each a
  hardware 2 × 2 comparison, averaged; no far bound, the clamp-to-edge
  border outside the map. Bevy's filter stays for `--bevy-sun-cascades`.
- The caster pass's state on Bevy's shadow pipeline
  (`exa_render::sun_casters::specialize_sun_caster`): every sun caster
  writes its depth less `factor · slope + units` (`sun_caster.wesl`), plain
  casters cull their back faces, alpha-tested ones cull nothing and cut by
  the cut texture's alpha alone; other casters no longer discard. The
  terrain gets the plain caster shader as its prepass fragment.
- `EXA_SUN_SOFTNESS=<c>` (debug) pins the softness source the sun map
  reads; `exa_world::sun_shadow=debug` logs the filter.
- Docs: RENDERING (Sun shadow fit), DEBUGGING, crate READMEs.

# Proposal

## Why

Round 55's arena weather lane left three night questions: night shots look
warm and orange; on `arenao3` the moon's shadow shows a hard octagon and a
light stripe, taken for the sun map's fit at low daylight; and the night
branch writes a value into the view's post-processor amount whose use was
not traced (nor what Exarust's fresh scene environment per load changes
about it).

Phase: the world and rendering milestones of docs/PLAN.md (environment);
no PLAN.md edit in this change.

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly): the
outdoor preset (the night branch's value: the environment's cloudiness and
time of day before the draw, handed to the view's amount setter), the
view's amount setter and post-processor maker (made when the view's post
flag is set, which its constructor does, or the amount is above 0.01 in
double; the amount is copied into the post processor), every reader of
that post-processor field (none: its passes read the bloom strength,
sharpen and desaturation fields only; the other writers are the arena
setup and the powers screen), the sun placement (the moon's choice under
0.44 daylight, the two lifts, the record's sun colour and the 4.5
intensity written to the light, the dimmer written to the light's
softness source, 0 without a sky), and the shadow-caster flags of
`arenao3`'s nodes and props.

## What Changes

- `exa_world::scene_env`: `sun_softness_source`, the dimmer the sun light
  carries as its softness source; `sun_dim` and the outdoor preset's
  `view_sun` documented with what the original does with them.
- `exa_world::environment`: the live frame writes the source into the sun
  map's region (`SunShadowRegion::c`), which the fit's softness and caster
  bias read; the load logs the source and, under `EXA_TIME_OF_DAY`, the
  body, its direction and the light colour; the post amount is logged as
  stored and never read.
- Docs: RENDERING (the softness source), DEBUGGING, formats/OTHER (the
  indoor amounts), crate READMEs.

## Findings without a code change

- The environment's night matches the original: the moon lights with the
  first day record's grey 0.1015 at 4.5, the fog and GI sky light take the
  record's blue fog colour, the ambient is the level's, the sky cube is
  tinted by the record. The orange is the arena fires' direct and GI light,
  which dominate once the sun is down (arenao3 and arenao4 have stand fires
  and lanterns; arenao1 and arenao2 stay grey). A retail night was not
  reachable: outdoor arenas need a career past the indoor ranks and the
  user's prefix has none (`Arena001.rsg` is empty), and no new career was
  made.
- The octagon and stripe are geometry: the octagonal tower roofs on the
  moon's side of the arena, which the original flags as casters too, under
  a moon 33° up. They stay with Bevy's cascades, sit at the same world
  place from a top-down camera, and move towards the towers as the moon
  rises (0.90).
- The scene environment's persistence only reaches the night value (the
  preset rewrites everything else it reads), which nothing reads, so a new
  environment per load changes nothing on screen.

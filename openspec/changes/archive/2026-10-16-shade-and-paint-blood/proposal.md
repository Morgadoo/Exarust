## Why

The original's arena screenshots show blood running down a belly, smears
on chests, arms and thighs, and bloodied braies. Exarust has no blood.

In the original, body blood is one byte per vertex, the alpha of the mesh's
colour stream (255 clean), lowered around each wound and never raised.
Materials with "dirt mode 4" (the human skin, painted garments, factory
items, hair, a bleeding race's skin) compare it with the tiling speckle
`spatter01`: dark red, speckled edges, a wet sheen.

This change adds that channel, the painter and the shading, with a debug
flag that paints test wounds. Wounds from hits are the next change, L7.

Phase 5 (Visual and audio fidelity), milestone M5 "looks and sounds like
Exanima", item "Effects: blood sprays and spatter decals". It also serves
Phase 4's M4 "first duel".

## What Changes

- **Bleeding races**: `races.rdb` records carry a flag word at `0x18`
  (inside the name field); bit 0 is "bleeds". `Race::flags` and
  `Race::bleeds()`.
- **Blood channel**: wounds paint a byte per vertex into the colour
  stream's alpha (`exa_character::blood`, Unorm8x4 white). A model's meshes
  stay shared by every character; the first wound that lowers a value of a
  shared mesh gives the struck character its own copy, painted in place by
  later wounds. Clean characters copy nothing.
- **Wound targets**: a character's skinned meshes (body and garments) and
  every other mesh under its head part (hair, helmets), never held items,
  rigid items on other parts or parts hidden under apparel.
- **Painter**: the original's maths on the current skinned positions:
  centre P − D/2, a ball behind it and a smear ahead along D, f = 1 − e/r²,
  255·(1 − f·g²) with g = f + (1 − f)·u, min-only, rounded.
- **Shading** (`exa.wesl`, `exa_render::material::BloodProgram`): dirt mode
  4 for the skin, cloth and hair programs: speckle at UV×2 (×1 on hair),
  da = clamp((a − α)/a·k), k 1.3 on cloth, the skin's 0.02 alpha floor, the
  three tints and the wet roughness.
- **Materials**: blood on the human skin (`_HumanSkin`), painted and
  run-time apparel (`_procmat1`), factory items (`_procmatr`), any record
  whose extra flags select dirt mode 4 with a diffuse map, and a bleeding
  race's body-model materials whose names start with `s`.
- **Debug helper**: `--debug-wound bone,radius_cm[,dx,dy,dz]`, repeatable,
  paints test wounds on the player's or the character view's body.
- Docs: `docs/RENDERING.md` (Blood), `docs/DEBUGGING.md` (Wounds),
  `docs/formats/OTHER.md` (race flags), `docs/PLAN.md` (Phase 5), crate
  READMEs.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, `exe.asm`):
  the wound painter of the skinned mesh (centre P − D/2 from a −0.5
  constant, the projection test against 0, 1.0 and 255.0, the random draw
  per vertex inside the wound, a min-only store rounded to the nearest
  byte), the targets (the `_` skinned child and the head body's children),
  the white fill of meshes without colours, the race loader turning dirt
  mode 4 on for a bleeding race's materials whose name starts with `s`
  (`context/re/wounds-and-blood.md` §1.1–§1.6, refreshed this change).
- The game's shader sources (`context/re/shaders/`): `shadesss_f`,
  `shadeopt_f`, `shadehar_f` and `shadetrl_f` dirt mode 4 blocks.
- Corpus: `races.rdb` flag words of all 18 records; the race body models'
  materials (types and diffuse maps); `spatter01.rfi` (1024², BC4,
  11 mips) in `Resource.rpk`.
- Live retail screenshots (`context/re/refs/blood/`, `combat-live/`), and
  Exarust in game through `gpu_run.sh` with `--debug-wound`.

Out of scope:
- wounds from hits: the bleed test, the 7-tick delay, the striker-anchored
  contact point and travel, the radius from R, corpses painting pending
  wounds (L7);
- blood sprays (L9), blade stains (L8), floor blood;
- the hair program itself (Exarust draws no hair yet) and the translucent
  program (`shadetrl`); dirt modes 1–3;
- blood lifetime across re-dressing and saves (L7 and the retail requests).

## Capabilities

### New Capabilities
- `blood`: the bleeding races, the per-vertex blood channel, the wound
  painter and its targets, and how surfaces show blood.

### Modified Capabilities
- `world-viewer`: the `--debug-wound` flag.

## Impact

- `exa_formats` (`races.rs`: `Race::flags`, `bleeds`, tests)
- `exa_render` (`material.rs`: `BloodProgram`, three flags, `with_blood`;
  `exa.wesl`: the blood block)
- `exa_assets` (`model.rs`: the speckle texture, apparel, factory and
  dirt-mode-4 materials, the race rule; `human_skin.rs`: the human skin)
- `exa_character` (new `blood.rs`: `PaintWound`, the painter, per-character
  copies, `DebugWounds`)
- `exarust` (`main.rs`: `--debug-wound`; `viewer.rs`: the character
  view's and the player's bodies marked as its targets)
- `docs/RENDERING.md`, `docs/DEBUGGING.md`, `docs/formats/OTHER.md`,
  `docs/PLAN.md`, the READMEs of the crates above

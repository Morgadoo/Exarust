## 1. Races

- [x] 1.1 `exa_formats::races`: `Race::flags` (the word at `0x18`, inside the name field) and `Race::bleeds()` (bit 0), with `BLEEDS`
- [x] 1.2 The synthetic test reads the flags after a short name; `shipped_races` pins every record's flag word and who bleeds (Human `0x0F` … DemonA01 `0x23` bleed; Skeleton, Ancient, Wraith `0x300`, golems `0x1030`/`0x2030`/`0x20B8` do not)

## 2. Shading

- [x] 2.1 `exa_render::material`: `BloodProgram` (cloth, skin, hair) with `of_type`, `flag`, `spatter_scale`, `amount`, `tint`, `roughness`; flags `BLOOD`, `BLOOD_SKIN`, `BLOOD_HAIR`; `SPATTER`; `ExaMaterial::with_blood` (flag and the speckle as dirt map, none without it) and `blood()`
- [x] 2.2 `exa.wesl`: under `VERTEX_COLORS`, for a material with a blood flag: the speckle at UV×2 (×1 hair), da = clamp((a − α)/a·k), the skin's 0.02 floor, the three tints on both diffuse shares and the wet roughness before the roughness clamp
- [x] 2.3 Tests: `thick_blood_is_dark_red` (skin (0.21, 0.0105, 0), cloth (0.42, 0.021, 0), hair (0.315, 0.021, 0.014), half blood on skin), `blood_shows_where_the_alpha_falls_below_the_speckle`, `blood_wets_the_surface`, `blood_programs_follow_the_record_type`

## 3. Materials

- [x] 3.1 `exa_assets::human_skin`: the human skin takes `BloodProgram::Skin` with the speckle; the ancient skin none (`SkinKind::shows_blood`)
- [x] 3.2 `exa_assets::model`: painted apparel and factory items (`painted_material`) and unpainted run-time apparel with a diffuse map (`apparel_material`) take cloth blood; `ModelLoader::spatter`
- [x] 3.3 Records whose extra flags select dirt mode 4 with a diffuse map take the program of their type; a bleeding race's (`races.rdb`, cached per body-model stem) body-model materials whose names start with `s` get the mode first
- [x] 3.4 Tests: `race_blood_takes_the_skins_not_the_eyes`; corpus (skipped without `EXANIMA_DIR`) `shipped_blood_materials` (humanbase skin → skin, eyes none; ogre `skinogre` → cloth, `ogreeyemouth` none; demon `sdemona01` → cloth; ancient and skeleton none; the painted braies → cloth; every blood material binds the speckle) and `shipped_spatter_speckle` (`spatter01` 1024²; texels ≥ 0.5, 0.25, 0.125 within 1.5 points of 13, 46, 83 %; a skin vertex at 128 bleeds on about 13 % of texels)

## 4. Painting

- [x] 4.1 `exa_character::blood`: `PaintWound` (body, contact, travel, radius; centre P − D/2), `BLOOD_STREAM` (colour stream, Unorm8x4), `wound_depth`, `wound_alpha`, `paint` (min-only, rounded), `WoundRng` (fixed xorshift, drawn only inside the wound)
- [x] 4.2 `paint_wounds` in `PostUpdate` after transform propagation: targets the visible `SkinnedPart` meshes and other visible meshes whose nearest body part is the head; CPU skinning from the joints and inverse bind poses; a shared mesh copied (white colour stream) on its first change and marked `Bloodied`, later painted in place; a log line per wound (meshes painted and copied, the copies' size)
- [x] 4.3 Tests: `wounds_are_balls_behind_and_smears_ahead`, `the_edge_is_speckled` (0; 128–223 at f = ½; 191–251 at f = ¼), `painting_only_lowers`, `wounds_paint_the_struck_characters_own_meshes` (only the struck body's reached meshes copied; the other body, the hidden part, the held sword and the out-of-reach helmet stay shared and clean; a head wound paints the rigid helmet; a second wound paints the copy in place without a new mesh)

## 5. Debug helper

- [x] 5.1 `DebugWound::parse` (`bone,radius_cm[,dx,dy,dz]`, bone by number or part name), `DebugWounds`, `DebugWoundTarget` (the character view's body and the player's), `paint_debug_wounds` three frames after the body is dressed and bound, at its part's frontmost visible vertex; test `debug_wounds_are_read`
- [x] 5.2 `exarust --debug-wound` (repeatable; a bad value prints the form and exits with code 2)

## 6. Verification

- [x] 6.1 Corpus probe of `races.rdb` and the race models' materials (types, diffuse maps, names), removed; REA and `exe.asm` reading of the painter and the race loader; notes appended to `context/re/wounds-and-blood.md` §8
- [x] 6.2 In game through `gpu_run.sh --gpu 4000` (logs and shots in the scratchpad, no validation error or panic):
  - `--debug-wound chest,20 --debug-wound luarm,12 character`, full view and close up: speckled blood on the chest and the left shoulder, a wet highlight; 6 meshes copied (106 KiB) and 3 (45 KiB)
  - `--debug-wound belly,15,0,-40,0 --debug-wound ruleg,14,0,-30,0 character`: blood running down the belly over glossy bloodied braies and down the thigh to the foot
  - `--opponent --debug-wound belly,18,0,-35,5 --debug-wound luarm,14,0,-25,0 --debug-wound chest,14 play arena`, a fixed camera on the player's front: blood pixels median sRGB (58, 25, 21), darkest (26, 0, 0), brightest (114, 56, 46), red/green 2.29 on skin of (91, 73, 62); the original's (refs/blood): (54, 25, 21), (31, 15, 2), (118, 62, 55), about 2.2 on (81, 68, 58)
  - `--outfit leathjac_a,trouser_a,helm_b`: blood on the helmet and the jacket; `--race 3` (ogre): blood on its skin; a 5 m wound copies 20 meshes (278 KiB) of a body in braies and 145 (1.9 MiB) in the duellist's outfit
  - clean: `model Objects armet___b` (a run-time material now flagged for blood) pixel-identical to main at 9b2bfc7 (0 of 1 344 000 pixels); the character view paused at tick 1: the mean body colour (79.6, 62.4, 50.0) in both builds, the remaining pixel differences from a pose that differs run to run and build to build (two runs of the same build already differ by up to 0.9 %)
- [x] 6.3 Remove the probes from the tracked tree

## 7. Documentation

- [x] 7.1 `docs/RENDERING.md` (Blood; Skin; Surfaces; Not done yet), `docs/DEBUGGING.md` (Wounds), `docs/INDEX.md`, `docs/formats/OTHER.md` (race flags), `docs/PLAN.md` (Phase 5), the READMEs of `exa_render`, `exa_assets`, `exa_character`, `exa_formats`, `exarust`
- [x] 7.2 `openspec validate --all --strict`
- [x] 7.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (287 passed, 3 ignored)

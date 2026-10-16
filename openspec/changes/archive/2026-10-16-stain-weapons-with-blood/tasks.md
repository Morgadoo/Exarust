## 1. Blood drawn by weapons

- [x] 1.1 `exa_combat::wounds`: `BLADE_BLOOD_PER_WOUND` 0.5, `BladeBlood` (on the holder of a weapon model) with `draw` (0.5·R more, clamped to 0..=1)
- [x] 1.2 The blow reactions add the blood inside the bleeding branch, after the wound is queued or dropped; the hit log names the blade and its blood
- [x] 1.3 Tests: `blades_draw_half_of_each_wound` (R 0.457 → 0.2285, 0.6855 after three, 0.914, then 1; 0.0501 → 0.025 05), `blades_draw_blood_from_bleeding_wounds` (0.2285 at the blow; eight cuts with two dropped → 1; none from a skeleton, R 0.045 on a hand, or into a fist)

## 2. Stain pass

- [x] 2.1 `exa_render::stain`: `STAIN_GRADIENT`, `STAIN_FROM` 0.01 (double), `ACROSS` (0.025, −0.5), `ALONG` 0.0125, `STAIN_DEPTH_BIAS` 0.000 005, `stain_shows`, `stain_offset`, `stain_uv`, `stained_length`, `stain_colour`, `StainParams`, `StainMaterial` (multiply blend, no prepass, no shadows), `StainPlugin` in the render plugin
- [x] 2.2 `stain.wesl`: the vertex stage keys the gradient on the mesh's position in the weapon's frame (cm) and nudges the clip depth toward the camera; the fragment stage reads the gradient with the colour textures' LOD bias and fades it to white
- [x] 2.3 Tests: `the_offset_follows_the_blood` (short sword −0.750 625 clean, √a − 1 for 80 cm), `the_stain_creeps_from_the_tip` (√(a·L/80) at the tip, the white row 34.7 cm below it at 0.25 and 69.3 cm at 1, u −0.5 on the centre line, 40 cm period), `faint_stains_are_not_drawn` (f32 0.01 no, 0.010 001 yes), `the_fade_whitens`

## 3. Stained meshes

- [x] 3.1 `exa_character::blood`: `placement`, `stained_meshes` (root's first mesh, L the top of the root's meshes; every part of an assembled weapon, L the top of all), `WeaponStain`, `StainGradient` (loaded as data, clamped along its height, the first time a stainable weapon exists), `stain_weapons` (a pass per stained mesh once the blood shows, its offset following the blood)
- [x] 3.2 Dressing gives a wielded weapon's holder `BladeBlood` and its `WeaponStain`; `weapon_length` shares `placement`
- [x] 3.3 Tests: `weapons_stain_their_root_mesh_or_every_assembled_part`, `blood_over_a_hundredth_stains_the_weapon`, `a_bleeding_cut_stains_the_blade`, `the_debug_weapon_starts_bloody`

## 4. Debug helper

- [x] 4.1 `exarust --weapon-blood a` (`DebugWeaponBlood`, for the player's or the character view's weapon; a bad value prints the form and exits with code 2); parsing tested in `the_debug_weapon_starts_bloody`

## 5. Verification

- [x] 5.1 REA and `exe.asm`: the blood branch's striking-item slot (register, then add 0.5·R from `0x3f000000`), the add routine (clamp to 1.0 and 0.0), the registration (refuses body-part nodes; k1 = L·0.0125, k2 = √(80/L); L = bounding-box centre y + half-extent y), the draw routine (blood > 0.01 as a double; the node's drawn flag; uniforms (0.025, −0.5, 0.0125, o); the node's first sub-object; +0.000 01 on each view's depth row; depth writes off; multiply blend), the GLSL pair, the gradient's loader flags `0x100F0` with the file's `0x20A90000`, the wrap setup (repeat on `0x10000`/`0x20000`/`0x40000`, else clamp) and the linear upload on `0x100`; notes appended to `context/re/wounds-and-blood.md` §10
- [x] 5.2 Corpus (with `EXANIMA_DIR`): `shipped_stain_gradient` (256 × 512, white first rows, red on 14/33/79/87 % of rows 64/128/256/448, flags repeat across only), `shipped_weapons_stain_their_root_mesh` (110 plain weapon models, 7 missing from the pack, 98 assembled; every root a mesh above its grip, tips 9.3 cm–2.03 m; 36 with several meshes, all tools and props)
- [x] 5.3 In game through `gpu_run.sh --gpu 4000` (light runs: `--no-gi --no-supersample --size 960x540`; logs and shots in the scratchpad; no panic or validation error): the character view with `--weapon ub05ssworda --weapon-blood` 0, 0.25 and 1 (clean; the outer part stained, specks at its edge about 35 cm below the tip; stained from the tip to the guard, grip and pommel clean; "1 meshes, tip 60 cm") and with the factory longsword `@sword_lnga1` at 0.45 ("4 meshes, tip 105 cm"; dark red at the tip, specks toward the middle, the lower blade clean); arena1 duels with `--opponent --weapon ub05ssworda --click …` paused at tick 1250: the duellist's blade drew 0.5·R from each bleeding blow on the player (0.073, 0.231, 0.407 after R 0.146, 0.317, 0.351; another run 0.134, 0.221, 0.387) and showed red from the tip to about mid-blade, the player's (0.075 after one R 0.150 cut) specks at the tip; arena1 with the longsword at 0.45 beside the duellist's clean one: stain pixels median sRGB (30, 0, 0), darkest (13, 0, 0), against the retail zoom's (24, 9, 5) and (16, 0, 0), covering about two thirds of the blade from the tip as in the retail shot (the short sword's own textures are missing from the shipped data, so its blade draws near white and its stain bright red)
- [x] 5.4 Remove the probes from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/RENDERING.md` (Blood on weapons), `docs/formats/RFI.md` (wrap flags), `docs/formats/OTHER.md` (Damage), `docs/DEBUGGING.md` (`--weapon-blood`, the blade's log), `docs/PLAN.md` (Phase 5), READMEs of `exa_combat`, `exa_character`, `exa_render`, `exarust`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`

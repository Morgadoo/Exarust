## 1. The jet at the blow

- [x] 1.1 `exa_combat::wounds`: `SPRAY_PER_WOUND` 120, `SPRAY_FULL_FROM` 1.67 (double), `SPRAY_FULL` 200, `DROPLETS_PER_STRENGTH` 0.3 (double), `spray_strength`, `jet_droplets` (halves to even), the `Spray` message (body, bone, contact, swing, droplets)
- [x] 1.2 The blow reactions send one `Spray` per bleeding blow, before the wound is queued, also when it is dropped; the combat plugin registers the message
- [x] 1.3 Tests: `jets_send_droplets_by_the_wound` (R 0.457 → 54.84 → 16; R 1 → 36; R 0.05 → 2; f32 1.67 not above 1.67 → 200.4 → 60; above → 200 → 60; 15 → 4, 5 → 2, 25 → 8), `bleeding_blows_spray_at_the_blow` (one jet of 16 at the blow with the hit's bone, point and velocity, none on the bleeding ticks after; eight cuts with one dropped → eight jets; none from a skeleton or R 0.045)

## 2. Droplets

- [x] 2.1 `exa_character::spray`: the template constants and derived values (`emission_interval` 1.3833, `droplet_life` 62, `droplet_speed` 48/83, `droplet_retention` 0.95^(1/83), `droplet_fall`), `jet_frame`, `Jet` (frame in the part's, last origin and motion, seed 2 × the axis in the part's frame, droplets left), `BloodSprays` (jets, droplets, shared clock, the original's integer generator, 513 slot sizes 12–24 cm, at most 512 live; `start` reuses a finished jet; `tick` ages, drops, steps and emits)
- [x] 2.2 `start_jets` (the struck part by bone among the body's descendants; its frame from the `Ragdoll`'s last tick, else its drawn transform, scale dropped), `tick_sprays` in the fixed step after the body solver, `load_spray_assets` (`BldPrt02` sRGB, clamped on both axes, with the first jet), `draw_sprays` (one world-space mesh spawned with the first droplets, hidden while none fly; `droplet_quad`, `droplet_alpha`, `droplet_uvs`; the camera's forward axis; the fixed step's overstep)
- [x] 2.3 `exa_character::blood`: `PartFronts` (shared part-front search) and `bone_by_name`; `paint_debug_wounds` uses them; `exa_physics` becomes a dependency
- [x] 2.4 Tests: `the_template_in_ticks`, `droplets_fade_in_and_out` (128, 192, 255, 128, 64, 32, 0), `the_jet_frame`, `a_jet_sends_its_droplets_at_forty_a_second` (exactly 16, in 20–50 ticks, gone by 200; 60 in 100–150 ticks), `droplets_fall_at_a_quarter_of_gravity` (0.035 566 the first tick, 66.43 cm in 61 ticks, none sideways), `the_first_droplets_carry_the_jets_seed` (2.0–2.58 cm/tick against the swing; (0, 0, −2) seed on a turned part, z −1.85 to −2.15), `jets_ride_with_the_struck_part`, `finished_jets_make_room`, `slots_and_their_sizes` (513 slots, 12–24 cm, 512 live at most), `a_resting_droplet_hangs_down` (±24 cm sides, tips 39.08 cm, 30 cm at age ½, none seen along its lean), `droplet_looks`, `debug_sprays_parse`, `a_spray_starts_a_jet_on_the_struck_part`

## 3. Material

- [x] 3.1 `exa_render::spray`: `SPRAY_TEXTURE`, `spray_colour`, `SprayMaterial` (multiply blend, culling off, no prepass, no shadows, no OIT), `SprayPlugin` in the render plugin; `spray.wesl` (`mix(1, texel, alpha)` with the colour textures' LOD bias)
- [x] 3.2 Test: `droplets_darken_by_their_alpha`

## 4. Debug helpers

- [x] 4.1 `exarust --debug-spray bone,R[,dx,dy,dz]` (`DebugSprays`, `fire_debug_sprays`: every second of game time from the debug wound targets, at the part front; a bad value prints the form and exits with code 2)
- [x] 4.2 `exarust --pause-after-jet n` (`SimControl::after_jet`; `BloodSprays::started`; the pause names the tick and the jet); test `pause_after_the_first_jet` (no pause before a jet; then at the tick that first sees it + 12)

## 5. Verification

- [x] 5.1 REA and `exe.asm`: the blood branch's two jet calls (200.0f above the double 1.67, else 120.0f × R in single precision; the direction the negated tracker velocity; the struck body), the jet routine (first source with life < 1 or a new one; life `cvtsd2si` of strength × the double 0.3; frame from the direction and (0, 0, 1); relative to the body's matrix; seed 2 × the turned axis), the template bytes and derived values, the particle tick (life −1 a tick without `0x40000`; the ⅔ gate; jets lose a life per droplet; their motion per emission), spawn (`0x80` path), the insertion's slot sizes (`0x2000`), the quad builder (mode 4, camera forward with `0x80`), the colour (no parity) and UV tables, the material payload and `shadeopt_f`'s blend mode 4 branch; notes appended to `context/re/wounds-and-blood.md` §11, the plan's corrections recorded
- [x] 5.2 Corpus (with `EXANIMA_DIR`): `shipped_droplet_texture` (`BldPrt02` 256², BC1, flags `0x20A80000` with no wrap bit; red on 9.2, 13.3, 10.1 and 10.4 % of its four cells, at most 4 red texels on a cell's edge); `exa-tool rfi` (9 mips)
- [x] 5.3 In game through `gpu_run.sh --gpu 4000` (light runs `--no-gi --no-supersample --size 960x540`; shots in the scratchpad; no panic or validation error): the character view with `--debug-spray chest,1` (spatter over the chest and braies, nothing against the black backdrop); arena1 duels with `--opponent --weapon ub05ssworda --click …`: jets logged for every bleeding blow (12, 2, 11, 13, 5, 6, 5 droplets for R 0.346 … 0.149), `--pause-after-jet 12` (the player's head cut R 0.311, 11 droplets: spatter flying off the head's side against the blade's swing over the wall and down the chest) and `--pause-after-jet 35` (a chest cut's 6 droplets fading beside the chest); arena1 with `--debug-spray chest,1,1,0,0 --pause-after-jet 45` (a full jet's cloud in front of the chest, falling)
- [x] 5.4 Remove the probes from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/RENDERING.md` (Blood sprays), `docs/formats/RFI.md` (`BldPrt02`'s flags), `docs/DEBUGGING.md` (`--debug-spray`, `--pause-after-jet`, the jet log), `docs/PLAN.md` (Phase 5), READMEs of `exa_combat`, `exa_character`, `exa_render`, `exarust`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`

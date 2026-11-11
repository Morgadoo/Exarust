## 1. Changes

- [x] 1.1 `material_state`: slot 2 mask `0x400008` as the smoke unit rewrites it, `draws_smoke`; tests `the_five_builtins_match_the_factory_table`, `only_type_two_draws_smoke`
- [x] 1.2 `smoke`: `program_words`, `program_builds`, `smoke_alpha`; tests `the_wraiths_program_words`, `the_program_builds_only_with_its_inputs_declared_and_no_distance_fog`, `the_alpha_follows_the_facing_ratio_and_the_opacity`, `the_smoke_library_validates`
- [x] 1.3 `shadesmk.wesl` library and the `exa.wesl` smoke branch (`flags2::SMOKE`), composed by `the_surface_shader_composes_and_validates` with and without the distance fog
- [x] 1.4 `exa_assets`: type-2 records set `flags2::SMOKE`
- [x] 1.5 `exa-tool corpus materials` and `shader --scene --smoke`

## 2. Checks

- [x] 2.1 fmt, clippy `-D warnings` and tests of `exa_render`, `exa_assets`, `exa_tool`; play build
- [x] 2.2 GPU grid: the wraith in exanima01 before and after, FPS on exanima01, exanima06, arena and arenao1; lockstep traces identical

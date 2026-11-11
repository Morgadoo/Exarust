## 1. Changes

- [x] 1.1 `material_state`: shared colour slot and its binding; tests `the_colour_slot_keeps_the_later_of_diffuse_and_opacity`, `the_builder_binds_the_colour_slot_by_the_diffuse_and_opacity_bits`
- [x] 1.2 start-up tables read by the feature word; test `the_start_up_tables_hold_the_builtins_and_the_default_builder`
- [x] 1.3 `loaded_render_state`, `late_class`; tests `the_loader_derives_the_render_state_word`, `the_late_rewrite_makes_a_full_transparent_record_opaque`
- [x] 1.4 `exa_assets` binds the slot once; a debug trace names records whose diffuse image is dropped
- [x] 1.5 `geometry_node::PartFlags::of_material`; test `a_part_reads_its_material_record`
- [x] 1.6 `exa-tool corpus materials`: plug-in masks, records with both layers and their position

## 2. Checks

- [x] 2.1 fmt, clippy `-D warnings` and tests of `exa_render`, `exa_assets`, `exa_world`, `exa_formats`, `exa_tool`; play build
- [x] 2.2 GPU grid: arenao4 and arenao1 before and after, FPS, lockstep traces identical

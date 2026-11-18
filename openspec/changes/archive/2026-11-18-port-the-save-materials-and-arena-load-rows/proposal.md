## Why

Roadmap task B4, lane `gameplay3`: the port queue's lanes Q05 (items and
equipment), Q06 (saves and the live scene) and Q07 (story flow and arena
career). A save's level section listed every material its meshes use where
the original leaves out what the shared material libraries hold; the arena
load dropped the week's whole tournament board for one event id no template
has and kept an empty rival field empty where the original draws one; an old
converted arena save kept the marked entries of its hub stash; and the
game's first run on a Steam Deck did not write the hand-held defaults.

## Evidence

REA (Exanima.exe 0.9.5): the exporter's shared-library check and material
list, the model reader's material add (a name the manager holds is never
added to the scene's library), the tournament chunk reader (template copy
over the live entry only on a matching id, the field draw when its first
company is 0), the arena player-data reader (the stash compaction after the
record repair, before the list regeneration), and the start-up entry
(the first run's Steam Deck branch: CursorSize 150, GUISize 1,
GILowPrecision 1).

## What changes

- `exa_assets::model::library_material_names`; the save's level export
  (`level_transition::export_level_scene`, `level_save::saved_section`)
  takes the shared names: they leave the material list and the library.
- `arena_tables::event_from_entry` keeps the live board's row under an
  unknown id; `arena_tables::apply` draws an empty field from the career's
  roll.
- `arena_save::compact_converted_stash`, run by `save_live::restore_arena`
  on a converted save.
- `user_settings::first_run_defaults` on a first run on a Steam Deck.

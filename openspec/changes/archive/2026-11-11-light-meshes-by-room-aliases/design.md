# Design

- `RoomLights` gains the live calls: `place` (forget, then the home sector),
  `add_cell` (the cell hook), `forget`, `reaches` and `rooms_of`; the scene's
  own sector is a list of its own.
- `shadow_links::track_lights` rebuilds a light's lists right after it
  floods (`CellLights::cells_of` gives the cells the flood recorded). New
  rooms start new lists and reflood every light.
- `shadow_links::link_casters` runs after `scene_live::run_scene` and reads
  the mesh's sector from `LiveScene::sector_of`; `run_scene` lists the meshes
  it registered or moved (`sector_changes`), and a mesh whose sector differs
  from its `LinkedSector` is relinked in full. The alias test runs after the
  weight and the wall test, so the refusal count is the links it changed.
- The links double as the cube casters, so a light also takes casters only
  from its sectors, as the original's link lists do.

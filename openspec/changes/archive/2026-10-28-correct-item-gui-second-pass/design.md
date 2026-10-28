# Design

The fixes stay in the modules the lane added: `exa_items::item_renderer`,
`exa_items::card`, `exa_game::item_container`, `exa_game::item_panel_draw`
and `exa_game::gui_screens`.

- `project` sums `(x·a + y·b) + (z·c + w)`, the order the packed transform
  helper's two vector adds give; `scene_bounds` takes the kept meshes, the
  seed depth and the box's other half extents, and reads each matrix's
  second row as the original's offsets do.
- `render_plan` takes the alpha global and reports `screen_pass`,
  `downsample` and `halo` separately.
- `CardBuildInput` loses the separate points count (the weapon builder
  derives points from the flags and skill, `weapon_points`) and gains the
  value gate; `BuiltCard::title_colour` is optional.
- `panel_event` takes the use class (double click) and the use section's
  kind (card) separately, and reuses `card_action` for the card.
- `CardWindow` holds a `CardTimer` rather than a second copy of it.
- The second copy of the icon program's arithmetic in `exa_items` is
  removed; `exa_ui::gui_draw::item_icon_fragment` is the one the panel
  draws with.
- The inventory paint offsets the panel's icon layer by the pack panel's
  client origin and clips it there (the entries' cells are the panel's own
  coordinates).

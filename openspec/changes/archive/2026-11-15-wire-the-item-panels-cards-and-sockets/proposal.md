## Why

Roadmap tasks B1 and B4 (wiring lane `wireitems`), the item container
(TGUIItemContainer), the item GUI (SG_ItemGUI) and the procedural
materials (FLX_Materials_Procedural), with findings F264, F265, F266,
F267, F268, F269, F270, F271, F273, F280, F284, F296, F320, F321, F323,
F324 and F325 of the items audits. Ported routines of the item panels had
no live caller (the place pass, the alpha hit test, the hold of a dragged
entry, the take-back of a held entry, the crystal-drain filing), and the
card builder fixed its gates and the pronoun sex.

## Evidence

REA (Exanima.exe), checked against the assembly where constants or order
matter:

- The container open loads the record, then clamps every child by its
  icon's half extents.
- Every centre add of an item panel (the inventory's give-back, the
  character view's drop, the shop and trade windows) is followed by the
  place pass: the cells become centres (half the icon size), the new entry
  takes a random spot in the middle half from the engine's shared
  generator and is pushed out of the others over six passes, and the
  centres become cells again.
- The panel's hit test takes the topmost non-held entry whose cell holds
  the point and whose icon alpha there is above 0, then, with the GUI
  switch on, the topmost with an opaque pixel within a GUI unit.
- The character view's hold: a drag from the view's own pack marks the
  dragged entry held at its cell instead of ending the source's drag; the
  inventory's give-back shows the held entry of a freed item again.
- The drop of item 499 files crystal-drain source kind 2 with the
  container's id, or kind 3 for an owned pack; the socket commit files
  kind 4 with the item's id for the first socket holding the AR crystal;
  an apparel commit calls the player's light switch. The kind-2 drain
  walks the record's container section (0x20000) and drains every stack
  of item 499.
- The card gates are three bytes: rank and points, set by the mode
  switch (on in the arena, off in the story and in a practice entered
  from outside the practice family), and value, raised by the trade
  window's open and lowered by its close (0 in the image).
- The pronoun sex is bit 0 of the player record's profession word.
- A right press with bit 0x2000 on a socket holding a crystal builds the
  crystal's card with its charge; a card request on an item whose usable
  section's first byte is 2 runs the game's object use first.
- The resolve error is a level-5 message titled `Item Error`, text
  `Unable to resolve item: ` and the id in eight hex digits.
- The dot size is the GUI level's large-icon cell.
- `TSuedeA01` names the family `suede`; the factory's per-material table
  marks only material 23 for the sRGB-alpha run-time material.

## What Changes

- `exa_game::gui_screens`: the container popup clamps after the load;
  `GameScreens::drain_sources`.
- `exa_game::panel_live`: the two-pass hit (`entry_at`), the pack move
  write-back (`body_inventory::reposition`), the drop's drain filing, the
  apparel commit refresh.
- `exa_game::item_panel_draw`: `PanelIcons::pixel_hit`, `near_hit`,
  `add_placed` and the item GUI's generator `random`.
- `exa_game::item_container`: `Container::place_at`.
- `exa_game::doll_view`: the hold of a pack item taken into a hand,
  `give_back_displaced`, the held add with its place pass, `dot_side`.
- `exa_game::figure_pick`: the held entry removed by its icon slot.
- `exa_game::socket_drag`: the AR filing, the placed give-back.
- `exa_game::crystal_drain`: `take_filed_sources`, the kind-2 drain.
- `exa_game::shop_panels`, `hub_windows`, `arena_hub_live`: the settle
  and the preview's to-pack adds placed.
- `exa_game::item_card`: `CardContext`, `sync_card_context`, the socket
  crystal's card, the use before the card.
- `exa_game::item_icon_render`: the resolve error report.
- `exa_items::item_gui::slider` takes the dot size.
- `exa_formats::procedural`: suede's family, `alpha_target`;
  `exa_assets::weapon` uses it.

## Out of scope

- The GUI's rescale walk over open forms (the caller of the panel's
  rescale): open game screens keep the unit they opened at.
- The arsenal columns' fill, save, release and scatter (no arena store
  fills them).
- The engine's shared generator: the item GUI keeps its own.
- The paint-job ring, the surface cache and the shademapa mask pass of
  the GPU painter (Exarust paints on the CPU).
- The invalid procedural material id's box.

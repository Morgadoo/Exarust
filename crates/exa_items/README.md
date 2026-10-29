# `exa_items`

Item database and procedural items: weapons assembled from components by factories, armour layers, materials and quality.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: equipping by occupancy (`carry`), an inventory pane of icons (`pane`), the description card's page and coin value, a weapon card's damage rows, an apparel card's coverage and rows, the card icon's place, a shield card's weight, impact, and encumbrance, the arena gate for rank and points, a description phrase picked from the rows in range, the weapon sentence around that phrase, the fixed-id name edit, and the apparel sentence (`card`). Possessions as the character record keeps them (`possession`: two weapon sets, the worn items as the record's id-and-flags pairs, held marks in the carried list, swapping sets, giving, having an item through nested containers, the take-into-hand and take-out rules, loose items, golem cores, the AI's planned arms), the doll's equipment pipeline (`doll`: the attachment plan over the record's two sets, its apply order (the weapon part cleared once when the class word has no weapon bits, the body refreshed when both stowed hands are empty) and stance restart, the split begin/commit swap, the take-out by where the item sits and the scheduled promotion of the first weapon's off hand, the take-in branch order, the shield take, the redress from what the hands hold, the model's flip and lit marks and the three job finalisers), the object-database catalogue and the swap systems (`catalogue`: each item's kind, its container children, the model section's `0x24` bulk byte and the usable section's kind and sort), copies of items (`instance`: condition from the template with the armour variants, price, desirability, protection, spreads and the roll), the palettes, part colours and the apparel roll of layers, materials and variants (`palette`), the refresh of a weapon or apparel copy from its template (`refresh`) and the world-load fill of containers from random tables with grades (`randomise`: the grade routine's factory copy behind the name flag, a row's damage, dirt, spreads and palette-byte write (item body first), the container children from the picks, the world-load walk; none of it is wired to a live object database yet); the inventory window's geometry and callbacks (`inventory_window`), the arena arsenal's columns, scrolling and checked-out items (`arsenal`), the ref-counted item icon cache and its error messages (`icon_cache`), and the item GUI's crystal sockets, coin, rating-dot and slider layouts, drag payloads, popup placement, icon framing and halo (`item_gui`), its two-stage item resolution, socket row events, sprite sheets and card actions, the card's weapon handling figure, pronoun markers and sample plate-armour sentence, and the handling sound (`item_gui`, `card`, `instance`). The item render plan (`item_renderer`) now projects an item's vertices or box corners, frames the camera at 20/30/70, scales the extents to the icon, and either runs the alpha global's screen pass or filters 2× down and haloes the result, with the renderbuffer resize and the streamed-icon wait budget. The item panel's icon program source (`item_gui::ITEM_ICON_SHADER`; its arithmetic is `exa_ui::gui_draw::item_icon_fragment`), the sprite sheets' atlases and cell crops (`item_gui::load_sheet`, not yet read by a running system since nothing draws the cells) and the five control classes are data with tests. The weapon, apparel and shield pages' rows, control ids, gates and refresh values are `card`. The icon scene (`item_icon`) fits the plan's camera to the item's bounds, builds the framing preset's sun direction, bakes a model's parts and surfaces, and paints them on the CPU for a headless App; `exa_game::item_icon_render` renders the same scene with an off-screen Bevy camera and readback. The windows' forms beyond the inventory and loot panels are still open. Injuries beyond damage are still open.

## Scope

- Object database (`objdb.rdb`, `objstrings.rdb`), item instances and inventory/containers.
- Weapon factories (`Factories.rpk`) assembling `Components.rpk` parts; mass/balance derived from geometry.
- Procedural material grades (steel, iron, copper alloy, leather, wood...) and their texture generation.

## The icon manager's tail

`icon_cache` now also carries the manager's construction (`ManagerSetup`:
the atlas's 0x103 blend and 0x100e608 format, forty-unit icons), the
render's apparel size (`apparel_size`: 1024 at texture detail 3 or more,
else 512), the draw, the two pixel tests on the slot's cell, the handle
refresh that repaints a slot's own cell from the caller's record, the
user-item tags (local, world or base) and errors with the record's own id,
and the exact release: a freed slot clears its id, reference count and
cell, keeping its variant and half extents.

The off-screen icon renderer (`item_renderer`) is modelled from its
constructor: an icon-size global starting at 160 pixels, depth and colour
renderbuffers at twice that, the two RGBA image headers, the sun along
(0.46, 0.47, 0.48) in the colour (0.75, 0.75, 0.75, 0.5), and the camera at
(0, 0, 350) with a 40° field of view and a 10 cm near field;
`frame_distance` is the engine's scene fit the render frames the item with.
`item_icon` adds what the running camera needs: that fit in metres on +Z
from the box's centre, the framing presets' sun direction, the model's
baked scene and a CPU rasteriser; the tests pin the fit, the camera, the
plan's pixel extents, the sun and a shipped weapon's non-empty icon.

The Body loot window (`char_loot`) is 244 by 440 with a 228 by 180
pack at the bottom of the client area and the equipment panel above it; a
drag off the body drops a held weapon, or takes the item off the record,
marks the character and rebuilds apparel, and Power Crystal AL (500) runs
the body's "Activate" dialogue. The inventory (`inventory_window`) has its
constructor geometry, the pack's place at the client bottom, bind and close
plans, drag permissions, drop-back rules and position memory; the arsenal
(`arsenal`) its layout (five columns, eight controls on the top row),
close order, key, scroll and release handling, drag hooks and drop rules.
`exa_game::body_inventory` drives the loot window's two panels from a
character's `Possessions` (the carried list through
`Container::load_list`, the worn items and the shields in the hands through
`worn_layout` with `DOLL_SEED`), and `exa_game::panel_live` runs the drag
between open panels; `exa_game::item_icon_render` fills the icon atlas
from off-screen renders. Drawing the other windows and the character view
remains unwired.

The description card is live (`exa_game::item_card`): a right click on a
panel entry runs the panel's own mouse decision (the use class delays the
card by 200 ms or builds it at once), the card is the original's 268 by 260
form scaled with the unit, holds the panel manager's icon until it closes,
prints the page's rows, material/socket gates, rank/points gates and value,
and fades on the popup's squared curve; leaving the card starts the fade.
The catalogue now keeps each record's card facts (display name and
description through `objstrings.rdb`, compact value expanded to copper,
condition, weapon, apparel and shield sections, sockets, material gate,
name id) and the worn burden and shield record. Names resolve as the
original's string resolver does (inherit, own-tier bit, shared library);
an unresolved name gives no card, and the tint, rank and socket bytes come
from the section of the page shown. The card holds an icon slot by its own
acquire key. Closing an inventory or loot panel queues its icon handles
and the manager releases its references and atlas cells; a freed slot is
reused by the next acquire. A body's encumbrance is recomputed from its possessions
after an equipment change (`exa_game::equip::refresh_encumbrance`), using
the same armour-pass sum as the spawn. The container popup's open geometry
is modelled, but no live container popup is drawn yet. The generated quality and
condition sentence is live (`desc_phrase`): its phrase tables and the eight
armour-variant words are read once from the game folder's `Exanima.exe`
(`exa_formats::desc_phrases`, nothing shipped), the catalogue records which
rule a card uses (a weapon page on a factory model, else any apparel body,
else a shield section on a factory model), and the card rewrites the
description through it (the apparel rule also prefixes the variant word to
the title). Without the executable the description is kept as it is.

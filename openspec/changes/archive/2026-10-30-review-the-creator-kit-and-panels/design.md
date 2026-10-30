## Context

The kit tables are 19-byte entries. The instance writer loops over four colour layers reading the entry's last four bytes and writes a marker of five after each; it then writes the entry's ninth and tenth bytes as the item's two dye bytes only when non-zero. The alias writer copies the variant into both of the target's variant words and the entry's eighth to tenth bytes into three setter calls. The kit match compares the four layer bytes in order.

## Decisions

- `KitItem` holds `tint: [u8; 3]` and `layers: [u8; 4]`; the four bytes between them are not read by this unit and are not modelled.
- `KitAppearance` is an enum with an alias and an instance arm, so each route carries only what it writes; `KIT_LAYER_MARKER` names the five.
- `KitRecord::layers` is optional: a record without the colour-layer block never matches by base, as the original's lookup failure returns false.
- `continue_texts` takes the label list and the name list; the row is optional because an empty list leaves it unwritten.
- `skill_page` takes the second panel's existence and whether the skills panel already exists.

## Risks

None for the running game: these rules have no live caller yet apart from the voice pick, whose behaviour is unchanged.

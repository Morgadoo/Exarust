# Proposal

## Why

The character view's drag-over test was ported (`exa_ui::character_view::accepts_drag`)
but nothing called it. The figure took any weapon, shield or apparel: a golem's
sword dropped on the player went into the human's hand. The view's drop also
assumed every race wears apparel. The trade preview read the wrong race word
(the flag word in place of the look word, and the race byte as the id), and it
was right only by its fallback.

Evidence (REA, Exanima.exe): the character view's drag-over slot, its drop slot,
the race record's look, flag and kind words, and the item section lookup.

## What Changes

- `exa_items::catalogue`: `ItemCatalogue::shield_flags`, the shield section's
  flags word (the first word of its body, read like the weapon section's).
- `exa_game::doll_view`: `figure_accepts`, the drag-over test for a figure of a
  given race. `DollItems` now keeps the race table (`DollItems::race`, with the
  original's built-in record for a missing id). The inventory's drop runs the
  test first; a refused drag ends as a cancelled one. The drop's
  "race wears apparel" comes from the race's look word.
- `exa_game::arena_hub_live`: the trade preview's drop runs the same test, and
  its race word is the look word of the race the record's byte names.

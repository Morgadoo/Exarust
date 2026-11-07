## Why

PLAN Phase 4 (items). The item-text gap lane listed the routines of the
item description unit and of the item panel class against the workspace.
The card refresh the timer list runs after a repair's timed write was
ported as row values only and called by nothing, the repair's copy from
the template was not run on the live record, and the apparel condition
routine was written twice (the item copies and the template migration).

## Evidence

REA (Exanima.exe), read in the assembly:

- The timer list (player controls) writes a repair's wear byte while it
  lowers it or arrives, then runs the repair's copy (only for an id with a
  tier tag whose header's parent word resolves): the template's model price
  word, then the apparel condition routine for an apparel body, or the
  template's weapon bytes 0x30–0x3F, the item factory's stat pass (looked up
  by the model name less its first letter) and the weapon condition routine
  with the copy as its own template. Then it calls the card refresh.
- The card refresh finds the open card of that item, reads the name and
  description again, and runs the apparel sentence builder for an apparel
  body, else the weapon sentence builder for a weapon section (no `@` test),
  puts the name in the caption and the description in the text box with the
  text set routine's markup flag clear (the builder's condition blocks and
  pronoun markers are not run), then writes the apparel card's controls
  0x101, 0x102, 0xd03–0xd05, 0x107 (8 above 0xa0) and the value, or the
  weapon card's next striking bars from 0xd03 for each non-zero side byte,
  0x106 and the value.
- The weapon sentence builder picks a crafted name's condition table from
  a callee-saved register's low byte; the card refresh calls it holding
  the weapon section's address there.
- The apparel condition routine stores every rounded product with a byte
  move before the 160 cap; the template migration and the copies used two
  copies of it in Exarust.

## What Changes

- exa_formats: `item_condition` holds the one apparel condition routine
  (`Protection`, `apparel_condition`, `condition_price`, `variant_scales`);
  the template migration calls it.
- exa_items: `instance` re-exports it; the weapon condition pass uses
  `weapon_stats::condition` (the duplicate weapon routine is gone);
  `refresh::repair_copy` runs the repair's copy over whole payloads and
  `copy_weapon` takes the factory stat pass; `card::apply_refresh`;
  `desc_phrase::{SentenceSource::refreshed, describe_refresh}` and
  `card::item_sentence_with` (the crafted register).
- exa_assets: `LiveObjects::set_payload`.
- exa_game: `use_cursor::tick_kits` runs the repair's copy on the live
  record and sends `item_card::CardRefresh`; `item_card::refresh_card`
  refreshes the open card.

## Impact

Only a repair kit's timed write reaches the new code; the exanima01 and
arena duel lockstep traces are unchanged against main.

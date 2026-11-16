## Why

Roadmap tasks B1 and B4 (lane `datastream`): the object database unit's
routines (SG_ObjectDB2) and the content stream (FLX_Content_Stream). Most
of the database's open rows were ported routines with no live caller: the
older layouts' converters ran only for a magic no shipped file has, and
several readers had live duplicates under other names (the operative's
use script, the search's gathering mask and container sizes, the loadout's
weapon points). One duplicate was wrong: the loadout form's equip cost
read a weapon's whole five-bit word where the original reads the object
database's weapon points (one less for a one-handed weapon).

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions where constants, order or
branches matter:

- The oldest body converter calls the count setter with 1.0 over the new
  body's count byte, and draws the paint seed with the run-time library's
  64-bit `Random(0xFFFFFFFF)` (two twister words, the low one first).
- The operative converter reads both class and kind tables with a stride
  of four bytes and any index; past the twelve class and sixteen kind
  entries it reads the image's writable data that follows them.
- The object script run hands the interpreter the database's counted list
  of the operative body (text from `0x3C`, none for a count under 1) and
  the count word at `0x38` as its length.
- The search's gathering asks the database's mask test with the apparel,
  weapon, operative and container tags (`0x35000`); the container add reads
  the item's model through the flagged section lookup (a world id has
  none) and compares its byte `0x1C`.
- The loadout form's cost subtracts and adds the weapon points routine's
  value (the word at `0x42` masked to five bits, one less, not below 0,
  when the flag bits `0x30` are `0x10`); the loadout's own total reads the
  word without that rule.
- The tournament draw grades its prize at level −1, whose arm sets the
  quality spread.

## What changes

- `exa_formats::objdb`: `Store::parse_with_generator` (the paint seeds from
  a run-time library twister); the body converter writes its count byte
  through `Quantised::set_count`; `Operative::script` reads through
  `counted_list`.
- `exa_game::slots::world_object_tier`: a save's world tier is parsed with
  a twister seeded from the clock, as the original's `Randomize`.
- `exa_ai::search::Loot::of_payload`: gated by `Record::has_any` with
  `GATHERED_PARTS`.
- `exa_game::npc_search`: the container sizes come from
  `ObjectDb::flagged_section` over the live database (`model_size`); the
  base sizes from the live base tier.
- `exa_game::loadout_kit`: the equip cost uses
  `exa_items::card::weapon_points` for the weapon and for the one it
  displaces.

## Out of scope

The container instantiate's copy and row condition (the randomisers'
component, which calls the five row-condition readers); the operative
converter's reads past its tables; the packed amount routines (the
company purse is a plain count); the strip's database deletes (the live
outfit generators dress fresh records); the content stream's job and
completion rings (their one poster, the refit request, is the character
generator's) and the arena's drain.

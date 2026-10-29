## Why

The character build's equip pass reads two hand flags. Its listing sets the held item's flip byte (the byte the flip toggle drives, which picks the weapon record's side six bytes on) for `0x2000000`, per item, and the character's half-sword byte for `0x1000000`, written by every weapon-section item. Exarust read them the other way round (`0x1000000` a character flip, `0x2000000` a lit light). A level's living placed characters also hung no off-hand item: their shields and second weapons were never drawn.

## What Changes

- `spawn::HAND_FLIPPED` is `0x2000000`, read per item (`item_flipped`); `spawn::HAND_HALF_SWORD` is `0x1000000`, the last weapon's (`half_sword`, the `HalfSword` marker, which builds weapon records with the hold).
- `offhand::insert_equipped` puts the equip and attach result on a model; the scene bodies and a level's living placed characters use it, so 27 shields and 12 second weapons over the shipped levels are drawn and held. The dead hang nothing; the off-hand torch stays unhung.

Phase 3 (bodies) in docs/PLAN.md.

## Evidence

The equip pass's listing (`and eax, 0x1000000` / `seta` into the character's half-sword byte; `mov byte [item+flip], 1` then the weapon record builder for `0x2000000`), the record builder reading the side six bytes on when the flip byte is set, and `context/re/master-combat.md` (RB11). Real-data test over the 13 shipped levels with characters.

## Out of scope

The inventory's hand plans (`exa_items::possession`, `doll`) keep their own reading of the flags; the off-hand torch entity.

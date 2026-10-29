# Design

Rules: [`docs/formats/SAVES.md`](../../../docs/formats/SAVES.md) (the
arena load's id passes), read from the classification routine's
instructions.

## Decisions

- Section presence is kept on `objdb::Item` as three flags, because the
  existing `weapon`/`shield` fields are parse results and a short section
  still counts in the original.
- `ItemKind::apparel_layer` carries the body's armour type byte, the value
  the original compares; `shelf_of` already applies the 3–6 rule.
- A body shorter than the armour type byte reads as type 0 (clothing); the
  original reads past the section there, which no shipped record does.
- The player-record reset stays `CharacterRecord::reset_loaded_identity`;
  no transition step is added, as the transition routine never calls it.

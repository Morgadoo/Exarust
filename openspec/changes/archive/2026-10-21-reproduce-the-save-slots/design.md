# Design

Extends [docs/formats/SAVES.md](../../../docs/formats/SAVES.md).

## Behaviour

**Package.** Entries are `{u32 offset, u32 size, char[16] name}` after the
magic and count; entry 0 is the sentinel `{0, 0x10, zeroes}`. The gap scan
walks entry pairs `(k, k+1)` from the sentinel, computes
`next.offset − offset − round_up(size)` in wrapping 32-bit arithmetic and
keeps the last `k + 1` whose room is strictly larger than the new size,
else the count. A new section is inserted there; an existing non-last
section whose `next.offset − offset` is below the new size is moved there,
the scan having run with it in place. The written entry's offset is then
`round_up(previous.offset + previous.size)`. A rewrite never shrinks the
file. Opening creates a missing file and resets a file shorter than 9
bytes or with another magic to the fresh header, without truncation.

**Player data.** 52-byte header (shared to byte 36), then the first chunk
as the character, then chunks while more than eight bytes remain. The
placement is `{reference, position, heading, condition, sub-chunks}`; the
reference's bit 30 selects the world character database. Masks and with
each id and dispatch on the result above `0xF`.

**Slots.** The scan lists a number once; the story deletes a slot only for
a signature below its own, the arena never; unreadable slots are left out
and on disk. A load moves the slot to the front, opens the first existing
file in the asked order and copies a loaded checkpoint back. Death (health
widened to double below 0.01) deletes the `.rsg` and drops the slot when
no `.rcp` exists. The transition save runs only when the save has neither
`exanima04` nor the target level.

**Clock.** `saved + (i64(ticks) − i64(base)) / 1000`, truncated toward
zero, kept to 32 bits.

## Model

`PackageImage` keeps the sentinel and the entries with their offsets;
`PlayerData` keeps the header, the placement bytes and the chunks;
`CharacterPlacement` decodes the placement. `Slots` is stateless: every
call rescans the folder, where the original keeps the list in memory.

## Open reverse-engineering questions

- The meaning of the save flag that blocks every save: traced only as a
  gate; answer by REA on its writers.
- The scan's record lookup and the player name: the character databases'
  importer (REA on the character DB component).
- The arena table payloads and the id passes: the arena component (REA).
- The placement word after the heading is named the condition from the
  level exporter's placement writer; a capture of a save made while the
  player is wounded would show other values.

## Deviations

- Exarust lists its own saves without the record lookup, and its fresh
  saves carry no placement (the original would not load them).
- The parser takes the first chunk as the character only when it carries
  the character's id, so placement-less Exarust saves still parse.
- A file with the magic but a directory that does not fit reads as an
  empty package and is left untouched.
- `Slots::create` writes the session at once.

## Docs

`docs/formats/SAVES.md`, `docs/formats/OTHER.md`, `docs/formats/README.md`,
`crates/exa_formats/README.md`, `crates/exa_game/README.md`.

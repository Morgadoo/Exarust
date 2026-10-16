## Context

`exa_formats::characters` stopped at the worn items, which was enough to
dress the placed characters. The master plan (`context/re/master-plan.md`,
F2 and its Critic) makes this change the one that reads the rest, so lane
F's derived stats (M41), encumbrance (M42) and saving (M47), lane C's swing
clock, clashes and damage (M22, M26, M30) and lane B's morphs (M18) only
read fields. `races.rs` is not touched (M40 owns it).

The research notes, with addresses, are git-ignored:
`context/re/master-body.md` §7.1 (record fields), §7.5 (skills and
techniques), §11 (save and load) and the new §17 (this change's reading of
the readers, with corrections), `master-combat.md` §12 (perks),
`roles-ai.md` §1 (record tail, roles, relations, sub-chunks).

The format page this change extends is
[`docs/formats/RFC.md`](../../../docs/formats/RFC.md) (Starts, characters
and placed objects).

## Goals / Non-Goals

**Goals:**
- every field of versions 5–7 read in the original's order, the ones a
  planned change reads kept with their meaning;
- the perk set and the armour technique values from the learnt techniques,
  by the original's rules;
- every shipped record (13 level databases, `actors.rpk`) parsed to its
  end, pinned by corpus tests;
- `exa-tool characters` to print them.

**Non-Goals:**
- deriving or consuming any value (proposal, Out of scope);
- the role database; saving records.

## Decisions

- **One record type for every version.** The legacy reader reads the same
  fields up to the inventory, then roles, relations and powers in older
  shapes. `CharacterRecord` keeps the version; roles of a legacy record are
  the ones its reader adds (with the three replacements), newest first like
  version 7's, without parents (they come from the role database).
- **Bounded by the record's size.** The original reads the tail's
  sub-chunks until the record's size and jumps by each sub-chunk's size;
  the parser does the same and rejects a sub-chunk that runs past the end
  or stray bytes where one would start. Bytes after the record (an actor's
  item database) are not read.
- **Strict counts.** Negative counts, a technique list longer than the six
  a skill stores, a role blob holding more than its entries and a power blob
  shorter than its size word are errors. A skill slot past the sixth is
  skipped: the record has no room for it (the original's next field, the
  inventory, overwrites it; the seventh name, Meditation, has no slot).
- **Learnt techniques clamp at six.** The original reads points / 5000
  technique bytes; past six it would read the next skill's bytes. No
  shipped skill has more than 25 000 points; the parser clamps (documented
  deviation for invalid data only).
- **Perks are Exarust's own set**, not the original's bit word: a `Perk`
  enum with each perk's skill and technique, and a `Perks` set. The
  Manoeuvring tier is the highest of techniques 1–3 learnt (the armour
  builder takes a maximum, not a count) and Deflection technique 4.
- **Pools stay 16-bit shares.** `Pools::shares` divides by 65535 as the
  load does; multiplying by the maximum health is M41's.
- **Saved state as stored.** Entries keep their tag and value word;
  `heal_pool` and `potion_expiry` read their own entry. The original's
  load reads the last entry's value for both; whether to keep that quirk
  is M47's decision (noted in the format page).
- **Names.** The feral level keeps the notes' name (`feral`): its
  documented effects are the undead's weakness; the creator's age is the
  separate `age` field (RB21: no string in the exe names the field). Skill
  and technique names come from the exe's tables; Dual Wield gives no
  perk (RB15: the technique routine sets nothing for it).

## Open questions and how they were answered

| question | answer | how |
|---|---|---|
| Are shipped records all version 7? | Every placed one is; 24 unplaced level records are versions 5 and 6, and 14 of the 21 arena actors are version 6 | corpus |
| What decides a record's end? | its size word; sub-chunks fill it | REA (both readers), corpus (every record exact) |
| Manoeuvring tier: count or highest? | highest of techniques 1–3 learnt | `exe.asm` (armour builder) |
| How many skills? | six slots; the name table has a seventh (Meditation) with no slot and no techniques | REA (reader zeroes six slots; name table) |
| Insight 6 and Concentration perk names (RB15) | Iteration; Sustain, Abstraction, Clarity, Volition | exe name table |
| Dual Wield (RB15) | no perk; learnt as a fifth Close Combat technique by many records | REA (technique routine), corpus |
| Name of the feral level (RB21) | not settled by any string; kept as `feral` | REA string search |
| Pool load | yellow and red = max × share, focus = share; share = stored / 65535 | REA (load) |

## Risks / Trade-offs

- [The perks are data no system reads yet] → each consuming change pins
  its own rule; this change pins the mapping with unit and corpus tests.
- [Strict parsing could drop a record the original would accept] → every
  shipped record parses; the only failures are 26 deleted (empty) entries
  and two stubs without a record tag (38 bytes in `exanima07`, 1302 in
  `exanima07_sw`), which no placement names.

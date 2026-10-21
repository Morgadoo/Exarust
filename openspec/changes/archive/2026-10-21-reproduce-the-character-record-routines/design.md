# Design

Format page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md)
(Starts, characters and placed objects) and
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Object database
items).

## Decisions

- The file record (`CharacterRecord`) stays in `exa_formats`, Bevy-free and
  total: a malformed count, a skill of more than six techniques or a role
  blob that does not hold its entries is an error, where the original would
  read past its buffers. The live record (`CharacterData`) is an
  `exa_character` component that derefs to the record and adds what the
  game keeps only in memory (reference count, staged list, the body value
  the save copies).
- Floating point follows the instructions, not the pseudo-code: the save
  divides and multiplies in single precision and converts with the SSE
  instruction (nearest, ties to even, the indefinite integer for NaN and
  out-of-range values, cut to 16 bits); the load and the spawn multiply the
  stored share by the double nearest 1/65535 first and then by the
  maximum, rounding to `f32` once; the trait bytes use the same
  conversion. `characters::round_even_i64` models the conversion.
- The perks keep their own reading of the learnt techniques; the record's
  technique check (five written slots, first match, its level funded) is a
  separate method, since the arena and the powers tree ask it and the
  perks come from elsewhere.
- The game's quirks are kept where they are observable and bounded: the
  load's last-entry value, the legacy reader's peek past an empty role
  list, the trade's worn loop that skips and reads the old tail, the
  cascade that resumes at the second slot, the 16-bit point arithmetic.
  The cascade's recursion has a depth bound the game lacks.
- The prerequisite table is reproduced as data (six skills of eight
  pairs, laid out flat so that an empty slot reads the previous skill's
  last entry, as the game's indexing does).

## Open questions and how they were answered

- Which byte the equip refusal reads: the section lookup returns the
  section's header, so the byte is 0x24 from the model section's start;
  the shipped object database answers its range (121 items over 50, the
  golem suit and the ogre armour among the apparel).
- Whether the constructor fills focus: its instructions store only the
  yellow and red words; the creator's reset likewise.
- What the voice formatter writes past 99 and for 0 or negative sums: read
  from the formatter and its two constant strings.
- What the role adder does with a role already held: read from it (kept,
  or made explicit); the parents need the role database and stay out.
- Still open: the role weight's conversion, the meaning of the equip lock
  and of the body value the save copies, and the item weight and rank
  fields.

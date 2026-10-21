# Design

## Context

Review fixes to the damage lane; see
[formats/OTHER.md](../../../docs/formats/OTHER.md) (Damage).

## Decisions

- **Per contact**: the struck side's work sits in a labelled block that
  yields the striker's reaction, so the target's borrow ends before the
  attacker's is taken; the striker is then dealt at once. Within one
  contact the original does the striker before the victim's damage call,
  but the two are different bodies, so the order inside a contact does
  not show.
- **Turned parry**: `Guard::Parry` on a body with `TURNS_BLOWS` is the
  original's clash on a body with 0x400: a recoil whatever the size and
  the sound bit, nothing on the parrier.
- **Sound bits**: the swing word is reset at each attack, so 0x1000 and
  0x200 hold for one swing as the original's; the wound bits 0x724
  already include 0x200, so a block after a wound in the same swing
  draws nothing.

## Open questions

None new.

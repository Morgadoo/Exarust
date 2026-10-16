## Context

M40 decoded every race field and M39 every record field; neither derives
anything. The master plan (`context/re/master-plan.md`, F4) makes this
change the one that turns a record and its race into the values the
original's spawn computes, so the support pins (C14), the walk (C18), the
look (M8), the weapon record (M21), the damage rules (M29) and
regeneration (M43) read one place.

In the original the spawn runs, in order: the character builder (it keeps
the record's height within −1…1, age and feral level within 0…1, then
sets the body scale, size, standing height and a first leg reach), the
registration (which ends in the character's init: most energy, reach,
pins, rule bits, perception, voice, maximum health, the sprint blend, the
head setup, yellow = red = maximum), the armour pass, the reach pass and
the load of the saved pools. The research notes, with addresses, are
git-ignored: `context/re/master-body.md` §4.1, §5.1, §7.2–7.4, §11 and the
new §18 (this change's reading, with corrections), `body-q8-q9.md` §8 (the
walking arms), `master-plan.md` F4.

The format page this change extends is
[`docs/formats/RFC.md`](../../../docs/formats/RFC.md) (character records);
the body rules are in [`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md)
(Physical body).

## Goals / Non-Goals

**Goals:**
- every per-character value of the builder and the init that a planned
  change reads, derived in one Bevy-free function with the original's
  order and precision;
- the body consuming what already has a consumer: the most energy, the
  head's gain and factor, the walking arms, the limbs' scale;
- health from the record with the race-byte rules, and the arena's
  unsaved records spawning full;
- unit tests pinned to the notes' numbers and corpus tests over every
  placed character and arena actor.

**Non-Goals:**
- encumbrance (M42), and every consumer that does not exist yet
  (proposal, Out of scope);
- records for the player and the duellist.

## Decisions

- **One derivation in `exa_formats`.** `CharacterStats::derive` takes the
  record, the race (or none, for the original's built-in record) and a
  `Spawn` (story or not; placed dead or not). It is pure and total, like
  M21's weapon record in the same crate, so corpus tests can pin every
  placed character without Bevy. Lengths are metres.
- **The body gets numbers, not rules.** The head gain (1 − f), head factor
  (1 − 0.8·f) and walking arms are computed once at spawn (the original
  computes the first two at init and the third from the record each tick,
  a pure function of f). `exa_physics::BodyStats` carries them with the
  most energy, the scale and the pins' factor; `Ragdoll::with_stats`
  applies them.
- **Last write wins.** The init writes several fields more than once
  (support pins, clip base, perception, voice volume, attack-voice step);
  only the final value is derived: pins race − 0.25·f² (every race-byte
  override is overwritten), clip base the race's, perception the race's or
  1 − f/2, voice volume scale × race factor (the 0 for corpses is
  overwritten too), attack-voice step race − 1 or 1.
- **Leg reach 107.2 − 11.1·h.** The builder writes 108.5 − 11.25·h; the
  spawn calls the builder before the registration that runs the init,
  which writes 107.2 − 11.1·h (RB51 answered).
- **Maximum health** follows the init's two race-byte blocks exactly; with
  the shipped races only the manipulator differs from 0.25 × its factor
  (0.375 against 0.25), but the rules apply to any race byte.
- **Unsaved records spawn full.** The original's (threaded) placement
  reader sets both pools full when a living placement's record stores a
  red pool of 0, before the spawn loads them; the shipped opponents and
  the proctor store 0/0. The rule is in `derive` (`Spawn::dead`); a corpse
  keeps its pools.
- **Rule bits** are a `BodyRules` word with the original's bit values,
  named after their traced readers (0x1 creature hands, 0x2 remedies, 0x4
  fierce for the AI, 0x8 no lasting wounds, 0x10 may rise, 0x20 red
  regeneration, 0x40 steadfast, 0x100 no red regeneration, set by the
  spawn's armour pass for golems and cleared when one wears its kind's
  core, item 500 or 431; every placed golem does).
- **Energy cap.** The drive starts at 1 and recovers 0.005 a tick up to
  the most energy, so a feral body is cut to its most on its first tick, as
  the original's body tick clamps it.
- **Player and duellist** spawn from a new human record (height 0, not
  feral) of the human race, keeping their earlier lack of a natural armour
  layer.
- **Name.** The record's `+0x90` stays `feral` (RB21 open for the name
  only; the rules are the same).

## Open questions

| question | how it is answered |
|---|---|
| RB51: order of the builder's and the init's leg reach | answered: REA/`exe.asm`, the spawn calls the builder, then the registration whose last call is the init's slot |
| RB49: rule bits 1, 4, 0x40 | partly: bit 1 is read by the held-item pass (fist sphere) and the AI, bit 4 only by the AI, 0x40 quarters a blow's daze and drops the swing stall; the AI uses stay untraced |
| RB21: the name of record `+0x90` | open (name only); kept `feral` |
| RB5: who sets state bit 0x1000 and `+0x12f6` | not needed here (C13) |
| Which placement reader the arena uses | inferred: the threaded loader's reader refills, the other does not; no shipped living placement stores an empty red pool, so only the arena's records depend on it |
| RR21: a feral 1.0 zombie standing and walking (loose head, shamble) | retail capture, not run (the game is the user's); Exarust's exanima01 checked in a light run |

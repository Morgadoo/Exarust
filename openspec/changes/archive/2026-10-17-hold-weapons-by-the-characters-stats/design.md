## Context

M21 (`weapon_record`) gave the holding pass a `Holder` (encumbrance,
size, scale, weightless, half-sword) and fed it `Holder::AVERAGE`
everywhere, waiting for characters to have sizes and encumbrance. M41
derived the size and scale at spawn and M42 the encumbrance; both noted
that the holder and the step reach were to be joined on merge
(`docs/formats/OTHER.md`, Weapon record and Encumbrance).

## Decisions

- **The holder comes from the fighter's stats.** Dressing already reads
  the body's `Fighter`; its `stats.holder()` replaces the average holder
  for both the wielded weapon and the fists. A body without a fighter
  (none dresses with weapons today) keeps the average holder.
- **One step reach, in `weapon_record`.** It is the module of a
  character's reaches and takes plain inputs (scale, feral level,
  encumbrance, cm), so `CharacterStats::step_reach` calls it. The
  arithmetic is M41's double-precision one, which the notes give for the
  init; M21's single-precision version differed by an ulp (59.840004
  against 59.84 cm at feral 0.84).
- **Weightless stays 0.** The armour pass computes the gauntlets' flag
  inside the sum; keeping it on the stats is left for equipment changes,
  as no shipped record powers them.

## Open questions

- Whether the original re-runs the holding pass when the encumbrance
  changes mid-fight (equipping): answered with the equip refresh's
  callers in `exe.asm` when inventory lands.

## Risks / Trade-offs

- Tall and armoured characters' weapons now reach further and recover
  slower than before: in the duellist's outfit (0.175) a sword of weight
  50 and balance 80 keeps its blend 7 but its recoil rises from 0.42 to
  0.5075, so its attack cooldown from 426 to 452 ms; the duel's timing
  changes with it, as the original's.

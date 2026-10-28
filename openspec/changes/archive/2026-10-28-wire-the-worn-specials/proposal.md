# Proposal

## Why

docs/PLAN.md, Phase 3 (characters), the character generator paragraph, and
Phase 4 (combat and items): the armour pass's special kinds 3, 5 and 7
were not modelled, and the per-character item-effect list
(`exa_game::item_effects`) was a tested rule that nothing filled, so a
worn healing mask, null helm or spirit necklace did nothing in play.

## What Changes

- The armour pass's special-kind dispatch over a record's worn items, in
  wearing order: the second wind (2), the items that register an effect
  (3 healing, 4 golem suit, 6 gauntlets) with their crystal bytes, the
  mind ward (5) and the focus charm's regeneration (7, the float
  `0x38D28EB4`). The burden keeps the crystal's flag and charge bytes.
- Spawned bodies (the scene bodies, the arena's fighters, a level's placed
  characters) get the effect list their worn items register; the list runs
  once a second of game time, and a healing item heals red health by
  0.004 of the maximum a run, spending a charge.
- A mind ward marks the body mind-masked, so perception sees it hidden,
  and sets the caster's power-refusing bit; a focus charm sets the
  caster's focus regeneration bonus.

Evidence: REA on the armour pass, its three effect registrations, the
healing tick, the effect-list run and its once-a-second gate in the body
tick, the focus tick's bonus and the perception's masked-bit read; a
corpus check that the shipped object database carries kinds 2 to 7.

Out of scope: the golem suit's and gauntlets' drawn light and garment
switch, the player's HUD flash on a healing step, ending running powers
on a later re-equip (bodies get their caster at spawn, before any power
runs), the armour pass's eye offset for helms, writing a spent charge back
to the item database, the per-bone hit-sound word and the body-scale write.

# Proposal

## Why

The clash-sound lane saw that `exa_audio::sounds::hit_set` read a layer's
hit-sound class as if 1 were flesh and 2 cloth, while the object database
stores 0 flesh, 1 cloth, 2 leather, 3 mail, 4 plate, 5 `HitHbr`, 6 golem.
A limb impact on a breastplate (class 4) played the mail set, a mail shirt
the leather set, leather the cloth set and cloth the flesh set, and the
sixth class played plate. The clothing foley also played `FoleyPlH` for
every armour class, where the original plays that class's own loop.

Phase: the audio and combat milestones of docs/PLAN.md; no PLAN.md edit.

Evidence (REA and the disassembly of `Exanima.exe`): the armour builder
copies the item's hit-sound byte into the layer's class word unchanged.
The character class's first setup loads two string tables in the byte's
order: the part impact's (`FlsCol`, `HitClt`, `HitLtr`, `HitChn`,
`HitPlt`, `HitHbr`, `HitGlm`) and the foley's (`FoleyFls`, `FoleyClt`,
`FoleyLtr`, `FoleyChn`, `FoleyPlt`, `FoleyHbr`), plus the flesh default
`bdycol` and the skeleton's `SkelHit`. The part's hit-sound routine sends
class 0 to the flesh or skeleton default and indexes the part table by
any other class; its foley routine sums by the raw class, weighs the sums
by (0, 0, 0.3, 0.6, 1, 1), picks among classes 1 to 5 and plays the foley
table's entry. The damage routine's layer loudness (0.5, 1, …) and the
item-window foley table use the same order.

## What Changes

- `exa_audio::sounds::hit_set`: class 1 `hitclt`, 2 `hitltr`, 3 `hitchn`,
  4 `hitplt`, 5 `hithbr` (new `HIT_HBR`), 6 `hitglm`.
- `exa_audio::sounds::foley_set`: a class loop plays `FOLEY_CLASSES`
  (`foleyclt`, `foleyltr`, `foleychn`, `foleyplt`, `foleyhbr`; no
  `foleyfls` in the pack).
- `exa_combat::sounds`: the factor table's and the class's comments name
  the byte's order (the values were already right by index).
- A debug log for each limb impact's set.
- Tests pin both orders, the worn items' sets and the installed variants.

Unchanged and checked: `damage::MATERIAL_LOUDNESS` and the layer sounds
(0-based), `armour_layers` (copies the byte), `exa_world`'s impact class
(0 to 6, as the original's `> 6` test).

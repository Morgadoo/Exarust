# Design

- **Death's loop.** The death routine walks the character's leg list (the
  lower legs with foot contacts), not the held items. On each leg it sets
  the link at the leg's limit index to strength 1, then gives the leg's
  first child world mask 0 and clears its strike-chain bit. `go_limp`
  already restores the leg limits, so only the invented held-item weld was
  removed. The weld is built disabled and nothing enables it, so the
  removal changes no motion; it also drops a per-tick system.
- **Thin-plate bit.** The leg routine writes mask 4 on each leg's first
  child after the femur factor, outside its limit-link test. It does not
  run while the condition has 0x800, and the collapse's start writes 0.
  Exarust keeps the bit as `Part::skips_thin`; bit 1 stays `no_level`.
- **Hand words.** The original's contact mobility is the part's contact
  word, restored from its mass value. Exarust's limb contact mobility is
  its inverse mass × 2/3, used as the base. The refresh's armed test reads
  the first held item's class bits 0x30; Exarust reads `Hands` (the main
  weapon, or the second once the main one is gone). Each system runs only
  on the tick its trigger changes.
- **Shield words.** The shield's mass value is 0 on an item body, so the
  equip copies its contact mobility into it and into the link mobility.
- **Revive.** Raise Dead and the undead's rise are not wired in Exarust,
  so the body and character halves are tested functions. The limb node
  turn copy has no Exarust counterpart.

## Why

Phase 5 (Combat) in docs/PLAN.md, the combat2 gap lane (round 180): the
combat rules rated 87% overall. The ratings left open the weapon stat
record's walk up the body chain, the holding pass's mass redistribution
result (it was computed and dropped), the armour pass's worn rules on an
equipment change, the damage routine's damage callbacks and the place pass
dropping the foe before the NPC's cast.

## Evidence

REA (Ghidra on `Exanima.exe` 0.9.5), checked against the assembly:

- The record fetch reads the part's own record, else steps to the parent
  and stops with nothing at a part that is its own parent, never reading
  the root's record. Its callers are the damage routine (striking part;
  no record ends the routine with "Unable to retrieve weapon values."),
  the struck-item sound (struck part; its frame and sound clock are that
  part's) and the grip-link set-up (second hand's place).
- Registration sets each part's mass value to its stored mobility over the
  material divisor and gives it to the contact and link mobilities. The
  holding pass's two walks latch that mass value, inflate it by 1.25, sum
  the reciprocals and write `1/(M·(0.5/n + 0.5·r/Σr))` into the link and
  contact mobilities; the pass then divides the top part's contact
  mobility by the holder's size, copies it to the held contact base and
  refreshes the top part's links' shares from the new link mobilities.
- The armour pass (run on every equipment refresh) clears the second wind,
  the ward and focus words and the worn items' item-effect entries, then
  dispatches each worn item's special kind; a golem race (flags `0x3000`)
  gets its red regeneration blocked unless a worn item without an apparel
  section is its core (500 for `0x2000`, else 431 for `0x1000`).
- The damage routine's wound tail calls the struck body's damage callback
  (installed only by Force Shell: the shell ripple at the contact with the
  yellow before its 0.75), then the striker's hook, then for an overhead
  with heavy claws puts both hands' contact mobility back to their mass
  value. Its global scripted override is read but never written: dead in
  0.9.5.
- The NPC combat state computes the footwork's change cost from the attack
  drive and draws the place count before the place loop; the cast sits in
  the first place's block after its contact pass and set-target, and needs
  the foe still set; it clears the attack drive for the places.

## What changes

- `exa_combat::record_chain` ports the walk; the pair stage asks it for the
  striking part and carries the striking and struck parts (`Strike::part`,
  `Struck::Weapon::part`); the clash's struck-item ring uses the struck
  part's frame and sound clock.
- `exa_physics::weapon_mass` runs the redistribution on the registered
  weapon body (`WeaponMass::of_body`, `apply`) from `prop`'s registration;
  grip links take the item's link mobility; `scale_held_contacts` scales
  the weapon's held contact base.
- `exa_game::equip::refresh_worn_specials` re-runs the worn rules on a
  `Possessions` change; `exa_formats::character_stats::golem_rules` holds
  the golem fix with the original's nesting.
- The damage routine writes `impact::ShellHit` in the wound tail and
  `OverheadWounded`, which `exa_character::body_words` answers.
- `exa_ai::combat::search` runs the cast at the first place after its
  contact pass.

## Not done

The struck item's own sound set is still the record's, not the struck
part's collision sound; the shell ripple is placed in the shell's frame,
not the struck part's; the mind ward's end of running powers on a refresh;
the separate motor heading of the NPC body; the factory weapon's icon-only
mode and paint-job queue.

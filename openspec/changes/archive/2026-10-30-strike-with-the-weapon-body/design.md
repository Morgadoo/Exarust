# Design

Extends the combat spec (`openspec/specs/combat/spec.md`, Weapon contacts)
and the motile-part spec (`openspec/specs/motile-part/spec.md`, Held item
body). No `docs/formats/` page changes. The factory recipe and the weapon
section's sound byte are already described in `docs/formats/OTHER.md`.

## The factory weapon's body

- The builder runs after the factory assembles the merged node. It calls
  the factory-weapon shape builder with the node and the fit record, then
  registers the body on the node and sets its sound set from the factory's
  table by the section's sound byte. Exarust's `exa_physics::shape`
  already ports the builder. The record's bytes (1.1, 1, 6, 0, 2, 0) match
  `WeaponFit::SHIPPED`.
- The shape builder sorts the spheres around the box centre when the node's
  parent is of the 0x11 kind, and otherwise around the zero vector (the
  constant is twelve zero bytes). Exarust's merged node is a root with no
  such parent, so it sorts around its origin.
- The body's three mass words take 10 000 over the fitted volume. There is
  no material divisor, unlike a stored node's mobility.
- `@GlmHalbrd A` (GPU log at the equip): 519 spheres from 106 cm below the
  grip to 176 cm above it, mobility 0.653. `@UBM Axe1H A1`: 36 spheres,
  −17 to 52 cm, mobility 4.90.

## The hand attach

- The hand's item-instance builder passes "build a body". The attach that
  follows it (owner, the class bits over the chain, the grip links)
  requires a body and does not test the node's kind. The one kind test, in
  a helper, only touches a sub-body flag.
- So a factory weapon takes Exarust's existing grip path unchanged
  (`PendingProp`, `HeldGrip`, `grip_runtime`). Its striking chain bits come
  from `impact::sync_held_striking`. The stab drive pushes its body rather
  than the hand, which is the original's path.

## Contacts to the damage routine

- The pair stage gives a hit event when a part with the strike bit meets a
  hittable one (a limb, a held weapon, a held shield or torch). The
  striker's handler classifies it. A kind 1 wound needs a living limb of
  another tree, the point on the blade side of the grip, no pending hit, no
  engaged-arm bit, and not the shield hand. The post-step deals the pending
  hit through the damage routine, and the wound shuts the window.
- Exarust's `part::classify` already had kind 1. `strike_held` took kinds 2
  and 3 and left limbs to the sweep. Now it also takes kind 1.
- Within a tick, the original lets a later kind 2 or 3 overwrite a pending
  kind 1 and never lets a kind 1 overwrite anything. Exarust keeps a clash
  or block pending across frames (`held_pending`) until the damage routine
  deals it, so a wound is also refused while one waits. A wound is dealt
  once per target per swing (`Attacking::struck`), as the sweep did.
- Where on the weapon a body contact lies is read in the striking part's
  frame (the item's scene transform can lag the body by a frame; one GPU
  contact read −63 cm from it against 22 cm in the part). The blow's
  speed share follows from it.
- The sweep's 1.5 m/s floor is Exarust's and does not apply to a weapon
  body: a slow touch is a wound of little impact, and it shuts the window
  as the original's does.
- A dead victim takes no wound, as the sweep skipped it [Exarust's: the
  original's dead body leaves the living-limb family, which was not
  traced].

## Open questions

- The world loader's factory weapons (placed in levels): answered by the
  same builder in the original. Exarust's placed-object spawner would need
  the builder moved where exa_world can call it.
- Whether the golems' haft contacts happen in the original too: needs a
  retail capture of a golem stab (the retail camera tools can pin a view of
  `exanima06`).

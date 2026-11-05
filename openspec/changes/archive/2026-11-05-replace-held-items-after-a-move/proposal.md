## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the motilepart gap
lane (round 62). An audit of the class's 120 routines against the crates
found two pieces of its own rules that the shipped game reaches and Exarust
lacked:

- The held-item re-place. Every time the original moves a character to a
  new place (the player's spawn placement on its third behaviour tick, a
  Long Door, a save load, an arena or AI reset), it puts each held weapon,
  shield and torch back at its grip under the hand, at rest, and resets an
  enabled second-hand link's slack to the span it now has. Exarust moved
  only the character's body; the item bodies stayed behind and the grip
  links dragged them across the move.
- The planted sole's side axis. The sole turn in the link iteration keeps
  the side axis it last took from the foot's animated orientation. While
  the body moves and the foot touches without being lifted, the turn
  projects onto that kept axis rather than the current pose's. Exarust
  always used the current pose's axis.

## Evidence

REA (Ghidra listing of `Exanima.exe`): the held-item re-place routine and
its per-weapon helper (the grip placement, the particle sets and matrices
copied, the wake with 1.0, the second-hand link's slack and held length set
to the vector length summed `(x² + z²) + y²`, the shield's and the torch's
re-place), its twelve callers (the player behaviour's third tick, the
displacement power's Long Door, save load, the arena, the AI resets and the
level reset), and the living link iteration's sole block (the moving mode
byte, the lower leg's first contact state masked with the touching and
lifted bits, the kept axis written from the animated matrix's first row).

## What Changes

- `exa_physics::grip`: `ReplaceHeld` asks for the re-place on a body;
  `HeldRest` keeps the item's place under its holding hand, measured when
  its grip links are built.
- `exa_physics::grip_solve::replace_held` (in the grip build set, before the
  solver): every item of the asked body is placed at rest at its grip under
  the hand where the hand now is, and woken; an armed weapon's enabled
  second-hand link takes its new span as slack and held length.
- `exa_game::limp` (the player's ground placement) and
  `exa_game::thaum::cast` (Long Door) ask for it after they move the body.
- `exa_physics::body_feet::turn_sole` keeps the side axis on the leg
  (`Leg::sole_side`) and uses the kept one for a moving body's touching,
  unlifted foot.

## Out of scope

- The other re-place callers (save load, arena placement, AI and level
  resets): Exarust has no matching move of a living holder yet.
- The re-place's stab aim point reset (200 cm ahead of the frame).
- The re-grip in the character creator's preview (no held items there).
- The pins' and foot contacts' scaling by the character's size and body
  scale, the Ground Query's floor bit, the race word behind the golem and
  skeleton hit sounds, the support pins' per-part order in the iteration.

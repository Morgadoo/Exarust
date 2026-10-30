## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart: the review of the
motilepart11 DeepSeek lane. The lane rewrote the living limit-link law and
split the held item's grip links into living and limp laws; checked against
the assembly, both were wrong. The limit law scales the unnormalised gap by
the strength (as the spec already said), and a held item never takes the
living-part bit, so its links always run through the body solver's plain
iteration. The review also found that the earlier "Death weld stiffening"
and "Collapse weld slack" requirements describe writes to the legs' limit
links, not to the held items' soft welds.

## What Changes

- The limit link keeps the spec's law (`strength·d`, capped at 10 cm, the
  squared gap in cm² compared with the stored slack); the lane's change is
  reverted.
- A held item's pivot (hand) links keep the limp link law whether the
  holder lives or not; its world welds (no pivot flag) take the plain link
  law shared with world links (`strength·d`, a `10·strength²` cm move at
  most past `10·strength` cm, or `strength` times the excess past a slack).
- A drop clears only the strike, chain and mark bits (`0x30001`); a held
  torch takes the held and mark bits, the shield's mobility words and
  friction 0.
- The collapse's and death's held-weld writes are removed (the legs' writes
  are `Ragdoll::go_limp` and `Ragdoll::slacken_femur_limits`).

## Evidence

The motilepart pack's assembly in REA: the living iteration's limit branch
(the vector subtraction helper is parent point minus part point, the scale
helper multiplies in place, `comiss`/`jp`/`jbe` against the stored slack and
100.0), the base iteration's pivot and plain branches, the setters of the
living-part bit (only the character's own part list, parts with anchors),
the foot-part list builder (body parts with foot contacts), the death and
collapse loops over that list, the torch attach and drop, and the drop's
recursive `0x30001` clear.

## Out of scope

- What enables the disabled world welds (the parry lane's rules); until
  then the weld law is visible only in tests.
- The original's muscle pull on a living part's slack links (the held item
  never takes it).
- The torch's own node placement (13, −1.5, 6) cm with its (70, 15, −20)°
  turn.

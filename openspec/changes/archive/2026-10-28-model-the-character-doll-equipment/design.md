# Design

The equipment routines are kept as record-side decisions: each returns
what the original would do (steps, flags, record writes) so a later doll
renderer can carry them out without re-deriving them. Where the original
compares scene-node pointers, Exarust takes the facts it needs from the
caller: the item a take-out makes room for (`TakeOut::instead`), where the
removed item sits (`HeldPlace`, with the first-weapon mark the promotion
reads), and the off hand's item and kind.

The original's take-in and shield jobs return "not yet" until the item's
scene node and its part are loaded. Exarust's items are complete when the
job is made, so that wait has no counterpart and is not modelled.

The view's drop and release plans are pure functions of `DropFacts` and
`ReleaseFacts`. One release fact (`keep_doll`) is a view byte whose role is
read from its one use and is marked `[?]`.

The live wiring is deliberately small: after `swap_weapon_sets` leaves a
`PendingHands` plan, `exa_game::equip` rebuilds `Hands` from the record's
out set, keeping the carried torch, which lives outside the record. No
models are spawned; the existing held-item draw follows `Hands`.

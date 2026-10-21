# Design

Format page: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md#topic-memory-at-run-time).

## One role list

The held-role list (id, weight share, explicit flag, parent references,
values) moves to `exa_formats::roles` with `add_held_role`,
`remove_held_role`, `held_role_bytes` and `held_role_word`; the AI's
`Roles` and the script's `Character` wrap it. The two relation models stay
apart: the AI's is the saved relation record, the script's carries the
topic-memory blocks.

## One interpreter

`exa_ai::activity::run_init` builds an interpreter context with one actor
slot and the role as the running role, and an `InitHost` that only
implements the activity slots and the role's variables. A corpus test
checks every shipped init script gives the same activities as the plain
assignments did.

## Questions answered with REA

- Unordered compares: COMISS with a parity skip fails every mood test, so
  NaN takes the neutral band and no hostility code.
- Rounding: Trust, Concern and the narrator duration use CVTSS2SI/CVTSD2SI
  without a mode change (nearest, ties to even).
- Memory filing: the load/store struct holds the block key before the
  entry; reading each caller's stores fixes the near activation, the
  witnessed event and the broadcast walk.
- The event node's 3/2 writes are the actor count; the conversation state
  is a different word, tested only by the greeting gate.
- The role-set validator looks each held role up in the database (and the
  relation roles toward the partner when that record's top flag is set,
  not modelled).
- The zone helpers are a 3D squared distance and a point-to-box squared
  distance.

## Open questions

- The meaning of the relation record's top flag (needed for the second
  half of the role-set check): REA on the relation-record writer.
- The contact value that scales the mood attitude: REA on the AI's contact
  update.

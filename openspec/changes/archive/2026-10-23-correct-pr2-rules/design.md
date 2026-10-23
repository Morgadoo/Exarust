# Design

Rules: [`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (body tick,
feet, footwork, death), [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md)
(burning, remedies), [`docs/PLAYER.md`](../../../docs/PLAYER.md) and
[`docs/THAUMATURGY.md`](../../../docs/THAUMATURGY.md), as the review lanes
corrected them.

## Decisions

- Every requirement PR #2 added or changed was read against the code on
  the review branch. A requirement whose rule the fixes changed is
  MODIFIED with its whole text and scenarios; a requirement that held is
  left alone, even when its wording could be shorter.
- A requirement over 500 characters is split by behaviour: the original
  requirement keeps its first behaviour and the rest become ADDED
  requirements with their own scenarios (Clip clocks, Support, Leg bend,
  Support pin pull), so no behaviour is lost and none is duplicated.
- Bleeding over time is REMOVED, not reworded: the code no longer has it,
  and the burning routine it copied has its own module.
- Where the fixed code keeps an Exarust reading (a hidden duellist casts
  nothing, the sole turn needs a contact state, the shield block stamps
  nothing because Exarust's shield has no body), the requirement says what
  Exarust does and the docs carry the `[inf: …]` label.
- `docs/PLAN.md` keeps the PR's paragraphs that still hold; the ones that
  claim a rule the fixes changed, or a stale "Not wired" list, are
  corrected in place, and one Done paragraph records the review.

## Open reverse-engineering questions

None are answered by this change; it only records what the lanes read.
Left open by the review and named in `docs/PLAN.md`:

- The shock reaction's caller (its zero clip rate is carried on the
  release value only) — REA, the callers of the shock-start routine.
- The original's placed flag and group root for the pair impact's gates
  (Exarust uses the settling count and the same ragdoll) — REA, the pair
  pass's owner reads.
- The world clock's saved base at load — the session load path.
- The player behaviour's game-view and status-bit gates around the hook
  asks — REA, the player behaviour tick.
- The footwork landing's 200 ms slow-turn extension — needs a slow-turn
  field first.

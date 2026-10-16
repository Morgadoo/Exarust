## Context

The original registers a character's bodies on the model's nodes
(`humanbase.rfc`), not on the skeleton's. The motion binder copies each
skeleton body's raw link records to the model node with the same bone
number. Registration converts each stored link: the part's point becomes
weights on the part's rest particles; the parent's point plus the node's
local translation becomes weights on the parent's rest particles, with no
rotation; the slack is kept. Then it appends one more link to every part
whose scene parent carries a body: self point at the origin, parent point at
the node's translation, slack 0, strength 1. Muscle setup replaces each
link's smoothing with (1 − (r² − 25)·0.001)², capped at 0.99, and makes a
lower leg's link with over 10 cm of slack its leg limit at 0.2 strength;
death resets that strength to 1. The research notes, with addresses, are
git-ignored: `context/re/body-links.md` (§0–2, §4.1, §5–6, §9–11),
`body-tick-order.md` (§5.2) and `body-controller-plan.md` (C3, Q3).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body).

## Goals / Non-Goals

**Goals:**
- the origin pivot on every part with a parent;
- parent link points as the original converts them;
- the original's shares, pivot laws (living and limp), slack limits with
  the 80 cm cap, and order;
- link strength and smoothing kept for the next changes.

**Non-Goals:**
- the limp damper inside the slack (C4) and the link muscles (C5);
- body scale on link points; saved corpse poses (S2).

## Decisions

- **Stored points in `PartSetup`.** `PartSetup::links` carries the stored
  points (the parent's still measured from the part's origin) and
  `PartSetup::origin` the part's origin in the parent. `Ragdoll::new` adds
  the origin to each parent point and appends the pivot, so the rule lives
  in `exa_physics` and tests state data as the file does. `exa_game`
  takes the origin from the model's rest translation of the part under its
  parent part (`rest_under(..).translation`, the node's translation when
  the parent is its direct parent, as in every shipped body).
- **Shares from the bone table.** m = 0.175 × `INVERSE_MASS`; the factor
  cancels between two parts and is kept as a named constant. Corrections
  move each particle by share × weight × correction, so Σw = 1 keeps the
  two parts' momenta equal and opposite (the existing momentum test still
  holds).
- **Limp pivots soft now.** The plan puts the soft limp pivot under C4.
  The limp bodies of R5 and R6 are this change's in-game checks, and the
  plain iteration's pivot law is observed, so the limp pivot follows it
  here; C4 keeps the slack damper, the resets and the retention.
- **Orders from the tree.** `Ragdoll::new` computes a pre-order and a
  post-order over the parts' parents (siblings in index order), instead of
  relying on the index order: the living order is children first, the limp
  one parents first, depth first.
- **`go_limp`.** Taking the muscles goes through `Ragdoll::go_limp`, which
  restores the leg limit strengths as the original's death does.
- **Units.** Exarust stores metres; the 0.001 softening, the 25 and 0.001
  of the smoothing and the 0.01 cm pivot threshold are the original's cm
  constants, applied after converting to cm.

## Open questions

| question | how it is answered |
|---|---|
| Which matrix places the parent point (Q3) | Answered by REA on registration and the motion binder: the node's translation only; corpus: \|parent point\| = \|point\| for all 36 links. |
| Whether a non-unit scale or appearance morphs change the model translations | Not traced; Exarust uses the model's translations as loaded. Left for the races and character-surface work. |
| Whether the shipped knee hinge axis matches every clip's knee rotation | Measured in game: living joint gaps stay under 4.5 mm walking and 0 standing; a probe against retail (T0) can confirm. |

## Risks / Trade-offs

- [Hard pivots fight the all-parts pose drive] → measured with a temporary
  probe: walking ≤ 4.3 mm at the feet, standing 0 mm, a shove's fall
  ≤ 24 mm, against 5–170 mm before.
- [Placed corpses stand up on furniture and collapse through it] → one
  exanima01 corpse on a cot keeps a hip 8 cm out of its socket, its thigh
  held under the cot by level contacts. Saved corpse poses (S2) remove the
  cause.
- [Limp bodies without the slack damper] → a dropped body can come to rest
  propped (R6 ends kneeling, forehead on the floor) and some corpses creep
  at 1–4 cm/s without sleeping; C4 and S1 address both.
- [Negative weights push the wrong way] → the original's own quirk, kept.

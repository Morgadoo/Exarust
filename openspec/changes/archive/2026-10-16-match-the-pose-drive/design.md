## Context

The original's limb step, for each living part: inertia and the carry,
then the part's origin measured on its particles is written into the
translation of its animated world matrix, then (with muscles on, the
has-support flag set and a gain above 0) the pose drive pushes each of its
four particles. The research notes, with addresses, are git-ignored:
`context/re/body-drive.md` (§0–2, §8–9, §11–12 and the new §13),
`body-tick-order.md` (§4) and `body-controller-plan.md` (C7, the answers to
Q1–Q3).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body).

## Goals / Non-Goals

**Goals:**
- the original's pose-drive law, targets, gains and gate;
- a fall keeping the tone, the fallen body left to its link muscles, the
  energy cap;
- standing, walking, falling and getting up still working while the
  capsule carries the body.

**Non-Goals:**
- the lift servo and the target pelvis height (C8), the frame anchor (C9),
  the support pins (C14), locomotion by the carry (C18), the original's
  falls and get-ups (C11).

## Decisions

- **Targets.** Each tick's targets stay the body root's world transform
  times the clip pose; the pose drive reads only their rotation, so the
  root's turn times the clip's. The target of particle k is R·r_k + O, r_k
  its rest vertex in the part's space and O the part's origin (the point
  (0, 0, 0) of its space) on its particles after integration, through
  weights kept when the part is built. A rigid translation of a part
  changes nothing; o is a pure orientation error.
- **Law, in cm.** o = target − particle, v = the particle's displacement
  over the last tick as integration found it (kept per particle before
  inertia is applied). Under 1.19e-7 cm² nothing; c = o·v/(s|o|²); c ≥ 1
  (or not a number) nothing; else o·(1 − c), its y × 0.1, times G. c is a
  ratio of lengths, so only the threshold needs the cm helper; the rule is
  written in cm like the others.
- **Gains.** By bone: pelvis (1), feet (4, 7), lumbar (8), thorax (9)
  0.03·K, head (10) 0.01·K. The original finds the spine by walking the
  pelvis's children within 3 cm of the midline and sets the feet in muscle
  setup and the head last; Exarust's bones are the same parts. G scales by
  t²·T·E⁴·(0.5T + 0.5)·S², t the tone (the original copies the root's tone
  to every part), T = tone·s. The gate is the balance's has-support flag;
  a body that does not measure its balance keeps it set and has S = 1
  standing, 0 fallen.
- **Velocity in the carried frame.** The original's v is the particle's
  world velocity, the body's travel included (walking, |c| ≫ 1: no push
  along the motion, an amplified one against it). Exarust's capsule moves
  a standing body by carrying its particles and their history, which is
  not velocity, so v is the velocity in the carried frame. Reading the
  carry as velocity would brake the body against its capsule every tick
  with nothing to balance it (the original's thrust); a falling or lying
  body is not carried, so its v is its world velocity. C18 brings the
  original's case back when the carry push replaces the capsule.
- **Controller hold (Exarust's own).** Gravity is compensated by tone ×
  energy (until C8), so a standing body has no weight; with the world goal
  gone nothing held its height or its place on the capsule: a humanoid
  whose feet the floor pushed up 1 cm floated 30 cm up, and a get-up would
  never rise. While a body stands on its controller (`supported`), the
  pelvis's origin is pulled toward the pelvis target's translation by
  0.168 × tone² × E⁴ (the old pelvis gain, Exarust's) with critical damping
  2√g of its velocity in the carried frame, the same push on all four
  particles. It acts on the pelvis only (about an eighth of the body's
  mass), so the whole body follows like a spring of about 0.02 a tick
  (period ≈ 0.5 s), softer than the old world goals; the spine and head
  are free to sway. C8 (vertical) and C14 (horizontal) remove it.
- **Falls.** `fall()` only stops the carry and the hold; the tone stays.
  Once the body lies, its balance measurement finds no ground within half
  a leg of the pelvis, so S = 0 and the gate is shut; while it is still
  falling S is grounded × balance (under 0.6), as in the original.
  `stand()` resumes both and leaves the tone. The targets are no longer
  cleared or re-measured from the pelvis: the link muscles read only
  relative turns.
- **Removed.** `Drive::before` (the pose's own motion), `Bulk`, the net
  shift and turn removal, the pelvis-aligned fallen targets, `DRIVE`,
  `SUPPORT`, `FALLEN_TONE`. The 25 m/s guard stays.
- **Energy.** Recovery moves out of the drive into the step, so it runs
  every living tick, capped at the most energy (1 for a normal human; the
  original caps at the character's maximum in the body tick).

## Open questions

- The midline lean (`body-drive.md` §2) that the original multiplies
  into the spine's animated rotation is not traced (what sets it); left
  out.
- The unconscious family's vertical-only push needs a knocked-out state
  (C23).
- Whether the original's walking braking quirk shows in retail (|c| ≫ 1
  along the motion) is read from code only; it returns with C18 and can
  then be checked with the retail probe (T0).

## Risks / Trade-offs

- [The controller hold is a world goal on the pelvis] → it is labelled
  Exarust's own in code, spec and docs, acts on the pelvis's position
  only and goes with C8 and C14.
- [A weaker hold on the spine means more sway and falls] → R1–R5 run in
  game; hits now sway the struck body (balance down to about 0.7, the
  pelvis 11–14 cm off) without knocking it down.

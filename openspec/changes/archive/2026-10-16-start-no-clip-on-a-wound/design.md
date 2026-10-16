## Context

How the original bends a character's bones. Each body part is a
tetrahedron of four particles, joined to its parent by a pivot at its
origin (no slack, a ball joint) and by two or three links from the file
(a hand three). A link pairs a point on the part with one on the parent,
each stored as weights on the part's particles, and lets them drift apart
by its slack. With the pivot fixed, a link holds while the angle between
its two points, seen from the joint, is at most 2·asin(slack/2r): a cone
around the parent's point. A zero-slack point off the pivot makes a hinge.
Neither link iteration tests an angle, a hinge axis or a twist, and none
calls a trig function: the only geometric test is a link's squared length
against its squared slack. A living link takes back its excess beyond the
slack in full on each of the 20 passes, whatever the muscle strength; a
limp one is a hard radius beyond the slack and a damper inside it. The
only weights are the points' four barycentric weights, each part's
mobility (0.175 × a bone table; the share m/(m + m_parent) only decides
which side of a joint gives way) and the link strength (1, the knee's
limit link 0.2). Explicit angle clamps exist only in the animated pose
(leg IK, head look, foot pitch, posture, daze), never for an arm, and the
clips are authored inside the link ranges.

What the original does to a struck character. The damage routine starts a
clip only through the recoil routine, and every such call passes the
striker. The struck character takes the daze, a knockback step when the
impact J exceeds 0.5, pain voices, the Force Strikes push and the clinch
release (which restarts its stance only if it was clinching). The daze, on
a combat-ready character, also calls the swing stall: if the character is
swinging (swing strength P above 1.19e-7) it computes
r = 4·D·f − (1 − D)·min(0.005·reach, 1)·w, with D the daze (at most 0.9),
f a factor the damage routine passes (0.25/max health, so 1 for a human,
in most blows), reach the character's attack reach and w its weapon
record's recoil factor; when r ≥ 0.001 it marks the swing stalled with
rate r and a deadline of min(round(80·P/r) ms, the swing's expected time).
Each tick the swing routine then lowers P by r, scales the swing's clock
by P, and at the deadline restarts the stance with a blend of twice a
weapon record duration. A lethal wound plays Fall Backward and collapses
the character. Rebalance, the Defence stance and falls come from balance.
Head Pain is written only by a game-script handler (the player recalling
a memory); the Defence-stance reaction with a step is called only by
electricity, tendril and energy effects. The research notes, with
addresses, are git-ignored: `context/re/joint-limits.md` (§0–§6 and the
synthesis S.1–S.6), `combat-damage.md`, `body-links.md` §5–§6.

The format pages this change extends are
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Damage) and
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body,
Motion set).

## Goals / Non-Goals

**Goals:**
- no reaction clip on a struck fighter that stays up;
- a struck attacker's swing ends and it is back in its stance;
- the shipped hinge ranges pinned by a test of the installed data.

**Non-Goals:**
- the swing stall's slowing and its deadline (C21, with the swing
  strength, the swing clock and the weapon's recoil factor);
- the knockback step (C16), staggers from balance (C11), a lethal wound's
  fall and collapse (C23), clip blending (C10), mirrored-part repair,
  contacts between bodies (C17).

## Decisions

- **No clip on a wound.** `react_to_hits` loses the branch that picked
  Head Pain (bone ≥ 9) or Rebalance and played it once. The `Animator`
  of a struck fighter that is not attacking is left alone, whatever it
  plays (a stance, a get-up).
- **A struck attacker returns to its stance at once.** Exarust has no
  swing strength, so the stall cannot be ported. For the dazes Exarust's
  duels deal (0.17–0.31) r is a few tenths, so in the original the swing
  stops within a few ticks and the stance returns within a few hundred
  milliseconds [inferred: reach and the weapon's factor estimated]. The
  stand-in removes `Attacking` (so `detect_hits` no longer tests its
  blade) and loops the stance its attack would have looped (the attack's
  follow-up, else the moveset's idle), without a blend until C10. A blow
  too light to stall (r < 0.001, a daze under about a tenth) also stops
  the attack here.
- **A killing blow also ends the attack**, so a dead fighter's blade
  deals nothing more while its body goes limp.
- **The range test measures turns from the clip's zero turn** about the
  axis through the hinge's tightest link (the elbow's 2.61 cm link, the
  knee's zero-slack link), in 0.1° steps, keeping every slack link within
  its slack. That is the frame the clips and the link muscles use. The
  research notes' other figures (elbow 4.1–128° between the joint lines,
  knee 11–129° and 11.6–131.8°) measure between bone lines or with
  coarser scans; the widths agree within 2°.

## Open questions

| question | how it is answered |
|---|---|
| Does a wound start a clip on the struck character? | REA: no; the damage routine's six clip starts all pass the striker, through the recoil routine (`joint-limits.md` S.5). |
| Where does Head Pain come from? | REA: an immediate scan of motion-field writes finds only the game-script handler of the memory event; a register-written id cannot be excluded, but no such writer is on the damage path (S.5). |
| What happens to a struck attacker? | REA: the daze calls the swing stall; constants 4, 0.005, 0.001 and 80 read from the image; the swing routine's stall block drains P and restarts the stance at the deadline (S.5, corrected in this change's research). |
| What is the stall's factor f? | Disassembly: the damage routine passes the daze's second factor, 0.25/max health (halved or squared in some states), so 1 for a human in most blows. |
| Do the links alone bound a joint? | REA: neither link iteration has an angle test or calls a trig function (§0, §2); the test of the installed links gives the ranges. |
| Does Exarust let an arm fold past its range? | In game, with a probe of each elbow's flexion and every arm link's stretch (tasks 3.x). |

## Risks / Trade-offs

- [A struck attacker freezes into its stance without a blend] → the arm
  snaps for a few ticks, as every clip switch does until C10.
- [Light blows stop an attack the original would let through (r < 0.001)]
  → blows with a daze under about a tenth; C21 brings the rule.
- [Fewer reactions make blows look weaker] → the struck part is still
  shoved and the body dazed (its muscles weaken by √energy), and balance
  still knocks it over; the original's knockback step comes with C16.

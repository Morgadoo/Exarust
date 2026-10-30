# Design

Extends the combat spec (`openspec/specs/combat/spec.md`). No
`docs/formats/` page changes: the weapon record's W7 (balance), W22 and W23
are already described in `docs/formats/` and `exa_formats::weapon_record`.

## What was traced first: reach and standoff

- The equip refresh clears the character's reach word and keeps the
  longest held record's W23. The holding pass writes W23 = 60 × the
  character's size + W22. The size is the one held weapons are placed by,
  1.4 for a golem guard; its body scale of 1.2 does not enter. With the
  golem halberd (W21 175.9 cm, striking part 0.705–0.885, W22 143.0 cm) the
  reach is 227 cm. Exarust logged 227 cm at the equip.
- The position score's strike ideal is the observer's reach plus
  10 × (the other's size − 1), plus 50 cm against a large body. Its close
  term is the other's reach + 150 cm. Both match `exa_ai::place`.
- The attack choice's size and scale gates: every 1.3 and 1.4 test in the
  combat state reads the body scale (a golem's 1.2 is "small", so it may
  stab), and the guard's lean reads the size. Both match.
- GPU (debug build): the golems started stabs from 196 to 260 cm (place
  distance). Stabs that touched within the last 14 cm dealt damage. Those
  that touched at 106–155 cm hit the belly or a forearm while the point
  carried past the body, so the line of the weapon missed the target's
  centre. That is a steering problem, not a standoff problem.

## The stab aim (the AI's behaviour slot)

- At the attack's start the executor sets the stab aim's yaw to the yaw of
  the attack's aim and its pitch to 0.
- Each behaviour tick while the motion is Stab:
  - v = target point − anchor. The anchor is (−20, 10, 0) cm in the
    character frame, whose origin is the standing position at pelvis
    height.
  - If the distance across the ground is at least 50 cm, v is normalised
    with the engine's approximate normalise; below 100 cm its height is
    first replaced by (10000 − h²)·0.0001. Otherwise v is the last aim.
  - The yaw turns toward yaw(v) by at most 0.0314 rad. The pitch turns
    toward −asin(v.y), raised to −0.6 at most, by at most 0.0314 rad. The
    yaw is then kept within 45° of the frame's forward.
  - The aim is (cos p·sin y, −sin p, cos p·cos y), and the aim point is the
    anchor plus 250 cm of the aim.
- The target point is the AI's target position. Exarust uses the combat
  state's estimate of the foe's chest. [inferred: the slot calls the
  behaviour's target-point method; for the duellist that is its estimate]

## The stab drive (the body tick)

- Runs in the attack-arm block, for motion Stab on a body whose race lacks
  flag 8 (the quadrupeds), once the clock is past 0.1, with a primary
  weapon and without the cleave bit (a blow landed).
- dir = the normalised direction from the anchor to the aim point.
- s = S((c − 0.1)·1.65) and g = 4·(1 − c)²·(3 − 2t)·t with t = c − 0.4,
  each floored at 0, where S(u) = (3 − 2u)·u².
- The weapon point (0.9 of the first held item's length up from its grip):
  push = (its offset to the line)·6·c·s, plus, past 0.4,
  dir·48·g·(W7 + 0.8).
- The left hand's centre: push = (its offset to the line)·c²·s, plus, past
  0.4, dir·12·g·(W7 + 0.8)·(1 − c)².
- Each push is the engine's push at a point, scale 1.

## Decisions

- Exarust's factory weapons have no item body. They stay fixed to the grip
  hand's bone, so the weapon point's push is applied to the gripping hand
  at its centre, as a push at a point (its share measured against how far
  the centre moved this tick). [Exarust's: the original pushes its own
  item body, and the grip links pull the hand.] A plain weapon with a
  rigid root takes the push on its own body, as the original does.
- A variant that turned the gripping hand about the grip, so that the
  weapon point moved onto the line, was tried and dropped. The golems
  steered no better with it (7 of 17 stabs at the point, against 6 of 14),
  and a human's spear flailed: its axis swung 75–85° off the line, and
  stabs met the target 21–53 cm along. The trace log (the weapon point's
  distance from the line and its axis's angle to it) stays at trace level.
- The aim lives on the duellist and is copied to a `StabAim` component on
  the fighter, which the swing stage reads. A fighter without an aim point
  (the player for now) runs no drive.

## Open questions

- The player behaviour's stab-aim slot (the cursor's aim point). REA:
  decompile the player behaviour's slot at the same table entry.
- Whether the AI's target-point method returns the estimate or the foe's
  chest. REA: resolve the duellist class's method at that table entry.
- A capture of a retail golem's stab, to compare where the halberd lands.

## Verification

- Unit tests with the original's numbers: the turn rate, the 45° and 0.6
  rad limits, the 50 and 100 cm cases, and the drive's ramps and gains.
- GPU, `exanima06` with W held for frames 400–560, counting stab hits on
  the player that land within the last 14 cm of the halberd (the only
  ones that deal damage):
  - Before the change, 5 of 18 over six runs. The rest hit at 45–155 cm
    along, most of them at 104–138 cm, with impact 0.
  - With the change, 6 of 19 over five runs (impact 0.20–0.55). The stabs
    that landed at the point started 196–253 cm away (one run had 3 of 3).
    The others started 188–234 cm away and met the player 71–138 cm
    along.
  - For the golems the change stays within the run-to-run variance (one
    run had 0 of 7 before, another 3 of 3 after): 28 % at the point before,
    32 % after.
  - What remains: the golems attack from 188–230 cm, inside the 216–250 cm
    their halberd spans in guard (176 cm plus the hand's reach ahead of the
    chest). Exarust's factory weapon is fixed to its hand's bone with no
    body of its own, so it passes through the target. The first contact
    after the window opens is then on the haft. The original's weapon body
    collides with the target's parts. Next: give factory weapons their
    bodies (the sphere extractor), or take strikes from the weapon body's
    contacts.
- GPU, `exanima01 --opponent`: with a sword the duel lands blows, including
  a stab at the point. With `--weapon "@UBM Spetum A1"` the spear's stab
  hits move from 74–77 cm along (all with impact 0, before) to 95–109 cm
  along a 108.5 cm spear. Nine of eleven deal damage (impact 0.01–0.59),
  and the spear's axis is a median 22.6° off the aim line.

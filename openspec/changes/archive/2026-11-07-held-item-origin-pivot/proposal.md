## Why

The owner reported that characters do not hold their weapons well. In combat
stance the player's sword floated away from the open hand: after
`--press Tab@120-125,LMB@200-240` and the attack's release, it pointed up
and away from the palm. Round 84 did the same, so this deviation predates
the dress-first regression fixed in round 89.

## Evidence

A per-tick grip trace (`exa_physics::grip_trace` at debug level) of the
player's sword through entering combat, the attack and its release:
- The grip links stay enabled and tight, with spans of 0–0.4 cm. The mode
  stays the one-hander's (no callback). No parry release or loosening
  happens in this scene.
- The item's root nevertheless moves from 13.5 cm off the hand part's
  origin to 25 cm, while its up axis turns. The grip table's one-hander
  ties only two points, (0, 20, 0) and (15, 0, 0) cm. Two pivots leave the
  rigid item free to turn about the line through them, and the stance and
  attack clips swing it about that line.

REA, the attach (equip, shield, torch):
each calls the register-under-parent routine. That routine makes the hand (or forearm) the item's parent
body, resets the item's link count, and adds the part's origin pivot: from the item's own origin to its node's translation in
the parent's frame, slack 0, strength 1, flags `0x410000`. The grip links are appended after it: the second hand and arm, the stab
link, then the table's points, with the main grip index the last. The
re-grip rewrites that first link as the origin pivot again.
The polearm callback checks that its arm index is at least 1, since index
0 is taken. Exarust built only the grip links, so its held items had one
free turn.

## What Changes

- exa_physics: `grip::origin_pivot` and `GripLinks::with_origin_pivot`.
  `build_grips` puts the origin pivot first for a weapon, a torch (to the
  holding hand) and a shield (to its forearm). The grip trace debug tool
  is added.
- Tests: the origin pivot brings back a turn that two points alone hold.
  The link layouts in the builder tests are updated.

## Out of scope

- A one-hander's grip mode word stays unset (−1) in Exarust. It has no
  callback, so nothing reads it.
- The placement's axis convention, which the master notes mark inferred,
  has not been checked against a retail capture.

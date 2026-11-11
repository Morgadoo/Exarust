## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the motilepart4
gap lane (round 177). The round-165 rating left four reachable TMotilePart
items: add-to-world's pin and contact re-scale by the character size, the
muscle setup's gate on the link smoothing, the living step's placed and
frozen branches, and the player's combat target being picked a tick after
the attack starters aim.

## Evidence

REA (Ghidra pseudo-code and the listing of `Exanima.exe` 0.9.5):

- The motile part's add-to-world first runs the rigid body's own. Then it
  re-scales every pin record: the point times the owning character's size
  word (SSE `MULPS`, in place), its weights and world point taken again,
  and its horizontal radius² `z² + x²` stored. Every level contact is
  scaled the same way by the part's own size word. It sets the class bits
  and runs the muscle setup.
- The same loops are inlined in add-to-world and in the separate re-scale
  routine, so every motile part does it each time it joins the world, for
  every race. Round 99's reading saw only the separate routine's two
  callers (the creator preview and the ghouls' init) and concluded that
  human pins are never re-scaled. That reading is corrected here.
- The ghouls' init halves the size word around the separate routine on
  the thorax, so a ghoul's chest pins end at the size times half the size.
- The muscle setup's first-contact move puts a leg's contact 0 at its foot
  child's origin, after add-to-world. So the contact re-scale reaches only
  a leg without a foot child, and contacts after the first. Exarust keeps
  neither.
- The muscle setup smooths a link only when the link word has `0x10000`.
  The skeleton's registration sets that bit on every link of a part with a
  parent, so every muscled link is smoothed. A part with no bone index (a
  plain part) skips the whole setup.
- The living step:
  - A part with no bone index takes the rigid body's step: the retention
    of its own speed band and gravity, with no carry and no pose drive.
  - A dead character (condition `0xA00`) takes the rigid body's step too.
    Exarust drops the body's drive at death, so its step is already the
    limp one.
  - The frozen branch (state bit `0x40`) copies the oldest particle set
    into both newer ones. A living root also remembers its anchors and
    keeps its tree awake at 2. No character part gets that bit in 0.9.5:
    only a body on a walkable-platform node or an item the cursor holds
    does.
- The player's behaviour tick runs its auto-target (the visible character
  within 500 cm with the best `(500 − d)(cos + 0.5)`) right before its
  attack executor, which calls the attack starters. So a starter aims at
  the pick of its own tick. Exarust picked it with the stance, after the
  attack request, and the starters read the previous tick's pick.

## What Changes

- `exa_physics::body::PartSetup::scale_pins`: the pins times the
  character's size. `exa_game::limp::build_bodies` calls it for every body
  it builds, and logs a size other than 1.
- `Ragdoll::scale_chest_pins` (the ghouls) now acts on pins add-to-world
  has already scaled.
- `Ragdoll::new`: the setup smoothing is gated on the body bit.
- `Ragdoll::integrate` and `drive_parts`: a plain part of a living body
  takes the rigid step (the limp retention and gravity, no carry) and no
  pose drive.
- `exa_game::player::pick_player_target` writes the player's
  `CombatTarget` before `exa_combat::parry::aim_foe_parries`. The
  behaviour's own stored target, which its steering reads before the pick,
  is still taken with the stance (`player_auto_target`,
  `target_candidates`).

Not in:

- The contact re-scale: Exarust keeps only a leg's first contact, which
  the muscle setup moves.
- The living frozen branch: unreachable in 0.9.5.
- The link iteration's remaining partial.

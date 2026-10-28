# Proposal

## Why

docs/PLAN.md, Phase 3 (bodies: skeleton, physics, control), the player's
controls: the playerctl3 lane (SG_PlayerControls unit routines) made the
item cursor live (lift, turn, placement, release), added the GUI drag claim
and drop queue, the item use rules, the wheel wrap, the corpse scripts on a
double click and the pad look's take-over. Its review checked each rule
against the executable's instructions and found several that differed, two
of them in the running game: the mouse spun a lifted item, and every lift
snapped back on release.

## What Changes

- Item cursor turn: driven by the pad's raw move stick (squared length over
  0.1 in double, NaN reads the keys) or the move keys (Forward over Back,
  Left over Right), the stick's forward part about the camera axis and its
  sideways part about the up axis; the unit speed is the frame time in ms
  times 0.0025 in double. The mouse does not turn the item.
- Placement: the hit's height is compared with the −100000 cm limit, the
  held item is left out of the pick, and the item is placed whether or not
  its box fits (the box test only answers).
- Release: a plain release leaves the item where the cursor put it; only a
  cancelled lift is put back (a GUI slot takes it, the world hover keeps
  it).
- The lift holds the item with a plain grab; the invented slack dead zone
  in the grab (0.003 is the door drag's untraced link argument) is gone.
- Frame orthonormalisation normalises the second axis first.
- The use dispatcher's bit-2 target is a world item.

## Evidence

The item cursor's turn, placement, per-frame, release, cancel and put-back
routines, the lift branch of the left-press dispatcher, the deferred
restore task, the pad's move-stick store, the engine's squared-length,
projection and cross helpers, and the use dispatcher, read from the
disassembly.

## Out of scope

The placement's mesh sweep to first contact, the item GUI's slots (no hover
word is set), the item-record spawner, live door, wheel and cloth bodies,
the right-press use cursor, and the game-mode registration.

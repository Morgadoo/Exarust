# Proposal

## Why

Phase 7 (game modes and content: thaumaturgy and its effects) of
docs/PLAN.md. The sweep-thaum-b lane closed small gaps across sixteen
thaumaturgy and effect components (the domain wheel, the force shell,
the rain, the force dome, the Energy unit, the fire presets, the force
bubble, the Long Door, the cuboid, barrier and tendril emissives, the
shock, the vanish class and effect, the barricade and the blast
geometry). Its review read every added rule against the original's
instructions and found several read wrongly or only half ported.

Evidence: the rain frame (its turn builder, the per-entry scale by the
square root of the camera field times the single 1/70, the box-centre
translation, the composite product and the point-into-frame helper's
sum order), the fire tiers' builders (the bed's `or 0x8000` alone in the
medium tier, `0x8000 | 0x1000` in the medium-small one), the Long Door's
grid query (the literal 200.0 compared with dz² + dx² in single
precision, the path's last point, the RSQRTSS normalise), the shock
burst's record words laid against the already pinned hand-arc and weapon
records, the vanish phase table's stored entries and callbacks with the
frame callback's unsigned end compare, the Barrier emissive's draw (one
multiplication by the manager's light scale, `bMat` the inverse of the
cap node's frame), and the blast geometry's frame (pass 0's upload
before its viewport, no draw with a single pass), all read in REA and
the objdump listing.

## What Changes

- `exa_world::fire`: the medium tier's bed takes the `0x8000` object
  bits; only the medium-small bed takes `0x9000`.
- `exa_game::thaum::power::displacement`: the Long Door's grid answer is
  usable past 200 cm² of floor distance (not 200 cm), a path under two
  points is impossible, the arrival facing uses the engine's approximate
  normalise, and the AI's start sound plays when the grid answers.
- `exa_game::thaum::effect_light`: the force Barrier's and Bubble's GI
  colour takes the manager's light scale once; the Barrier's frame is its
  cap node's (its facing, 75 cm ahead).
- `exa_game::thaum::arcs::shock_burst`: growth 0.25, lift 0.5, light 1
  and shock strength 0.05, so the live burst shocks.
- `exa_game::effects::vanish`: the table's third entry reveals (show call
  0x20); the table advances only past an unsigned end, and the phased
  body's hook reads the −1 entry as the end.
- `exa_game::effects::rain`: the passes carry `ObjMat` (the scaled turn at
  the box centre times each sub-frame's composite) and `EyeVec`, over the
  shared engine turn and the view's sub-frames.
- `exa_game::effects::effect_geometry::force_uploads`: pass 0 draws in its
  own viewport; a single pass draws nothing on that path.
- `exa_game::thaum::energy`: the second `ShockWeapons` struct is replaced
  by `armed_weapon_arcs`; the danger sight resource is inserted so a
  standing barricade's registration runs.

## Out of scope

The rain's renderer node, the shock burst's aim at the controller's
target, the AI Long Door's live grid query, live owners for the cuboid
and tendril lights, an AI reader for the barricade's sight entry, the
vanish phase table's live driver, the bubble's own draw material and the
force shell's merged index buffer. The overlapping GI and hazard work of
the parallel sweep-thaum-a lane is reconciled at integration.

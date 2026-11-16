## Why

Roadmap task B4 (lane `tendrils`): the electric arcs' tendril emitter and
the displacement wisps' tendril effect. The ledger left the emitter's tick
and frame partial (resolved from block lists, not the assembly), its small
node slots (attach, add to world, remove from world, the node update under
a local-frame parent) with no caller, its follow slot uncalled, and the
wisps' tick and strip with two differences recorded without a decision.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly block by block:

- The simulation sector runs an emitter's level test only when its word
  has 0x10000 and its bodies' test only with 0x20000; the arc record's
  bits 16–19 go into that word. The Shock's burst record (0x30310) asks
  for both; the hand (0x201), weapon (0x301), mace hum and spark arcs for
  neither; the level device for the bodies' test only.
- The level and body tests and the bounds take the points as offsets from
  the hand part's place (the emitter's hand, its part position), not
  through the node; the frame draws them through the node's matrix with
  the hand's node offset point added, takes the camera into the node's
  frame, places the hum and the voxel light at the node's mean and the
  light points in the node's frame.
- The electricity node build attaches the emitter (slot 17); the sector's
  registration enters it into the world (slot 29: the in-world bit, the
  mean at the node's box centre, the local-frame bit from the parent, the
  named hum started looping at the node at gain 0, pitch 1); the node's
  removal takes it out (slot 20: the light points' emissive freed, the hum
  stopped); the per-draw placement of a node with a child runs slot 28,
  which copies a local-frame parent's matrix and carries the node's offset
  through it. The weapon arcs' node hangs under the weapon body, the mace's
  under the item at 53 cm, the spark's under the struck part.
- No call site of the 28 `+0x138` virtual calls dispatches to the
  emitter's slot 27 (follow): their receivers are GUI controls and skins,
  TMotileAI, TPathFinder (the sector's and the world's pathfinder) and FPC
  classes. The level device sets its follow target once at creation.
- The records share one buffer of (segments + 1)·count points, a segment
  count apart; the tick's scroll, width, life fade, share, chord test, lift
  test and jitter step are in double; the frame's fill, half segment and
  first texture step too; several sums run (x+y)+z and several clamps keep
  an unordered value.
- The wisps' blow creator adds its effect to the scene, so its geometry
  setup and add-to-world run as a body set's do. The fade-in branch (with
  its write behind a head of 0) needs the state below 1 at a tick; only the
  setup leaves it there and the add-to-world, which precedes every tick,
  sets 1. The strip reads past a lit last point into its scratch.

## What changes

- The record's test bits gate the live level and body tests; the hand
  arcs no longer shock or clip.
- The emitter has the hand's place for its tests and bounds, and the
  frame draws through the node's frame (per frame from a local-frame
  parent's drawn matrix); weapon spheres start tendrils unturned.
- The live arcs carry their node: the build attaches the emitter, the
  first step enters the world, the frame's removal or a despawn takes it
  out; the hum starts silent at pitch 1 at the node and sits at the mean.
- The records share one strided buffer.
- The tick's and frame's float order and NaN branches follow the assembly.
- A blow's wisp is placed as it enters the world; the wisps' tick and
  strip float order follows the assembly.
- DEC-11 asks the owner about the strip's scratch read and the respawn's
  endless draw.

## Impact

`exa_game::thaum::{arcs, live, wisps, veil_live, cast}`,
`exa_game::item_ability`, `docs/THAUMATURGY.md`,
`docs/roadmap/DECISIONS.md`.

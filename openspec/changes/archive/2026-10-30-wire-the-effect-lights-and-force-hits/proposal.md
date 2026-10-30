# Proposal

## Why

Two ported features had no live caller. The voxel GI draws registered
GI shapes (round 40), but the force blast, the force torrent and the
teleporter bubble computed their emissive values and never registered a
shape. The blast and torrent hits computed the struck character's
support-pin gain and a "5 s recoil" and wrote neither to the body.

Phase 6 (thaumaturgy), live effects milestone; Phase 4 rendering (voxel
GI shapes).

Evidence (REA, Ghidra provider, `Exanima.exe` 0.9.5.2g): the blast's,
torrent's and teleporter bubble's creators (each makes its emissive and
appends it to the scene's GI manager list), the manager's append and swap
removal, the blast's and torrent's destroy slots and the blast's frame
(which writes each chunk's light point and frees the emissive once it
builds nothing), the three emissives' bounds and draws (the points are
mapped into the volume before upload), and the blast's and torrent's
character-hit routines (pin gain written over the body's, then the
character's landing momentum weight set to 5).

## What Changes

- `exa_render::gi`: the line and multi-point shapes measure in the
  volume's units (the block's width is 1), as the original's do; their
  positions arrive in world centimetres and are scaled, `Atten` is the
  original's. A shape carries up to 16 points (the blast's 16 chunks, the
  torrent's 16 light points).
- `exa_game::effects::blast`: the frame writes each live chunk's light
  point (the second sample of its bent ribbon spine, back in the world);
  the hit's pin gain is named as such; the "recoil" is the landing
  momentum weight 5.
- `exa_game::effects::Frame`: the inverse transform takes a point's dot
  product with each basis column (it applied the frame a second time,
  which only a self-inverse frame survives); the blast's ribbon and its
  light points go into the camera's space and back correctly.
- `exa_game::thaum::effect_light` (new, live): the blast, torrent and
  teleporter bubble register their GI shapes, refresh them every frame and
  free them (the blast once its frame builds nothing; each when it leaves
  the scene). The manager's light scale is 0.09.
- `exa_game::thaum::blast_live`, `torrent_live`: a character hit writes the
  knock's pin gain over `Drive::pin_gain` and gives the balance its full
  landing momentum (`Drive::land`); the body tick's recovery brings the
  gain back by 0.001 a tick.
- `exa_physics`: `Balance::landing_weight` reads the landing momentum.
- `exarust --debug-force-cast <s>`: the player casts its first power at
  the nearest character every s seconds and logs the GI shapes and the
  struck bodies' pin gain.

## Out of scope

The barrier, bubble, barricade, cuboid, strikes and tendril emissives
(no live effect registers them: the strikes effect is not live), the
teleporter bubble's mesh-node radius and its force texture (a stand-in
radius and the pass's shared white texture), the hit's other side effects
(the state flag and blocking word it clears), sounds.

## Impact

`exa_render`, `exa_game`, `exa_physics`, `exarust`.

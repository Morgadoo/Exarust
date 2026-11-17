## Why

Lane `collision` (roadmap B4, port lanes L02 collision & queries and L03
float call sites): leaf routines and call sites whose Exarust form still
differed from the original's arithmetic (library trigonometry, glam's
quaternion and lerp forms, single-precision friction, a summing order, a
wrong epsilon). Each change was read from the assembly; the leaf routines
are checked by the function oracle.

## What Changes

- The stored-contact pass (the body's contacts held inside the
  iterations) takes the level pass's friction: limit and slide factor in
  double, the strict compare, and the engine's paired weighted point.
- The thigh's extra turn follows the original's compares (the engine
  epsilon, a NaN pair has no angle, a cosine over 1 none) and the engine's
  axis-angle quaternion; the decay is the engine's lerp toward the identity
  and its quaternion normaliser, not a slerp; the hip and knee hinges are
  the engine's quaternions about X.
- The foot pitch turns by the engine's X turn and composes by its 4×3
  product; the limp's physical turns read the engine's matrix-to-quaternion.
- A weapon grab's re-anchor takes the exact barycentric weights over the
  struck part's particles, stored as they come.
- A body part's drawn matrix is the rest basis (the widened rest points'
  4×4 inverse) times the particles, as the original builds it.
- The particle ring and the emitter's spawn ring turn +X about +Y by the
  engine's axis-angle routine; the thaum helper delegates to it.
- The thaum tree's frame colour unpacks the accent bytes by the stored
  1/255.
- Oracle specs for the widened rest points, the 4×4 inverse and both basis
  products, identical over 5000 cases each.

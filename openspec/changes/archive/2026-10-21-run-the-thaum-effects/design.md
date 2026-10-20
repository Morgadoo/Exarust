# Design

## Behaviour

The rules are pure functions and simulation structs in `exa_game::thaum`,
in cm, at the body tick, drawing from the runtime's MT19937 stream
(`random(n)` as the high half of the draw times n, `random` as the draw
over 2³²) or the engine's `x·0x0808_8405 + 1` generator where the original
uses that one. Vector helpers keep the original's summation order, its
rsqrt normalisation and its polynomial sine and cosine.

- **Casts**: a body state struct (`CastBody`) carries the fields the gates
  and casts read; each cast returns the original's word and what it makes.
  The Displacement gates hand back the caller's last word on their open
  path, as the original's register does.
- **Arcs**: the tendril records keep their counts, points, previous
  points, branches, lives, phases and timers; a branch's origin names its
  parent's point by index, so a trade re-points it as the original's
  pointer fix-up does. Each record owns growable arrays where the original
  shares one strided buffer (a long trade can spill into a neighbour's
  slots there).
- **Barricade**: its frame is built from the facing as the engine builds
  it; ripples, reaches, halves and the mesh follow the original's fields.
- **Wisps**: rings of eight nodes with the original's darkening, growth
  and Verlet retention; the strip reproduces the smoothing and side
  easing; where the original reads its scratch past a lit last node, the
  last point stands in.
- **Live**: `ThaumEffectsPlugin` ticks the effects after the bodies, tests
  the level with the static colliders and the bodies with their parts'
  spheres, pushes particles through `Ragdoll::push_sphere`, knocks back
  through `Shove`, and draws with `GlowMaterial`.

## Open questions

- The limb query of the arcs carries a 20.0 whose role is not traced; the
  live test uses the limb spheres alone (REA: the body's query function).
- The order of the wisps' geometry setup and add-to-world is inferred from
  the budget the second copies from the first.
- The barricade's impact sound position reads uninitialised stack in the
  original; only its depth is kept.
- The effect programs (`effectfbrcd`, `effectforce`, the electricity
  shader) and the force noise texture are not ported.

## Docs

`docs/THAUMATURGY.md` describes the rules; research notes in
`context/re/thaum-casts.md`, `thaum-arcs.md`, `thaum-barricade.md`,
`thaum-wisps.md`. No `docs/formats/` page changes.

# Design

The body solver stays in `exa_physics`; the names it needs to sound reach
the body's root from `exa_world` (`BodySounds`), and `exa_audio` plays them
in the fixed step after the solver, reading the solver stage's touches and
each body's level impacts and scrape events. The collision arithmetic is
the existing `base_impact_sound`; the stateful wrapper `impact_sound` keeps
the part's clock and threshold and the generator draw, replacing the
earlier duplicate. No docs/formats/ page changes: the material's collision
sound field is already documented with `exa_formats::material`.

The frozen branch keeps Exarust's two particle sets: the original copies its
oldest set into both newer ones; holding the previous set is the same pose
for a body already held still.

## Open questions

- Which shipped objects would ever play a scrape loop: only two flails in
  `Objects.rpk` carry a slot. Answered by the corpus (`cargo exa bodies`).
- Whether the original's single generator order matters for the draw:
  each Exarust sound path keeps its own generator (labelled guess); a
  capture of the running game would be needed to compare draws.
- The dormant statue's count of 5 has no reader; REA found no reader of the
  count-reached mark, so it stays unset.

# Proposal

## Why

docs/PLAN.md, Phase 5 (visual fidelity) and Phase 7 (game modes and
content), the forms paragraphs: the arena loadout, divination, emblem and
destination forms had their rules and draw orders as tested functions but
their shaders were not run, nothing was submitted to the GUI draw path, and
the loadout form's control table was not decoded.

## What Changes

- The arena loadout's 27 control records (`exa_ui::loadout::CONTROLS`) and
  their layout pass (`loadout::controls`).
- The divination card shader per fragment (`div_card::card_sample`,
  `face_image`, `back_image`), the paint's frame on the draw path
  (`div_card::frame`: darkening, shadows, backs and faces, turning cards
  through the engine's axis-angle matrix with the shader's light dimmed by
  the turn) and a running screen (`exa_game::div_card_screen`).
- The company emblem shader per fragment (`emblem_form::emblem_sample`,
  `preview_image`) and its draw-path layer (`exa_game::emblem::preview_layer`).
- The destination symbol shader per fragment (`selection::symbol_sample`,
  `symbol_image`), the screen's frame (`selection::frame`) submitted by the
  running destination screen, and each row's marker word carried to the
  journey.
- Review fixes: the card turn matrix was the transpose of the engine's; the
  turning card's light, the turn's own shadow and the held face's mirror
  were missing; the card and emblem shaders' product order; the symbol
  colour is `base + add · mix` (the engine's add-scaled), not a blend; the
  divination screen no longer clears other screens' frames.

## Evidence

The control table read from the executable (27 records, the count the
constructor passes) matches `CONTROLS` field for field, captions included.
The engine's axis-angle routine writes its second term as `x·y·(1−c) − z·s`
and its diagonal as `(1−x²)·c + x²`, which `exa_physics::engine_math::
axis_angle_to_mat3` already reproduces. The paint uploads the light
(0, 0, 1) to `par0`, transformed by the turn matrix while a card turns. The
destination paint's colour step is a four-wide `a + b·t`. The shader
sources are the shipped strings already in the crates.

## Out of scope

The loadout form's own paint and its three-container drop reconciliation,
the skill pickers' skin primitives, the emblem form's chrome and palette
table, the divination item opener and sound, the destination's symbol atlas
decode in the asset pipeline and the skin colours behind the symbols (the
line skin stands in), and the GL face-culling state that decides whether a
turning card's back or face shows.

# Proposal

## Why

The Mind unit's routines (target pick, the sixteen casts, the Sense Minds
overlay and tracker, the glow, the screens; 22 KB) and the force blast and
torrent simulations with their geometry and GI lights (37 KB) were only
partly in Exarust. A first port of them reached the tree, but a review
against the instructions found most of it invented or wrong: thrown blast
chunks never moved, the direction ring started on the wrong axis and
angle, the casts drained the focus pool instead of the spent focus, bound
the caster instead of the target, read record kind 3 as living, and the
sight's weights, colours and billboard did not follow the original. This
change replaces that port with one taken from the instructions.

## What Changes

- The Mind casts run on the shared cast body and hand their work to the
  power classes: the front end, the gates, Calm, Enrage, Confusion, Trust,
  Suspicion, Fear, Embolden, Loyalty, Control Mind, Raise Dead and Command
  Dead (with the revival), Sense Minds, Mask Mind and Spirit Form, the
  restore of a saved power (wired into the load) and the binding helpers.
- The Mind sight is rewritten: pick, overlay, tracker, mood weights,
  colours, billboard, glow, sphere shaping and the spirit screen's start.
  The older control steps that duplicated the power classes go.
- The blast and the torrent are ported: directions, aim, start, charge,
  fizzle, thrown and running ticks, splits and removals, bounds, alert,
  the contact arithmetic, the frames' sounds, colours, texture offsets and
  light points; their geometry classes and multi-point lights join the
  existing effect geometry and emissive modules. The invented draw
  descriptions and ribbon mesh go.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the Mind casts and sight, the force blast and torrent.
- `effect-nodes`: the blast's and torrent's geometry classes and lights.

## Impact

- `exa_game::mind` (`casts`, `sight`), `exa_game::effects::{blast,
  torrent, effect_geometry, emissive}`, `exa_game::thaum::{force, power}`
  (`BlastSetup`'s field names, the Mind restore in the load).
- `docs/THAUMATURGY.md`, the `exa_game` README.
- Evidence: REA (Ghidra) assembly of every function in the group, the
  engine helpers they call (vector, matrix, trigonometric, random and
  turning routines), and the data they read (the Mind table, the colour
  and direction constants).

## Out of scope

- The ribbon vertex builds of both force effects, the sight's and screens'
  GL passes, the animated noise and the shader sources.
- The engine's mesh and sphere tests that find the force effects'
  contacts; the sounds' playback; the player behaviour's turn spring the
  torrent's frame follows (taken as an input).
- Casting: no power bar or AI cast spawns the effects or runs the casts.

## Phase

Phase 5, visual and audio fidelity: thaumaturgy effects (force waves,
barriers, tendrils), and the Mind powers behind the power tree.

# Proposal

## Why

The player could not touch the world: a left press did nothing outside
combat mode, props and corpses could not be dragged, the cursor's reach was
an Exarust guess (3 m around a point 80 cm up), and the classification and
reach rules decoded earlier (`pick`) were not wired into the game. The
original's player controls take hold of props with Ctrl and of corpses with
the left button, pull them with a world link of the body solver, refuse
anything else until the release, and measure the reach every frame.

## What Changes

- The solver's horizontal drag link (the hand's grab) on limp bodies: the
  gap across the ground, twice the strength, the 3·√mobility (≤ 5 cm) cap,
  the approaching motion taken off, the per-tick pass limit.
- The left press: kept for combat, out of reach and down ignore it; a
  movable prop is held (no standing quarter turns) and dragged with Ctrl
  (strength 1, two passes) or "lifted" without it; a corpse is dragged by
  the limb under the cursor (strength 0.5, every pass); anything else is
  refused until the release (the cross, the movement request left alone).
- The drag each frame: let go beyond 330 cm of the hip (double precision),
  when a held character lives, or on the release; otherwise the anchor
  follows the cursor's ray on the plane at the grab's height within 200 cm
  of the hip, waking the body to 256; out of reach it stays.
- The reach each frame: the hit's world height at most 245 cm, the offset
  from the hip within 300 cm up/down and across (doors 300/350 by their own
  place), NaN passing; a drag in hand is in reach; the player down or dying
  lets go and is out of reach. The cursor's pointer/cross follow it.
- The pick under the cursor: the level and the props' stand-ins (by
  collider), the characters' collision spheres, nearest first.
- `pick::classify` reads the prop's mobility (the field was misnamed mass).

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: the cursor pick's prop rule, the reach, the press, the
  drag, the refusal.
- `object-physics`: the hand's grab on a body.

## Impact

- `exa_physics::body` (`body_drag.rs`: `Grab`, `drag_pull`,
  `Ragdoll::grab`, `pull_grab_to`, `hold_grab_still`, `grab_point`,
  `release_grab`; the pass counter in `iterate`), `Physics::fixed_pick`,
  `prop::StandIns::owner_of`.
- `exa_game::hand` (new: `Hand`, `Holding`, `press`, `lets_go`,
  `drag_anchor`, the press, hold and reach systems), `exa_game::pick`
  (`within_reach` exact, `mobility`), `exa_game::cursor` (reads the hand),
  `exa_game::player` (`steer_player` keeps the request while refused or
  lifting; `PlayerBody::hold_object`).
- Docs: `docs/PLAYER.md` (The reach, The hand), `docs/UI.md` (World cursor),
  crate READMEs. `docs/PLAN.md` is the integrator's.
- Evidence: REA (Ghidra 12.1.4) on the left-press, drag, release and
  refusal handlers, the per-frame request's reach branches, the controls'
  frame, the reach test, the world-link adder and the limp-body link
  iteration's drag branch, read in the assembly for the arithmetic
  (constants 245, 300, 350, 200, 40000, the double 108900, 256, 1.19209e-7,
  3, 5, 2; the pass test `>`); notes in `context/re/player-control-hand.md`.
  No corpus data is needed: the rules read no game file.
- Out of scope: the lift onto the item cursor (no item cursor exists), the
  door, wheel and cloth drags (those objects are level geometry in
  Exarust), the Ctrl interaction (inventory, loot, talk, containers), the
  click's naming and use, the hotkeys, the player light, the idle camera
  orbit, the reach's navigation-region test, a living body's world-link law
  (the hand never grabs one), and the hand's handling of Tab while dragging
  (the original runs its lift cancel).

## Phase

Phase 3, bodies and control: M34, the hand's drag ("Still to do:
interact").

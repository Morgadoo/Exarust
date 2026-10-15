# Proposal

## Why

Combat is Exanima's core. Milestone M4 ("first duel") needs a playable loop
to build the physically driven version on: wield a weapon, attack, get hit,
go down.

## What Changes

- Weapons from `Objects.rpk` held in the right hand (`--weapon`).
- Animator: one-shot clips that hand back to a looping stance.
- `exa_combat`: blade segment vs body-part bounding spheres during attacks,
  damage ∝ tip speed², staggers, knock-downs into the unconscious pose.
- Player: combat mode (F, one-handed moveset, facing the cursor), attacks on
  the left button with direction from mouse motion.
- `exa_ai`: a duellist that faces, approaches and strikes (`--opponent`).
- `exa_physics`: characters have capsule colliders, so they block each other.
- Node bounding spheres decoded and kept (used for body-part hit volumes).

Out of scope: physical bodies, armour layers and damage types, weapon
assembly from factories, blocking/parrying, health UI.

## Capabilities

### New Capabilities
- `combat`: attacks, weapon contacts with body parts, damage and reactions.

### Modified Capabilities

## Impact

`crates/exa_combat`, `crates/exa_ai`, `crates/exa_anim`, `crates/exa_character`
(wielded weapons), `crates/exa_game` (combat controls), `crates/exa_physics`,
`crates/exa_formats` (node bounds), `crates/exa_world` (`PartBounds`),
`crates/exarust`.

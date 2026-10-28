# Proposal

## Why

The simulation sector's pair contact generation tests the more mobile
part's spheres against the other part's collision mesh, and the level
detection's impacts reach the character part's impact call (sound and
damage) with no other body. Exarust only tested spheres against spheres and
dropped the level impacts on the floor. A DeepSeek lane (simsector3) added
both; its review found an invented static-only gate in the pair sweep, a
reversed NaN/epsilon friction gate, a mesh-box pre-test applied to the
sphere fallback, and a level impact that named the character as its own
striker.

## What Changes

- `exa_physics::body_pairs`: the pair stage's per-triangle mesh query (the
  static/swept choice on 0.1 cm² of travel with no static-only gate, the
  mesh box and mesh sphere pre-tests, the sphere re-read after each hit),
  with the sphere fallback pre-tested against its own spheres' bound;
  `Ragdoll::set_part_triangles`; `Pairs::hit_events`; the pair friction
  skips only a product under the epsilon.
- `exa_physics::body`: the level push gate rejects only NaN and pushes under
  10⁻⁵ cm²; the placed contact offset array is readable and cleared by the
  integration while placed and by sleep; level pieces are tested against
  the part's body mask.
- `exa_physics::sector`: the static collider grid and simulated-object list
  as tested data structures.
- `exa_combat::impact::level_impacts`: a character's level impacts take the
  limb curve, sound, damage, daze and collapse path, the overhead drop, and
  send no limb impact message.

## Capabilities

### Modified Capabilities

- `physical-body`: pair friction, mesh pair contacts.
- `combat`: level impact damage.

## Impact

- `crates/exa_physics/src/{body.rs, body_pairs.rs, contact.rs, sector.rs}`,
  `crates/exa_combat/src/impact.rs`, crate READMEs, `docs/formats/RSQ.md`.
- Evidence: the assembly of the sector's contact generation, the per-hit
  level callback (COMISS with the parity branch to the NaN check), the pair
  friction (COMISS, JP to the rub, JC to the skip), the body–level
  detection's impact call, and the motile part's impact routine (the
  combat-state step only with another body, the overhead drop);
  `context/re/packs/simsector`, `context/re/packs/motilepart`.
- Out of scope: per-part triangles from the loaders (the game runs the
  sphere fallback), collider masks from the level loader, the static grid
  and simulated-object stage on the live step, the pain voice and BadFall
  message, props' level-impact sounds, the hit-event damage path.

## Phase

Phase 3, bodies: physics (contacts between bodies) and Phase 4, combat
(impact damage).

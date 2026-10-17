## Why

In the original every prop and item with a rigid body is a body in the
83 Hz solver: registered asleep where the level placed it, it stays put
until a contact, a blow or the hand's drag wakes it, then falls, tumbles
and settles by the same Verlet rules as a corpse, and sleeps again.
Exarust bakes every prop into the static level, so nothing can move.
Moving props is the base of everything else in the working dungeon: world
links (M33), the hand's drag (M34), doors (M12), levers and plates (M35),
dropped weapons (L13), impacts (C22).

Phase 3 (Bodies: skeleton, physics, control), milestone MS4 "working
dungeon" (and MS3 "weapons as bodies", which needs free bodies for L13);
master plan lane E, change M32 `simulate-props-as-bodies`.

## What Changes

- The material record's physical divisor (RB32 answered: parameter block
  `0xA4`, 0 read as 1) and impact sound prefix (`0xAC`) decoded in
  `exa_formats::material`; `exa_assets::ModelNode` carries a node's rigid
  body with the divisor of its first mesh part's material.
- The level prop placement flag `0x80000` (static: none of its bodies
  registers) decoded (`Prop::STATIC`), and the same node flag on a
  level's own nodes (`rfc::Node::STATIC`: `arenao1`'s tavern table).
- `exa_physics`: a rigid body built as the original registers one
  (`Ragdoll::rigid`): particles at the node's matrix × rest points, at
  rest; mobility = stored / divisor; gravity shares 4/mᵢ normalised to 4
  (gravity at the centre of mass); the stored spheres against the level
  with friction 0.75; articulated children joined by their stored links
  and a pivot at their origin, shares by mobility; registered asleep
  (countdown 0). Solver seams kept small: per-particle gravity shares on
  `Part`, a body that skips movable level pieces, a fall-out flag below
  −50 m, deep sleepers not redrawn.
- `exa_physics::prop`: registration (`PendingProp`, `BodyNode`,
  `PropBody`), the meshes a body moves built as its stand-ins (fixed
  colliders moved with it, so capsules and character bodies keep meeting
  props as before), removal below −50 m.
- `exa_world::bodies`: which body trees register (free trees of props not
  placed static, of placed objects that are no mechanism and of the
  level's own model); counts in `LevelInfo`.
- Debug: `--shove-props <frame>`; the body overlay draws awake props, the
  traces and pauses keep to characters; `exa-tool bodies` prints each
  body's material, divisor and impact sound set.
- Docs: `docs/formats/RFC.md`, `docs/PLAN.md`, `docs/DEBUGGING.md`,
  `docs/ARCHITECTURE.md` if it names the solver's bodies, crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5: body registration, material scaling, the
  sector's node walk, link conversion and the runtime link adder, the
  mobility shares, the body chunk reader and set-up, placement at a
  matrix, the material constructor and loader, the level's prop reader
  (`context/re/master-physics.md` "M32 research").
- Corpus: every body node's first material in `Objects.rpk` and
  `Objlib.rpk`; the props of all 28 levels with props; the bodies of
  `exanima01` and `arenao1`.

Out of scope:
- body–body contact (C17): props meet only the level for now, so a woken
  prop falls through another prop's body (a cup through a table) and
  nothing but a blow wakes props in play (no pair contact, no drag yet);
- world links (M33): bodies hung from the world (lamps, wall chains,
  hanging braziers) stay static level geometry; doors, levers and push
  surfaces stay static until M12/M35;
- the hand's drag (M34), impact and scrape sounds (C22, the sound set is
  only decoded), item pick-up and drop (M36, L13);
- the prop sub-chunk `0x3DE0EC00` (child matrices saved by the editor),
  placed objects' articulated-part sub-chunks, the frozen node flag
  `0x800000` (no shipped body has it), the pair layer of flags2 `0x800000`
  (C17), the static-collider flag flags2 `0x200000` (Exarust bakes every
  mesh);
- the nav grid following moving props (M6).

## Capabilities

### New Capabilities
- (none)

### Modified Capabilities
- `object-physics`: material divisor, static props, props registered as
  bodies, props meeting the level only, falling out of the world.
- `physical-body`: deep sleepers keep where they were drawn; the traces
  keep to character bodies.
- `world-viewer`: `--shove-props`; the body overlay draws awake props.
- `asset-inspection-cli`: `exa-tool bodies` prints a body's material.

## Impact

- `exa_formats` (`material::Material::{physical_divisor, collision_sound,
  mobility_divisor, collision_sound_set}`, `level::Prop::{STATIC,
  is_static}`, `rfc::Node::{STATIC, is_static}`)
- `exa_assets` (`ModelNode::rigid`, `NodeRigidBody`)
- `exa_physics` (`body.rs` seams, new `body_rigid.rs`, new `prop.rs`,
  `contact::Level::movable`, the static build's stand-ins)
- `exa_world` (new `bodies` module, `spawn_model_nodes`, `LevelInfo::bodies`;
  depends on `exa_physics`)
- `exa_game` (debug tools keep to characters), `exarust` (`--shove-props`,
  load log), `exa_tool` (`bodies` prints the material)
- `docs/formats/RFC.md`, `docs/PLAN.md`, `docs/DEBUGGING.md`, crate READMEs

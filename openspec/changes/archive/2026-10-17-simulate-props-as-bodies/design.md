## Context

Formats page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md) (Rigid
body, Materials, Props). The research notes, with addresses, are
git-ignored: `context/re/master-physics.md` §1, §6, §12, §13 and "M32
research", `context/re/body-tick-order.md` §3 and §5.3,
`context/re/body-sleep.md` §3–§4, and the master plan's entry for M32.

What the original does when a level loads and a body ticks:

1. **Which nodes register.** The sector's node walk registers a node as a
   body when it has a controller and is not marked static; it then skips
   the node's whole subtree (its meshes are never level colliders).
   Registration recurses only into body children, so a body under a plain
   node under a body never registers. A level prop's placement flags
   `0xB80000` are copied onto its root node, and `0x80000` onto every node
   under it: such a prop is level geometry (3 883 of the 79 935 shipped
   props).
2. **Registration.** Countdown 0 and the asleep, awake and deep bits;
   mobility = the stored mobility over the divisor of the node's first
   mesh part's material; link mobility = mobility; particles = node matrix
   × rest points, history = current (at rest). A root body with links gets
   the "no level" mask and, unless its first link is a slider, a world
   pivot; a child body's links become body links (parent point + the
   node's local translation, shares m/(m + m_p)), then a pivot from its
   origin to the node's local translation in the parent (slack 0,
   strength 1).
3. **Material divisor.** The material constructor sets the divisor to 1;
   the loader copies the record's parameter block `0xA4` and turns 0 into
   1. The impact sound set is the record's `0xAC` prefix (up to 4
   characters) or the material name's first 3, then `col`.
4. **Set-up.** Gravity shares wᵢ = 4/mᵢ scaled so they sum to 4, from the
   stored point masses: gravity acts at the centre of mass. Rest edges
   from the unscaled rest points; friction 0.75 from the constructor.
5. **Tick.** Asleep: nothing (the first sleeping tick drops the velocity).
   Awake: the TTetraRBD step (retention by the default environment's
   bands, gravity by share), the 20 iterations (body links with the plain
   body-link law, which is Exarust's limp link law; level contacts; edges),
   level detection with the stored spheres, 8 friction passes, the
   post-step (countdown, sleep). A root below −5000 cm is flagged and its
   node removed.

## Goals / Non-Goals

**Goals:** every free body of a level's props, placed objects and own
model in the solver, asleep where placed, woken by a blow; mobility,
gravity, contacts, articulated children and sleep as the original's;
capsules and character bodies still blocked by props; the existing
character solver untouched in behaviour.

**Non-Goals:** body–body contact (C17), world links and hung bodies
(M33), mechanisms (M12, M35), the drag (M34), sounds (C22), pick-up and
drop (M36, L13), moving the nav grid (M6).

## Decisions

- **Props are `Ragdoll`s.** The limp body already is the original's
  TTetraRBD tick (retention bands, countdown, link law, contacts,
  friction), so a prop is a `Ragdoll` built by a new constructor in its
  own file (`body_rigid.rs`). The seams in `body.rs` are small and named:
  `Part::gravity` shares (1 on limbs, so characters do not change), a
  flag for bodies that skip movable level pieces, the fall-out flag,
  and `draw_bodies` skipping deep sleepers (unchanged since drawn) and
  bodies registered asleep (their nodes already show them).
- **The Ragdoll sits on the top body's node.** A model may hold several
  body trees; each is its own body on its own node, its part entities
  being the nodes. A part's `bone` is its index.
- **Stand-ins, not a second rapier body.** The static build keeps baking a
  body's meshes, tags them as stand-ins (`user_data`) and lists them per
  body; when the body moves, each stand-in is moved by its part's motion
  since registration. Capsules and the camera ray see them as before;
  character bodies still meet them as level (no regression for corpses on
  tables until C17); prop bodies skip them (`Level::movable`), so a prop
  meets only the level, as the task scopes it.
- **Hung bodies and mechanisms stay static.** Without world links (M33) a
  hung lamp would fall forever (it never meets the level), and doors
  without their motors (M12) would flop: they keep their baked meshes
  until those changes register them.
- **Registration after transforms propagate.** The loader inserts what
  registration needs; a `PostUpdate` system after propagation places the
  particles at the nodes' world transforms in the frame they spawn.
- **exa_world depends on exa_physics.** The loader decides which trees
  register (it has the item's mechanism and the prop's flags) and writes
  plain-metre setups; exa_physics stays free of the formats crates.
- **A child's slider link** keeps its stored value as plain slack (the
  original drops flag 4 on child links); none ships.

## Risks / Trade-offs

- [C17 missing] A woken prop falls through other props' bodies, and in
  play nothing wakes props yet. Accepted and stated: a cup on one of
  exanima01's tables (bodies) would fall through it once woken;
  `arenao1`'s tavern table is a node marked static, so its shoved
  tableware lands on it.
- [Scale] The original keeps body scale 1 and places particles at the
  node matrix, so a scaled placement would be squeezed back to the rest
  edges once awake. Body props' placements are unit to 1 % (exanima01:
  3 133 exact, 15 within 1 %), so this never shows.
- [Cost] exanima01 registers 4 591 trees (5 000 bodies, up to 2 927
  spheres each). Asleep they cost a sleep check per tick; drawing and
  stand-in updates skip them. An awake body with thousands of spheres
  is expensive for its 1.5–3 s awake.

## Open Questions

- The editor's saved child matrices (prop sub-chunk `0x3DE0EC00`) and
  the placed objects' articulated-part sub-chunks: decode when an
  articulated prop shows posed wrong (REA on the level's prop reader).
- flags2 `0x200000` (static collider) is not honoured by Exarust's bake,
  which bakes every mesh: a later change of the static build.
- The 7 props parented to a body prop keep their meshes static in
  Exarust (they hang under the prop's placement entity, not its body
  node); none has a body of its own.

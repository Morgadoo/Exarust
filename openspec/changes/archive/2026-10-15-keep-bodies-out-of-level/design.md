## Context

The body solver (`exa_physics::body`) reproduces the original's articulated
bodies: each part a tetrahedron of four Verlet particles, links between
parts, muscles toward the animated pose. Level collision was Exarust's own: 8
passes after the links, each pushing every sphere off the nearest point of
any fixed collider (rapier `project_point`), clamped to twice the radius.
The research notes (git-ignored) are `context/re/body-contacts.md` and
`context/re/body-tick-order.md`; the format page this extends is
`docs/formats/RSQ.md` (Physical body).

## Goals / Non-Goals

**Goals:** keep body parts out of level triangles the way the original
does, in its tick order; stop the drive from flinging fallen bodies.

**Non-Goals:** body–body contacts, the original's own drive, foot planting.

## Decisions

- **Tick order.** Integrate and drive; 20 iterations in which each part
  takes its links, then its stored level contacts, then restores its edges
  (a living body from the hands and feet in, a limp one from the pelvis
  out); clear the contacts; detect against the level (push at once, store up
  to four per part); 8 friction passes; remember where the tick left each
  part. As in the original, a contact found in one tick is enforced in the
  next tick's iterations and then dropped.
- **Detection.** Per triangle, in the mesh's frame: one-sided static test
  (centre in front and within the radius; push off the closest point by the
  overlap, along the closest-point direction); swept test from the last
  tick's end when the centre moved more than 3.2 mm (an end that crossed the
  plane goes back to where it first touched). Rapier is only the mesh store:
  `intersect_aabb_conservative` for colliders near a part, the trimesh's BVH
  for triangles near a sphere or sweep, `Triangle::project_local_point` for
  the closest point. `project_point`, shape casts and contact queries are
  two-sided or single-hit, so they are not used.
- **No `TriMeshFlags::ORIENTED`.** It makes parry treat a mesh as a closed
  solid and recover centres behind a face through the nearest boundary; the
  original never recovers such a centre, and level meshes are open pieces.
- **Friction** uses the part's own coefficient (2.5 for limbs as in the
  original; 0.5 for feet, a guess until foot planting is ported) with the
  original's limit, kept in its centimetre units.
- **Deliberate departure.** In one rare branch of the original's swept test
  (the closest point lies beyond both ends of the sweep), the push scales the
  vector from the start rather than from the end, which can push by metres.
  Exarust uses the bounded push off the touched point. It is the only place
  the contact code does not follow the original.
- **Drive fixes** (Exarust's drive, until the original's is ported): each
  tick's share of the pose change turns a part about itself; `fall()` clears
  the aimed pose, as `stand()` does; a fallen body's muscles act inside the
  body. Its targets follow its own pelvis, so damping relative to them feeds
  its motion back and flings it, while damping toward zero drags its fall
  (limbs settled near 0.4 m/s). The limbs are therefore damped relative to
  the body's rigid motion (its bulk velocity and its spin), and the drive's
  net shift and net turn of the body are removed after each pass: muscles
  change the pose, not where the body goes. The pelvis, which the pose is
  measured from, has no muscle of its own, so pulling the limbs towards the
  pose also turns the whole body; with only the net shift removed, fallen
  bodies in `--shove` rolled and crawled up to 1.9 m from where they fell
  (median 0.66 m), against 0.58 m at most (median 0.35 m) with the turn
  removed too.
- **Link shares.** A link moves each of its points exactly (Exarust's
  spread over the particles, weights over their sum of squares), so its
  correction is split by each point's generalised inverse mass: the part's
  inverse mass times the point's squared weights. The two parts' momenta
  then change by equal and opposite amounts, as with the original's spread
  by raw weights. Split by the parts' inverse masses alone, a link whose
  points lie unevenly in their tetrahedra (the pelvis's link points lie
  outside it) fed momentum into the body on every iteration: limp bodies
  flew off at tens of metres per second, and fallen living ones were
  thrown metres from where they fell.

## Risks / Trade-offs

- One-sided faces assume level meshes face outward, floors up. A part that
  starts under a floor falls through, as in the original. The corpus check
  in the tasks counts the share of up-facing floor triangles per level.
  Placed dead characters are spawned standing at their feet position and
  only then go limp: in exanima01, 4 of the 7 start with leg spheres inside
  props (centres up to 0.4 m under an up-facing face). Those parts stay
  inside the prop; none goes below the floor. Laying the dead out where
  they are placed is a separate change.
- Collider and triangle order come from rapier's BVH, not the original's
  grid and kd-tree; sequential resolution can differ in edge cases.

## Open Questions

| question | how it will be answered |
|---|---|
| Do retail level meshes face consistently, floors up; are there mirrored collidable nodes? | corpus statistic over every level with `exa-tool` (skipped without EXANIMA_DIR) |
| Which level nodes collide (collidable flag, thin plates living feet ignore)? | REA: references of the static-collider registration and its mask set-up |
| How often does the departing swept branch fire? | a counter in a probe run of the game |
| Foot friction per tick and the foot-sole push | the foot-planting change, from the body tick |

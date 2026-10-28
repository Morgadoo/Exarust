# Design

- The static grid is a resource filled where the static builder inserts a
  collider, so it costs one AABB and one map insert per collider at load
  and nothing per frame. Pieces are the colliders' raw indices, which the
  physics world's triangle query already takes.
- Masks are looked up per queried piece in body detection (one hash lookup);
  a piece with no record answers 1, so colliders made outside the builder
  behave as before.
- The foot mask follows the body's muscles (`drive`), the Exarust state
  that stands for the original's living, uncollapsed body.
- The cloth stage runs the sphere test before the box distance (which
  inverts the part's matrix), and gathers triangles only from records that
  pass the object's pre-test, inside the object's bound box.
- Static records keep a world-aligned box; the original's oriented mesh box
  is not stored, so the static stage's box distance uses the world box
  (labelled guess).

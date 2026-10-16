## Why

Exarust joins a body's parts only by the skeleton's stored links, each a
hard radius. The original adds one more link to every part when it
registers the body: a pivot from the part's origin (its joint centre) to
where that origin sits in the parent. That pivot is the ball joint; with
the knee's zero-slack lateral link it makes the knee a hinge, and the
stored links only bound each joint's range. Without it, a femur's origin
can drift up to the slacks (3 + 14.9 cm) from the hip: in Exarust's limp
character view the joints gape by 8 cm on average and up to 19 cm.

Exarust also places the parent's link points through the part's full rest
transform. The stored parent point is already turned into the parent's
axes, so the original adds only the model's translation; the extra turn
put the upper arm's links up to 8.7 cm off. And Exarust splits a
correction by a generalised inverse mass, where the original shares it by
the parts' mobilities.

Phase 3 (Bodies), milestone M3 "standing man". This is change C3 of the
body-controller plan. It depends on C1 (done) and on Q3, which is answered:
the bodies sit on the model's nodes (`humanbase.rfc`) and the parent point
is the stored point plus the node's local translation, with no rotation.

## What Changes

- Every part with a parent part gets an origin pivot after its stored
  links: its origin to its place in the parent (the model's local
  translation), slack 0, strength 1. A femur has 3 links, a hand 4.
- A stored link's parent point is the stored point plus that translation,
  unturned. The game passes the stored points and the part's origin.
- A correction is shared m/(m + m_parent), m = 0.175 × the bone's
  mobility, and spread over each part's particles by the point's weights,
  unclamped.
- Living bodies: a pivot (slack ≤ 0.01 cm) closes d/(1 + 0.001|d|) of its
  gap in cm, in full however far apart; a slack link takes back its excess
  beyond the slack at full weight, cut to 80 cm.
- Limp bodies: a pivot closes strength/(1 + 0.001|d|²) of its gap (the
  original's plain iteration); a slack link takes back strength × its
  excess.
- Order: a living body children first (post-order over the part tree), a
  limp body parents first, depth first, each part's links, then contacts,
  then edges.
- Each link keeps its strength (1; a lower leg's first link with over
  10 cm of slack, the leg limit, at 0.2 while living, restored to 1 when
  the body goes limp, as death does) and the original's setup smoothing
  (1 − (r² − 25)·0.001)², at most 0.99, for the limp damping to come.
- Removed: `rest_under`'s rotation in link placement, the hard radius on
  every link, the generalised-inverse-mass split.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5: body registration and link
  conversion, the origin pivot, mobility shares, the living and plain link
  iterations, muscle setup's smoothing and leg limit, death's strength
  reset (`context/re/body-links.md` §0–2, §4.1, §5.1, §6, §10–11;
  `body-tick-order.md` §5.2; `body-controller-plan.md`, C3 and the answer
  to Q3).
- The installed `humanoid.rsq` and `humanbase.rfc`: every stored parent
  point is as long as its part's point (corpus test), and the model's
  translations place the pivots.
- Exarust runs with a temporary joint-gap probe, against the build before
  this change: `--limp character`, `play exanima01`, `--shove 60 play
  arena`, `character Default/Walk`.

Out of scope:
- damping a limp joint inside its slack through the smoothed length, and
  the limp retention (C4);
- link PD muscles toward the animated angle and the leg limit's per-tick
  strength blend (C5);
- scaling link points and slacks by the body's scale (other races);
- saved corpse poses (S2): placed corpses still stand up and collapse;
- growing the synthetic test humanoid to 18 parts.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: joints at the part origins, the original's link laws
  and shares, link strength and smoothing; limp bodies and body iterations
  updated to match.

## Impact

- `exa_physics` (`body.rs`: `Link`, `PartSetup` gains `origin` and
  `foot`, `Ragdoll::new`, `go_limp`, `links_of`, `link_points`, `iterate`,
  `tree_orders`; tests in `body_joint_test.rs`, setups in the other body
  tests)
- `exa_game` (`limp.rs`: `part_setup`, the part's origin from the model
  translation, `go_limp`; `exa_formats` becomes a normal dependency)
- `docs/formats/RSQ.md` (Physical body), `docs/PLAN.md`,
  `crates/exa_physics/README.md`

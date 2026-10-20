# Design

## Behaviour

The node's frame is three rows (m0, m1, m2) about an origin. The eye is `origin + (anchor_a + 15)·m1 + anchor_b·m2`; the direction is `normalize(eye − camera)`. The view transforms the direction; the blend is `1 − |d.x|`.

The four helper vectors are `a = normalize(m2 × dir)`, `b = normalize(a × dir)`, `c = normalize(m0 × dir)` and `e = normalize(c × dir)`. The axes are `U = normalize(lerp(a, e, blend))·0.025` and `V = normalize(lerp(b, c, blend))·0.025`, then transformed by the view. The eye output is `−view·dir`.

## Model

`sight_billboard(rows, origin, anchor_a, anchor_b, camera, view) -> SightBasis { u, v, eye, dir, blend }`.

## Open questions

The shader upload and the per-view matrices are separate.

## Docs

`SG_Thaum_Mind` is **9%**. The headline stays 14.4% and 5.7%.

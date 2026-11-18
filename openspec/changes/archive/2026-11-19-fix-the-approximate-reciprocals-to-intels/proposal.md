## Why

Decision DEC-7 (taken 2026-11-19): the original's approximate reciprocal and
reciprocal square root instructions are reproduced by one fixed software
approximation equal to a common Intel result, so runs repeat on every
machine. Exarust ran the CPU's own instruction (Intel and AMD disagree on
about half the inputs) and computed exactly at several sites where the
original approximates. Two port differences from the rulings review ride
along: the wisp strip's last side at a strand's visible end, and the fist
contact clamp at the far end of a striker.

## Evidence

REA (Exanima.exe 0.9.5): the sites' `RCPSS`/`RCPPS`/`RSQRTSS` instructions
(the swept box test, the weight normalisations, the sweep set-up and the
moved sphere's sweep, the level hit's centre fallback through the fast
barycentric routine, the engine normalise called by the light basis, the
polyhedron subdivision, the sphere tangent and the terrain window normal);
the wisp strip builder's end branch; the damage routine's hit point gate.
The Intel table was checked against an Intel Coffee Lake's four
instructions over all 2³² inputs.

## What changes

- `exa_formats::approx_recip`: Intel's reciprocal and reciprocal square
  root from two 2048-entry tables; `engine_math` and every local mirror use
  it.
- The listed exact sites take the approximation.
- The wisp strip adds no extra point at a strand's visible end.
- A striker's contact at or past its far end is ignored, fists included;
  the fallback sweep's contact point lies on the struck part's surface.

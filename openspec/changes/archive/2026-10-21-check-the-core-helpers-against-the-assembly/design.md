# Design

Format pages: [`docs/formats/SAVES.md`](../../../docs/formats/SAVES.md)
(package opening) and [`docs/formats/TERRAIN.md`](../../../docs/formats/TERRAIN.md)
(brush table and material channels).

## Method

Every routine of the seven units was read in the assembly, not the
pseudo-code: sum order, `MINSS`/`MAXSS` operand order, unordered compares,
the MXCSR bits of the rounding helpers (floor `0x2000`, ceiling `0x4000`,
restored), `CVTSS2SI` (nearest even, 64-bit where the code uses `RAX`) and
`CVTTSS2SI`, double-precision steps and signed against unsigned jumps.

## Decisions

- One maths implementation: `exa_physics::engine_math`. The UI depends on
  `exa_physics` for it; the other copies in the tree are left for their
  own reviews (listed as out of scope).
- The look basis's `RSQRTSS` normalisation changes the running body twist
  by at most about 4e-4; the twist tests allow 1e-3.
- A package open writes nothing: the original resets before it keeps the
  file's handle, and its reset writes only through a kept handle. A
  missing file is created empty (open-always).
- The original's tip draw uses the draw itself as the tip's index; this is
  kept, not "fixed".
- The object manager's rules stay plain functions; the asset loader keeps
  drawing every material because the original's `*` hide depends on the
  record (weapon or item body), which the loader does not see.

## Open questions

- The uncached actor reader's object chunks (`0xDBCD`, `0xDBDB`): REA on
  the reader's callees when the shopkeepers and summons are built.
- Whether the practice spot (0, 5, 107) is in Exarust's frame for the
  practice arena: a capture of the original's practice fight.
- The runtime library's x87 transcendentals are stood in for by the
  platform's double functions; a bit-exact port would need the runtime
  library's own routines (REA).

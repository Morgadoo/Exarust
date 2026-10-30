# Design

Format pages: `docs/formats/OTHER.md` (palette ramps, procedural material
programs), `docs/UI.md` (arsenal, inventory bind, item card).

The palette ramps keep the original's mixed precision: Rust computes
`step × 0.13` in `f64` and narrows it, multiplies by `k` in `f32` and adds
the base in `f32` (linear), or widens that product and adds `base + 0.1`
in `f64` before squaring (lifted). The cap is written as `if v > 1 { 1 }`
so a NaN passes, which `f32::min` would not do.

The paint draw helpers stay data functions over `procedural::programs`;
nothing in the CPU painter binds texture units, so they are tested, not
called.

The equipment panel's drop keeps the accept test as the only gate: the
panel moves the dragged item when `char_loot::accepts` says so and the
activation component is inserted only when `equip_drop` activates, which
is the AL crystal alone.

The arsenal view stays registered (one early-returning system); it is
painted only while open, and nothing opens it yet.

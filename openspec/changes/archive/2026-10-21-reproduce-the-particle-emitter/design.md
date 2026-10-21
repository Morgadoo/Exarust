# Design

## Behaviour

**Emitter** (`exa_world::particles`). A template is scaled by the fire
size, then turned into per-tick values: interval (83/rate)·⅔ in double,
lifetime trunc(83·life), speed/83, retention exp(ln v/83) in double, growth,
the pool sources·trunc(rate·life·1.5 + 1.501) + 2 and the density
normaliser. The ring has pool + 1 slots; its live range is [head, tail),
each part inclusive of its end slot.

A tick: the light walk and density (doubles); fire ageing (the emitter's
motion, the body carry, the half-chance extra tick, the ellipsoid cap
rounded half to even) or a plain decrement; the head pops, counting the
dead slot an emptied ring stops on; the burst refund; Verlet integration
with buoyancy·gravity and the centring pull; the finite or one-shot check;
the accumulator's two-thirds loss; spawning while the accumulator is below
one (NaN stops it); the bounds. On its first tick the emitter enters the
world at its node, so its sources' and node's last positions start there.

A spawn draws the life (when randomised), the atlas variant, the ring's
angle and radius (u⁴ in double), then the random velocity's three integers
(in double), x·axis₀ + y·axis₁ + z·axis₂ of the source frame; the Verlet
pair is p ± speed·dir, then q loses 0.3 of the source's motion.

Per frame, quads are built for the live particles oldest first with the
age 1 − (life − α)·(1/L): the diamond (k = S/ℓ + t²·growth, X = c·k,
Y = c × A scaled by ±(1 + stretch)·k, the fast arc cosine), the bed quad
(half-length (1 + Y.y)·W) or a billboard. Keys are lerped per channel and
rounded half to even.

Random draws: an integer LCG and a Mersenne twister per emitter, with the
original's generators and draw order. Both are process-wide and seeded
from the clock in the original, so no sequence can match it.

**Fires** (`exa_world::fire`, `exa_world::flame`). Tiers by size as
`CreateFire` (0.35 and 0.2 compared in double). The key tables are the
raw records scaled at load: byte·(1/255)·0.7·255, rounded half to even,
RGB of keys 1…n. Large fires are one-shot and their entity is removed when
the emitter finishes. A flame under a `CarriedFire` root gets the node's
displacement per tick as its body velocity.

**Arcs** (`exa_render::electricity`, `exa_world::arc`). The presets'
segment length (4·size, then ×0.75, ×2 or the record's scale) is separate
from the width (2·size). Record flag 1 makes a sustained arc renewing every
n + Random(n) ticks; otherwise it is one-shot over n ticks. The ribbon's V
runs across it.

## Open questions

- The per-frame α the original passes to its vertex build (taken as the
  sub-tick fraction) — would need the caller of the render callback (REA).
- What the tendril emitter does with the reach, the second tendril value
  and the drift — `TTendrilEmitter`'s tick (REA, another component).
- Whether a carried torch's item is a physics body while held — the item
  attachment code (REA) or a capture of a moving torch's flame.
- The readers of the per-node detail value (size·1000/distance) and of the
  pool override's quality setting (REA).

## Formats

No `docs/formats/` page changes; the 84-byte arc record is described in
`docs/RENDERING.md` (Electricity).

## Why

The runtime lane (round 182): FPC SysUtils (90%) and FPC System RTL (95%)
were credited to Rust std without checking which of their routines the
shipped game's own code reaches and whether any of those change results.

## Evidence

REA (Ghidra on `Exanima.exe` 0.9.5), checked against the assembly, and the
Ghidra export's code and data references:

- Outside the run-time units, nothing references a SysUtils routine except
  the three FPC Math helpers already split off. Its Format, float and date
  formatting, locale settings and TEncoding serve only Classes, Variants,
  TypInfo and ComObj, which engine code never calls. The game's number
  text comes from the engine's own formatting units.
- Engine code calls the System unit's random generator: `Random` as a
  double (about 135 call sites), `Random(l)` for 32 bits (tendril
  emitters, electricity, the divination cards, the terrain manager, the
  object editor) and for 64 bits (the object database's paint seed).
  It is MT19937 behind `RandSeed`. A draw reseeds when `RandSeed` differs
  from the value the last reseed left, or when the word index has run one
  past the end (its initial value), and a reseed stores the seed's
  complement back. `Randomize` stores the tick count; particle systems
  call it when they set up. The item container (fixed seeds), the item
  description (the item's seed) and the character textures store into
  `RandSeed` directly.
- `Random(l)` raises a negative `l` by one, multiplies the zero-extended
  draw by the sign-extended `l` and keeps the high word. The 64-bit form
  takes the first draw as the low word and the second, top bit cleared, as
  the high word, and returns the signed remainder (0 for `l = 0`).

## What changes

- `exa_formats::rtl_random` holds the generator once: `Mt19937` (one
  stream, with `unit`, `random`, `random_i64`) and `RtlRandom`, the
  process-wide generator with the `RandSeed` rule and `randomize`.
- `exa_game::arena::Mt19937` and `exa_world::particles::Mt19937` re-export
  it (two copies removed); the thaumaturgy draws' `below` and the
  divination deal use its `Random(l)`.

## Not done

Exarust keeps one stream per subsystem, seeded with fixed values, where the
original shares one clock-seeded stream, so the cross-subsystem coupling
(the item container's or the item description's `RandSeed` store deciding
the next draw elsewhere) is modelled by `RtlRandom` but not wired.

## Context

Checked against the assembly of the wheel's hit method, hover method, click
method, paint and label, the GUI level switch and the tree's wheel texture
load.

- The GUI level switch stores the unit (level + 3, or 8 at level 4) and the
  scale global as the unit in double times 0.25, stored as single.
- The wheel texture's load writes the size word, the unit shifted left by 3.
- The hit method: ring `cvtss2si(76 · scale)`, reach
  `cvtsd2si((word · 0.65)²)` in double; the icon place is the ring times the
  single-precision sine and cosine of `(i + 2) · 2π · (1/6)`, each rounded;
  the compare is a signed 64-bit `dx² + dy² < reach`.
- The click: the hot icon, the 0x2000 modifier, a choice other than the
  current one and a known domain, and the parent tree's domain flag word
  without bit 2. It ORs bit 2 into the tree's flag byte, stores the game
  clock plus the GUI close time (256) as the tree's domain deadline and the
  hot icon as the tree's pending domain, shifts the wheel's choices, stamps
  the real clock and plays the confirm sound at 0.25, pitch 1.
- Only the click sets bit 2; the tree's present assigns 1 when the
  fade-out ends and clears bit 1 when the fade-in ends.

## Decisions

- Engine-replaced: the three program binders (compile once, look up `par0`,
  `par1` and `Tex`, bind the sampler to unit 0) are the software fragment
  evaluation of the tree canvas (`Program::WheelBack`, `WheelSector`,
  `WheelGlow`); `WHEEL_PROGRAMS` keeps their records. The mip level is the
  canvas rasterizer's per-triangle level over the flamelets chain, standing
  in for the GL sampler's per-pixel rule. The label's string-cleanup frame
  is the language's drop.
- The wheel form stays 280 pixels at `(10, height − 290)` at any unit, as
  the tree's opening places it; at units above 4 the ring reaches past the
  form, as in the original's code (no retail reference of the power screen
  is on file to confirm the picture).

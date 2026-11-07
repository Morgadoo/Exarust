# Design

## Evidence (REA on Exanima.exe)

- The race loader loads `difm`/`spcm`/`nrmm0…5` only when the race look
  word has bit 1 (both sexes) or bit 2 (men only); `races.rdb`: Human
  0x1bf, Ancient 0x86, every other race neither bit.
- The motion binder pairs each skeleton node with the model node of the
  same bone number and copies the skeleton's body chunk into it; a model
  node with no counterpart keeps its own body (the instancing clones mesh
  nodes' bodies). The part list takes only limb-class bodies with a bone
  number, so the demon's tail and wings are plain bodies, not limbs.
- The character init's limb mass value is the bone table value times
  `n / ((20·s³ + Σ) · d)` (n the parts, s the size, d the race mass
  divisor, Σ the skeleton parts' volume mobilities, volume / 10⁴ cm³;
  constant 20 read from the executable). Exarust keeps limbs at their
  table values, so a plain part's mobility is divided by that factor. The
  init gives a part-list member without a bone index the hittable class
  and a part tone of 0.01, so its pose drive and muscles are near zero;
  Exarust keeps its links limp.
- The equip refresh sets the character's reach to the race's unarmed
  reach when nothing is held (`exa_formats::weapon_record::equip`).

## Not reproduced

- The plain bodies' own point masses and rest points: Exarust fits its
  usual tetrahedron to the node's mesh box and keeps one inverse mass.
- Whether the carry reads the part tone: Exarust carries plain parts as
  limbs (the body's tone and energy).

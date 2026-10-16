## Why

In the original a weapon that draws blood keeps it: the retail arena shows
the player's longsword dark red from the tip down to about two thirds of
the blade, near-black at the tip and breaking into specks toward the guard,
a minute after the fight and with the skin already clean. Exarust's blows
bleed on their victims (`bleed-from-wounds`) but every blade stays clean.

In the original the weapon that deals a bleeding blow draws half the
wound's size R of blood (up to 1, never washed off; fists none), and a
second pass draws a blood gradient over the weapon, keyed on the height up
the blade, so that the stain runs down from the tip further as the blood
grows.

Phase 5 (Visual and audio fidelity), milestone M5 "looks and sounds like
Exanima", item "bloody weapons". It also serves Phase 4's M4 "first duel".

## What Changes

- **Blood drawn** (`exa_combat::wounds`): `BladeBlood` on the entity
  holding a weapon model; a bleeding blow adds 0.5·R to the striker's
  blood at the blow, kept within 0..=1, even when the wound is dropped
  from a full queue. Fists carry none.
- **Which meshes** (`exa_character::blood`): `WeaponStain` on a wielded
  weapon's holder: the tip's height L and the meshes the original stains
  (the first mesh of the model's root node; every part of an assembled
  weapon, which the original merges into one mesh), each with its
  placement in the weapon's frame.
- **Stain pass** (`exa_render::stain`, `stain.wesl`): `StainMaterial`, a
  Bevy material with the multiply blend over the same mesh: u = 0.025·x −
  0.5, v = 0.0125·y + o (cm in the weapon's frame), o = −(L/80)(1 −
  √a·√(80/L)); `BldGr02D` read as data, repeating across and clamped
  along; the texel faded to white by the object's fade; depth writes off,
  the depth nudged toward the camera, no shadows. Drawn once the blood
  passes 0.01; the pass follows the blood afterwards.
- **Debug helper**: `--weapon-blood a` gives the player's (or the
  character view's) weapon blood a from the start, for shots without a
  fight; the hit log names the blade's blood after each bleeding blow.
- Docs: `docs/RENDERING.md` (Blood on weapons), `docs/formats/RFI.md`
  (wrap flags), `docs/formats/OTHER.md` (Damage), `docs/DEBUGGING.md`,
  `docs/PLAN.md` (Phase 5), crate READMEs.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, `exe.asm`): the
  damage routine's blood branch (the striking item's blood slot: register,
  then add 0.5·R; fists' slot none), the add routine (clamp to 0..1), the
  registration (k1 = L/80, k2 = √(80/L) from the node's bounding box), the
  draw routine (threshold 0.01, uniforms (0.025, −0.5, 0.0125, o), the
  node's first sub-object, the +0.000 01 depth nudge, depth writes off,
  multiply blend), the two GLSL sources, the gradient's loader flags and
  the loader's wrap and linear-upload rules
  (`context/re/wounds-and-blood.md` §2.2 and §10, refreshed this change).
- Corpus: `BldGr02D` (256 × 512, its rows, its file flags), every weapon
  item's model in `Objects.rpk` (root mesh, tip heights, several meshes),
  pinned by tests.
- Exarust in game through `gpu_run.sh` (character view with
  `--weapon-blood`, arena duel), compared with the retail shot in
  `context/re/refs/blood/`.

Out of scope:
- blood sprays (L9), floor blood;
- the attacker's own blood when it strikes with a body part of its own
  (creatures' slot), blood across saves, session end and respawn (Exarust
  has none), the blood of a dropped weapon (L13 makes weapons free bodies;
  the holder keeps its stain until then);
- the original's 8-bit frame clamping a lit value before the multiply
  (Exarust's float frame clamps only in its post-processing).

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `blood`: weapons draw blood from the bleeding blows they deal and show it
  as a stain that runs down from the tip.
- `world-viewer`: the `--weapon-blood` flag.

## Impact

- `exa_combat` (`wounds.rs`: `BladeBlood`, `BLADE_BLOOD_PER_WOUND`;
  `lib.rs`: the blow reactions add the blood)
- `exa_render` (new `stain.rs`, `stain.wesl`; `StainPlugin` in the render
  plugin)
- `exa_character` (`blood.rs`: `placement`, `stained_meshes`,
  `WeaponStain`, `StainGradient`, `stain_weapons`, `DebugWeaponBlood`;
  `lib.rs`: the wielded weapon's holder gets `BladeBlood` and its
  `WeaponStain`)
- `exarust` (`main.rs`: `--weapon-blood`)
- `docs/RENDERING.md`, `docs/formats/RFI.md`, `docs/formats/OTHER.md`,
  `docs/DEBUGGING.md`, `docs/PLAN.md`, the READMEs of `exa_combat`,
  `exa_character`, `exa_render`, `exarust`

## Context

`bleed-from-wounds` made blows bleed on their victims; nothing records the
blood a weapon draws. In the original, the damage routine's blood branch
looks at the striking item's blood slot: a weapon registers with the
scene's weapon-blood list on its first bleeding blow and adds 0.5·R; the
hands carry a slot that adds nothing. Every frame a callback of the scene
renderer draws each registered weapon whose blood exceeds 0.01 a second
time: its node's first sub-object, with a two-stage program that keys a
blood gradient on the mesh's local position and multiplies it into the
frame.

Exarust holds a weapon as an entity (the `Blade`'s holder, its +Y up the
blade) with one child per mesh of the model: the root node's meshes at the
holder's origin (a plain model's own placement dropped), an assembled
weapon's parts at their factory placements.

This change extends [`docs/RENDERING.md`](../../../docs/RENDERING.md)
(Blood on weapons) and [`docs/formats/RFI.md`](../../../docs/formats/RFI.md)
(flags). Notes with addresses are git-ignored
(`context/re/wounds-and-blood.md` §2.2 and §10, `combat-look-plan-final.md`
L8).

## Goals / Non-Goals

**Goals:** the blood a weapon draws, per weapon, exactly as the original
adds and clamps it; the stain drawn over the same meshes with the
original's coordinates, gradient, sampling, blend and threshold; a flag to
show it without a fight.

**Non-Goals:** sprays, floor blood, creatures' own blood, blood across
saves or sessions, dropped weapons (L13), the 8-bit clamp before the
multiply.

## Decisions

- **Blood on the striker.** `BladeBlood` lives on the entity that strikes
  (`Hit::striker`), the holder dressing gives every wielded weapon. The
  blow reactions add 0.5·R inside the bleeding branch, after the queue and
  whatever it does with the wound, as the original does. Anything that
  strikes without one (a fist) draws nothing: the original's hands slot.
- **Registration up front.** The original registers a weapon on its first
  bleeding blow and computes L/80 and √(80/L) then; Exarust computes L
  when it dresses the weapon (`WeaponStain`) and draws nothing until the
  blood passes 0.01, which is the same.
- **Which meshes.** The original draws its weapon node's first sub-object.
  Exarust's parts of a node are its material groups in file order, so a
  model whose root node has meshes is stained on that node's first part
  (L the top of all the node's meshes, its bounding box). The factory
  merges an assembled weapon into one mesh with one material, so Exarust
  stains every part of an assembled weapon (L the top of them all, in the
  weapon's frame), each part's placement passed to the shader so that the
  coordinates are the merged mesh's.
- **A material, not a render node.** The stain is a Bevy material
  (`StainMaterial`) with the multiply blend (frame × colour with an opaque
  fragment: the original's ZERO, SRC_COLOR) on a child of the stained part
  that shares its mesh, so it is culled, drawn per view (supersampling
  included) and sorted with the other transparent draws. It casts no
  shadow and has no prepass. One material per stained mesh holds the
  offset; more blood rewrites it.
- **Depth.** The original adds 0.000 01 to the projection's depth row, a
  constant −0.000 01 on its −1…1 depth, so the overlay passes against its
  own weapon. Exarust's depth is reversed and 0…1: the shader adds
  0.000 005·w to the clip depth.
- **The gradient.** Loaded as data (the loader flag `0x100`), repeating
  across (flag `0x10000`, also in the file) and clamped along (no
  `0x20000` anywhere), trilinear with the colour textures' −0.75 LOD bias
  and anisotropy (no `0x200`, mode 0). Loaded the first time a weapon that
  can be stained exists.
- **Fade.** The original fades the overlay to white by the node's fade
  (an effect value that no Exarust object has): the uniform is kept at 0.
- **Debug blood.** `--weapon-blood a` sets the blood of the debug body's
  weapon when it is dressed (the body `--debug-wound` paints: the player
  or the character view's body), so a stain can be shot in the character
  view on a light run.

## Open reverse-engineering questions

- `BldGr02D`'s wrap mode. *Answered (REA):* the loader keeps the file's
  flag bits from `0x10000` up and adds the constructor's `0x100F0`; the wrap
  setup repeats S, T and R for `0x10000`, `0x20000` and `0x40000` and
  clamps to the edge otherwise. The file has `0x20A90000`: S repeats, T
  clamps, so below the stain's edge the white first row shows.
- Whether the gradient is sRGB. *Answered (REA):* the loader flag `0x100`
  selects the linear upload.
- What L is. *Answered (REA, corpus):* the node's bounding-box centre y
  plus half-extent y; for the weapon models it equals the top of the
  vertices (the short sword's 60.05 cm, the dagger's 35.9 cm, read from
  the files' headers and vertices).
- Which mesh is drawn. *Answered (REA):* the node's first sub-object;
  *[inf]* sub-objects are the material groups. 36 weapon items' models
  have several, all tools and props.
- What the node's fade is. *Open:* used by effect programs (barriers,
  teleports); held weapons are not faded, and Exarust has none.
- Blood across saves and respawns. *Open, retail request R2:* the original
  clears every weapon's blood at session end and respawn; Exarust has
  neither yet.

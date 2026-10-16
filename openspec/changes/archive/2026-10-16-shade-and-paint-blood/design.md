## Context

The original keeps wound blood as the alpha of each vertex's colour (255
clean) on the character's merged skinned mesh and on the meshes under the
head, lowered by its wound painter and read by the shaders' dirt mode 4
against the speckle `spatter01`. Exarust draws a character as a skinned
body plus separate skinned garments, every one a mesh shared through the
model cache by all characters wearing it, with no colour stream.

This change extends [`docs/RENDERING.md`](../../../docs/RENDERING.md) (new
section Blood, Skin) and
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Races: the flag
word). Research notes with addresses are git-ignored
(`context/re/wounds-and-blood.md`, `combat-look-plan-final.md` L6).

## Goals / Non-Goals

**Goals:** the blood channel on exactly the meshes the original paints, the
painter's maths, dirt mode 4 for every material the original gives it, the
race flag, a debug flag to see it before wounds come from hits, and no
change at all to characters that have not been wounded.

**Non-Goals:** wounds from hits (L7), sprays (L9), blade stains (L8), the
hair and translucent programs, dirt modes 1–3, blood across saves.

## Decisions

- **The colour stream, in bytes.** The blood is Bevy's vertex colour
  attribute quantized to Unorm8x4 (white, blood in alpha), the original's
  RGBA8 stream: 4 bytes a vertex, and Bevy's mesh pipeline turns on
  `VERTEX_COLORS` for exactly the meshes that have it. The shader's blood
  block sits under that define, so a clean mesh compiles the same program
  as before; the materials only gain a flag and a bound texture.
- **Copy on the first wound that changes a mesh.** Meshes stay shared
  through the model cache. The painter computes a mesh's new values from
  the shared data first; only when one falls does the character get a
  copy (colour stream white, then painted), swapped into its `Mesh3d` and
  `SkinnedPart` and marked `Bloodied`; later wounds edit the copy in place.
  Copying at dressing would cost every character its meshes again; copying
  every target on the first wound would copy a whole body for a cut on the
  arm. Cost, measured in game: a 20 cm chest wound copies 6 meshes
  (106 KiB), a 12 cm arm wound 3 (45 KiB), a whole body in braies 20 meshes
  (278 KiB, 2 989 vertices), the duellist's outfit 145 meshes (1.9 MiB,
  20 668 vertices), on the CPU and again on the GPU (76 bytes a vertex,
  4 bytes an index). Blood never fades; a copy lives as long as its part.
- **Painting on the CPU-skinned pose.** The painter needs the vertices
  where the pose puts them now. Exarust skins on the GPU, so the painter
  skins the target meshes once per wound on the CPU from the joints'
  global transforms and the inverse bind poses (Σ wₖ·Gₖ·Bₖ·v, as Bevy's
  shader does), after transform propagation in `PostUpdate`. Rigid head
  meshes use their global transform.
- **Targets by Exarust's structure.** The original's targets are its merged
  `_` skinned mesh and the head body's children. In Exarust: every visible
  `SkinnedPart` of the character (the body and every fitted garment, all
  skinned) and every other visible mesh whose nearest body part is the head
  (bone 10). Held items hang under the hand part and are never painted;
  parts hidden under apparel are not drawn and are skipped (they stay clean
  if later shown, as the original rebuilds its merged mesh).
- **The painter's random numbers.** The original draws its runtime's
  random number once per vertex inside the wound. Exarust uses its own
  fixed xorshift sequence (`WoundRng`), drawn the same way: the same
  distribution and the same per-vertex ranges, a reproducible picture.
- **Three program flags.** Dirt mode 4's maths differs per program (skin,
  apparel and items, hair). `BLOOD`, `BLOOD_SKIN` and `BLOOD_HAIR` pick it;
  `BloodProgram` holds the same formulas on the CPU for tests and binds the
  speckle as the existing dirt map slot.
- **Which materials.** The human skin (`_HumanSkin`, skin program), every
  painted or run-time apparel material and every factory item (painted
  through the same path; `_procmat1` and `_procmatr`, cloth), any record
  whose extra flags select mode 4 and that has a diffuse map (the original
  selects the mode only with a diffuse map), by its type, and a bleeding
  race's body-model materials whose names start with `s`. Race skins are
  type 1, so they take the cloth program, as Exarust already draws them.
- **Debug wounds on the front of a part.** `--debug-wound` picks the
  part's visible vertex furthest along the body's facing as the contact
  point, so a test wound lands where a frontal blow would, on the garment
  when one covers the part, three frames after the body is dressed and
  bound.

## Open reverse-engineering questions

- The painter's maths and store. *Answered (REA, `exe.asm`):* centre from
  a −0.5 constant times the travel; the projection d·D/|D| tested against
  0 (a zero travel's NaN falls through to the ball); 1.0 and 255.0 as
  doubles; the random draw inside the wound only; a float compare against
  the stored byte, then a store rounded to nearest. Notes updated.
- The race base material. *Answered (REA, `exe.asm`):* the race loader
  walks its body model's materials and, when the race's flag bit 0 is set,
  turns dirt mode 4 on (DrtTex slot, extra flags `0x4000`) for every one
  whose name starts with `s`. Corpus: every bleeding race's skin starts
  with `s`, every eye material does not; all are type 1 with a diffuse map.
- Which garments the original merges into its skinned mesh and which it
  keeps rigid on a bone (mail, plate). *Open:* Exarust skins every fitted
  garment, so all take blood; the code says rigid items on bones other than
  the head never do. Retail request (a wound through mail or a breastplate).
- The dirt texture's LOD bias. *Open:* Exarust samples it without the
  colour textures' −0.75 bias; to settle with the texture-quality code.
- The meaning of `_procmat1`'s extra bit `0x800` (`0x4800`). *Open*, not
  needed for blood.
- Hair: the original's `_HumanHair` has the mode; Exarust draws no hair
  yet, so the hair program's blood is in the shader and `BloodProgram` but
  on no mesh.

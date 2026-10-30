# Proposal

## Why

docs/PLAN.md Phase 7 (game modes and content), the thaumaturgy paragraphs
(SG_Thaum_Mind): the Mind sight's rules (pick, fade, tracker, billboard,
colours, sphere shaping) were in, but its draw was not: no sight program,
no noise picture and no sphere. Round 43's thaummind lane added them; its
review found the sphere reinvented beside the engine's ported UV sphere, the
object matrix transposed, the noise object's two weights misnamed and left
at their defaults, a second noise-to-byte copy, the CPU fragment missing the
shipped clamp, and a draw system that ticked and uploaded a 256² picture
every frame with nothing to draw. The lane's force-emissive GI registration
was dropped in the rebase: main already registers those shapes
(`thaum::effect_light`).

Evidence: the sight program's vertex and fragment source strings, the global
sight setup (sphere call, program, uniforms, the 256² R8 picture and its
noise object), the per-entry draw pass (depth test off, state block, object
matrix from the head node's 3×4 frame with each row scaled, `ObjMat` per
supersample layer), the 3×4-to-4×4 copy, the material state apply and blend
routines, the spirit screen's setup and the glow's add, read in REA (Ghidra
listing) and the objdump listing.

## What Changes

- `exa_render::mind_sight` (new): `MindSightMaterial` and `mind_sight.wesl`,
  the shipped sight program; the CPU `fragment_colour` with the shipped
  clamp; depth compare Always without depth writes; alpha-blended
  (labelled guess).
- `exa_game::mind::draw` (new): `MindSightAnchor`, `MindSightView`,
  `MindSightPlugin`; the sphere from `exa_assets::polyhedra::uv_sphere`
  shaped by `shape_sphere`; `object_matrix` from `sight_basis`'s rows; the
  noise from `ScreenNoise::field` on the game clock and the effects'
  generator; nothing runs without an anchor.
- `exa_game::mind::sight`: `ScreenNoise::weights` and `ScreenNoise::field`;
  `spirit_screen_init`; `MindGlows`.

## Out of scope

No Mind cast or game system creates a `MindSightAnchor` yet, so the sight
does not show in play. The spirit screen's own draw, the per-layer GL
uploads, the original's exact blend factors and the force audio are not in.

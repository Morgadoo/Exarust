# Proposal

## Why

Component gap lane for `SG_CharacterGen (unit routines)` (rated 80%). The
creator's preview rebuilt the whole figure for every edit: a new body,
skin and hair, back in its first idle frame. The original's creator calls
one of the generator's three rebuild paths per control: the texture-only
rebuild for the skin tone, hair colour, gloss and facial-hair edits, the
creation scene's hair rebuild for the hair list and the straight/wavy
toggle, and the preview wrapper's whole rebuild (which keeps the
character's frames) for the rest. Two of those generator routines (the
texture-only rebuild and its wrapper, the creation hair rebuild) were
ported as rules but had no caller.

Phase: Phase 4 character creation (no PLAN.md edit in this change).

Evidence (REA, Ghidra on `Exanima.exe`): the creator's control callbacks
(the hair-colour picker and its gloss, the skin-tone picker, the two
facial-hair steppers call the re-texture wrapper; the hair list and the
straight/wavy toggle call the creation hair rebuild; the shared change
callback and one more call the preview wrapper), the re-texture wrapper
and the texture-only rebuild, the creation hair rebuild, the preview
wrapper, the background dispatcher and the in-game re-dress it runs.

## What Changes

- `exa_character::rebuild`: `PreviewChange`, `preview_change`,
  `BEARD_BITS`, `HAIR_SHADING_BIT`.
- `exa_game::scene_bodies::character_retexture`: the skin job through
  `rebuild::retexture_only`.
- `exa_game::creator_view`: the built record is kept; an edit re-textures
  the figure in place, regrows only its hair, or rebuilds it carrying its
  animator and turn; `EXA_CREATOR_EDITS=1` debug edit script.
- `exa_game::creator_screen::Creator::edit_record`.

## Impact

The creator preview no longer rebuilds the body for colour or hair edits.
Not reproduced: the original's route from the hair-colour picker to the
scalp hair's material (Exarust regrows the hair with the new colour).

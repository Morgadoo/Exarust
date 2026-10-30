# Proposal

## Why

The base GUI draw skin (`TGUIDrawSkin`) still drew several parts with the
wrong slot or inks: the panels, control boxes, slider and scroll thumbs and
the form background's scroll troughs used the plain frame slot where the
original calls its raised box, the scroll bar's trough was not the accent
gradient, the shell, panel and scroll bar skipped their style frames, the
selection band took its inks from the caller, and the frame hit test probed
the wrong code for the small mark and the horizontal arrows in the wrong
order. This change advances Phase 7, milestone "Full UI using the original
GUI skin, icons and manual pages" (docs/PLAN.md).

## What Changes

- The panel, control box, slider thumb, scroll thumb and the form
  background's troughs and horizontal thumb draw the raised box (the skin's
  box slot), pressed and hot from the control's state.
- The scroll bar draws its style frames on its moved rectangle and the
  accent gradient for the trough; the shell and the panel draw their style
  frames on their own rect first.
- The selection band reads the highlight group's fourth and seventh shades,
  or the fifth and eighth when selected.
- The hit test answers the small mark's code −6 for style bit 8 and probes
  the horizontal arrows −13 then −14.
- Kept as they were (the lane's changes contradicted the assembly): troughs
  and marks press on the stored pressed code with the button down; the hot
  mark's inner bevel takes the hot group's seventh and eighth shades; the
  vertical thumb's bevel and line gradient run between the control group's
  fifth and seventh shades.

## Evidence

The skin's virtual table orders its slots style frames, cross, raised box,
accent bar, control box; each part's listing calls those slots and reads
its inks at fixed palette offsets that map to the group shades above. The
hit test's listing loads the codes −4, −5, −6 for style bits 2, 4, 8 and
−13 before −14.

## Out of Scope

The textured skin subclass (`TGUIDrawSkinTex`) and its overrides of these
slots; a running caller for the panel, shell and scroll bar parts, which
wait for forms that use them.

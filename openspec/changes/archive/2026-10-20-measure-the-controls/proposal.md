# Proposal

## Why

`RF_GUI_Controls_Ex` (16.5 KB after the split, rated 0%) carries the trackbar, the menu/drop-down list and the radio/check item semantics. Exarust has skin drawing and form metrics but no control geometry.

## What Changes

- The three text spans, including the `3U|1` quirk.
- The trackbar's cursor-to-value map with the knob offset and step quantisation, the thumb-left map and the value caption centre.
- The list's `6U` row pitch, `3U` separator, 1-based row hit test, clamped navigation and the 16-row extent with the `1/(count−16)` page fraction.
- The content width rule `2·inset + max row + 16U`, the `8U` shortcut inset and the starting `2U`.
- Radio and check activation.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the extended controls' geometry and item semantics.

## Impact

- `exa_ui::controls`, from the text spans, `Track`, `ListMetrics`, `item_at`, `step_selection`, `ListExtent`, `list_extent`, `ItemMeasure`, `item_extent`, `MenuItem`, `Activation` and `activate`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `RF_GUI_Controls_Ex` becomes **12%** in the source rating. The headline rises slightly.
- Evidence: the controls' span routines, the `3U|1` measure, the value, the thumb and caption, the list extent, the row hit, the navigation, the content size and the activation. No new corpus files.
- Out of scope: the text field, drop-down event table, spinner, colour picker, progress bar, image/texture views and the 2.5 KB menu draw.

## Phase

Phase 7, UI. The extended controls.

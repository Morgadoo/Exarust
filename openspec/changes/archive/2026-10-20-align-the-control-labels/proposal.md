# Proposal

## Why

The control-label alignment routine has exact horizontal and vertical style switches that the repo's skin alignment does not match.

## What Changes

- Horizontal style `0x1000`/`0x2000`/`0x3000`/`0x4000` and the default `x + pad + (U+1)/2`.
- Vertical style `0x100`/`0x200`/`0x300`/`0x400` and the default `y + (U+1)/2`.
- `align_label` returning the aligned point.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the control label alignment.

## Impact

- `exa_ui::controls`, from `LabelAt` and `align_label`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `RF_GUI_Controls_Ex` becomes **13%** in the source rating. The headline rises slightly.
- Evidence: the control-label alignment routine (the style switches and the `+U`/`+pad+U` differences). No new corpus files.
- Out of scope: the text-run drawing and the control-specific handlers.

## Phase

Phase 7, UI. The control label alignment.

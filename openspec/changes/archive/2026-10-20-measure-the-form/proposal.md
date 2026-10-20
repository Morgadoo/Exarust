# Proposal

## Why

`TGUIForm` (20.7 KB, rated 5%) owns the form's layout: skin measures, the default 302×402 form, anchoring, the client rectangle and the scrollbar. Only its parent helpers were modelled; the layout bodies that the report verified were missing.

## What Changes

- The skin measures `base = 6u−border`, `close = 4u+1`, `rail = 4u−1`, `tab = u/2+close`, and the DPI border `max(2, trunc(dpi/135))`.
- The constructor defaults 302×402, border 2, content 300×800, scroll step 0.025, page minimum 76×90.
- Anchoring: default (4, 36), centred, or far (parent − size − 4).
- The client rectangle from size, border and the style word, including the right and bottom insets.
- Scroll metrics and the scroll setter: step `10/overflow`, thumb `max(9, trunc(cw/content·track))`, thumb x `border+rail+trunc((track−thumb)·f)`, 0..1 clamp, optional page snap and the no-overflow reset.
- Label brackets `[t]`/`[[t]]` and the selected row's `unit/3` nudge.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the form's metrics and scroll geometry.

## Impact

- `exa_ui::form`, from `FormMetrics`, `border_for_dpi`, the default constants, `Anchor`, `anchor`, `ClientRect`, `client_rect`, `ScrollMetrics`, `scroll_metrics`, `set_scroll_x`, `styled_label` and `selected_nudge`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TGUIForm` becomes **12%** in the source rating. The headline rises slightly.
- Evidence: `0x100031a80` (skin), `0x10002a8a0` (defaults), `0x1000291d0` (anchors), `0x10002be20` (client rect), `0x10002c2d0`/`0x10002bf60` (scroll), `0x100128f30` (labels/nudge). No new corpus files.
- Out of scope: the main-menu row spacing (the binary's `70·0.5·round(H/135)` does not fit its own list and needs a screenshot), text advances, key traversal, and the settings/manual rows.

## Phase

Phase 7, UI. The form's layout geometry.

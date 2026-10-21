# Proposal

## Why

The template decode shows the Settings page also has a **Music** toggle beside the audio slider, and the Controls page has **22** bindings; the screenshot only showed the first 14 (the rest scroll below the fold). The screenshot-derived model omitted both.

## What Changes

- Add the Music toggle on the audio row's right column.
- Extend the Controls list from 14 to 22: Skills, Journal, Talk, Use Power, Power Bar, Previous Power, Next Power and Power Tree.
- Keep the screenshot's visible keys and use the template defaults for the newly exposed rows.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the Settings and Controls rows.

## Impact

- `exa_ui::settings`, from `SETTINGS`, `CONTROLS` and their tests.
- `docs/PLAN.md`; the coverage headline is unchanged (the page is screenshot-derived).
- Evidence: the Settings template array (Music, class 2 id 4 at x 320) and the Controls template array (43 records = 22 labels + 21 keys; rows below 404 px). No new corpus files.
- Out of scope: the exact anchor/record layout pass.

## Phase

Phase 7, UI. The Settings and Controls rows.

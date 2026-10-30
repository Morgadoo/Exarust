# Proposal

## Why

A sweep over eleven small components of the game modes, arena and session
area (Phase 3 game flow and Phase 7 arena career, see docs/PLAN.md) closed
some of their gaps: the loadout form's equipped marks and arsenal return,
the story's live experience, the end menu's registered scripted modes, the
prelude's tip banner, the divination card's sound and the shop baskets'
items. The review checked each rule against the assembly and corrected four:
the loadout constructor's mode words write the form's control records (0x110
is the hidden state), the arsenal drop asks the company arsenal only when
the form's own entry was not lent, the company arsenal's lend query goes on
to every box, and the card draw paints both sides of a turning card with
face culling off. It also wired the tip banner's hide and drove the live
experience from the flow's existing discovery pass, writing the converted
state back to the player's record.

Evidence: REA (Ghidra) reads and objdump listing of the loadout form's
constructor, its equipped-mark refresh and its equipment drop handler, the
arsenal unit's lend query and entry write, the experience unit's hook
installer and tick, the prelude's tips show and hide routines and their
string table, the end-session menu's hide, and the divination card draw with
the engine's cached cull-face setter.

## What Changes

- `exa_ui::loadout`: `mode_records`/`controls_for` (the constructor's
  per-mode record changes), `EQUIPPED_MARK`/`mark_equipped`; the invented
  24-slot `creation_equipment_slots` removed.
- `exa_game::loadout`: `mark_stock`, `return_to_arsenal` (lend query only
  after the form's own entry, flagless write only for another category).
- `exa_game::arsenal::Arsenal::release`: one lend back per box.
- `exa_game::experience_live` (new): `AccountExperience`, `StoryExperience`,
  `DiscoveryTick`, `TechniqueWord`, `ExperienceLivePlugin`; the flow's
  `Step::Discovery` sends the tick.
- `exa_game::end_menu_screen`: `EndHandlers`, `ScriptedEnd`.
- `exa_game::prelude`: `banner_text`, `TipBannerText`, `TipsScreenPlugin`;
  `exa_ui::game_tips::{TIPS, HINTS}`; the flow applies `Step::Tip` and hides
  the banner on `Step::HideEndMenu` and `Step::ClosePrelude`.
- `exa_game::div_card_screen`: `PprMov` at each paint sound step.
- `exa_game::shop_window`: item ids beside the basket words (a list model,
  labelled a guess).

## Out of scope

- The loadout's 3-D preview and arsenal icon tree; any live caller of
  `return_to_arsenal`, `take_stock_row` and `offer_item`.
- The explored-cell half of the discovery tick (no level grid adapter), the
  pickup and kill hooks, and a body system reading `TechniqueWord`.
- Scripts registering end-menu modes (`exa_script` is empty).
- The emblem editor screen, the character creation and class selection
  screens (unchanged; their rules were checked as present).

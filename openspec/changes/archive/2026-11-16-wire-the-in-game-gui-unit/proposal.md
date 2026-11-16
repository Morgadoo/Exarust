## Why

Roadmap tasks B1 and B4 (wiring lane `gamegui`): the in-game GUI unit
(`SG_GameGUI`) had about 34 open ledger rows and the findings F31, F32, F34
to F38 and F278. The heartbeat stopped abruptly, the HUD never took a shield
or the power bar, its 1000 ms holds and its reset's hold clear were missing,
the dead powers left the corpse's loot window and the released body's
inventory open, the AI search did not see an open inventory, the main menu's
and the power tree's tasks missed the focus loss, container popups kept
their opening unit, the inventory's mind window was never set, and a game
save did not write the open packs.

## Evidence

REA (Exanima.exe), checked against the assembly where constants or order
matter:

- The heartbeat's fade start (running flag test, clock, fade flag), the
  death fade's reset ending with it, the death and knock-out starting it
  first, and the border draw's fade arm (gain `0.3 · (1 − Δ/1000)`, stop
  under the GUI's threshold).
- The bars' and the gauge's show routines: the power-bar flag (set by the
  bar's opening toggle, cleared when its 900 ms close has run), the shield
  pointer and the combat byte, the gauge's stamina field.
- The hold setter's callers: the quick-bar cast entry (after the slot's
  index test) and the healing charge tick (player only).
- The per-character loot close (first loot-tagged form on the body) and its
  Raise Dead and Command Dead callers; the marks' end callback (root
  activation, the first visible inventory owned by the body closed).
- The AI search's open-container tests on its own, the corpse's and the
  container's ids.
- The inventory fill's mind-window flag (`is_bound` on the shown body) and
  the pack's drag permission (flag, pack source, the body's Volition bit,
  the hold break, then the close).
- The panels' save on a game save (the inventory's and the loot windows'
  list save).

## What changes

- `exa_ui::Heartbeat` gains the fade clock (`start_fade`, `fade_gain`,
  `stop`); `exa_game::hud` lowers the sound and stops the heartbeat; the
  flow's `Step::FadeHeartbeat` (death, knock-out) and the fade reset start
  it.
- `exa_ui::HudBars` gains `power_bar`; `exa_game::hud::pick_hud` fills the
  shield, its stamina and the bar flag; `hold_bars` serves the cast entry
  (`CastEntry::hold_bars`) and the healing charge; the HUD's reset clears
  the hold through `game_gui::reset_hud_fades`.
- `gui_screens::close_loot_on_body` (from `mind::runtime::raise_corpse`) and
  `gui_screens::release_marked_body` (from `MindAction::ReleaseScreens`).
- `npc_search::OpenSearchScreens` copies the GUI's `OpenContainers`.
- `deactivate_on_focus_loss` tells `MenuRouter` and `PowerRouter`.
- `GameScreens::rescale_open` rescales container popups.
- `GameScreens::inventory_mind_window` and `panel_live::drag_permission`.
- `body_inventory::store_pack` from `save_open_screens_for_saves`.

## Impact

- Lockstep: the HUD hold, the AI search's open check (an open inventory
  now counts), the save's carried-list order and the mind window's hold
  break change traces in scenes that use them; the heartbeat and the HUD
  draw are frame-only.
- Grid scenes owed while the GPU is on hold (see tasks).

## Why

Roadmap tasks B1/B4 (lane `arenaui`, port queue lane L13, UI menus and
arena screens). The arena's NEW CAREER skipped the company name and emblem
dialog (F446): the menu answered it with the arena creator, so the
dialog's sixteen routines (its opener, BACK, NEXT, their fade steps, the
form's constructor and paint, the emblem form's callbacks) ran only behind
a debug flag. Several smaller screen items were also open: the menu's back
cue and its drag rule, the version line over a show, the transition
screen's pointer, the divination reading's stream, mouse close and unit,
the loadout form's encumbrance dots, the tip banner's box and the
destination rows' words.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- NEW CAREER's show: the menu's close before a show (rows cleared, the
  game GUI hidden, the viewport's version-line flag cleared), then the
  company opener, the main form hidden and the session-state word 0x40.
- The opener: the dialog made, the last stored emblem record restored
  when its background word is above 0, the form refreshed and shown, the
  pointer moved to (W/2, round(370·f) + (H − 1024·f)/2), control 2 of the
  emblem form focused.
- BACK runs its step behind the transition (no music fade): the dialog is
  freed and the game returns to the title. NEXT refuses an empty name with
  the "Company Name" box (0xc1); otherwise it stores the name and the
  emblem record in two globals, starts character creation and runs its
  step behind the transition: the dialog is freed, the creator's reset
  and two releases run, and the session-state word becomes 2 (the
  creator's; the ledger's "game mode 2" was the session-state word).
- The menu's back handler (pages 1 and 8) plays MenuClick at pitch 1.0,
  gain 0.06, flags 0x4060, before the page shows. Its drag slot keeps the
  pressed row selected only while the pointer is on it (else selection
  0), stamping the clock once 200 ms have passed, the select cue only
  for a row.
- The transition screen's hit slot answers the form itself for every
  point.
- The divination reading deals from the run-time library's generator;
  its mouse slot closes on any word with bit 2 (the right button).
- The loadout form's view change writes the shown body's encumbrance
  squared times 10.0 into its control 9 (the inventory's dots read the same
  field unsquared).
- The tip banner's paint fills its box through the rounded program:
  corner 4, alpha 0xa0.
- The destination click hands the journey the destination word of the
  selected row's entry.

## What changes

- `exa_game::company_live` (new): NEW CAREER (`MenuCommand::CompanyDialog`)
  opens the hub forms' company dialog over the front end; its BACK and NEXT
  run behind the transition (`Action::CompanyBack`, `Action::CompanyNamed`,
  `Step::CloseCompany`); a named NEXT stores `CompanyDraft` and asks the
  creator with `MenuCommand::NewCareer`.
- `exa_game::option_forms`: `open_company_with` (restore, focus),
  `CompanyExit`.
- `exa_game::menu_screen`: the back cue, the drag-off selection.
- `exa_game::flow`: `takes_pointer`, the version-line clear on the menu's
  hide.
- `exa_game::div_card_screen`: the running stream, the mouse words, the
  unit.
- `exa_game::loadout_kit` and `hub_forms`: the encumbrance dots' value and
  draw.
- `exa_ui::game_tips`: the rounded box.
- `exa_game::destination`: the rows' destination words.

The creator's side (DEC-3) is not touched: the creator answers
`MenuCommand::NewCareer` as before.

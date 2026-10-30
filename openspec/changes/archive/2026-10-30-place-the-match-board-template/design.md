# Design

The placement rule lives in three small public functions in
`exa_ui::match_board` (`template_x`, `template_y`, `template_bottom_y`) so
`layout` derives each template control's rectangle from the template's own
numbers instead of hand-computed pixels. The template's zero heights use
`LABEL_HEIGHT` and `ITEM_SLOT_SIZE`, both labelled guesses, because the
class table that supplies defaults is filled at run time.

`TEMPLATE_CONTROLS` holds the template's initial state words; `Board::new`
starts from it. The day-end rule reads the board's own entries
(`has_entrants`, `has_unsettled_entries`), so the caller no longer passes a
verdict. Bottom-row clicks map to `BoardAction::{EndDay, Close, Forfeit}`;
`exa_game::hub_windows` keeps the closed board's `CloseState` and restores
it on the next opening.

The show keeps the original's two paths: a forced switch to the tournament
goes through the normal switch with the current tab set (so the old tab is
hidden); the re-open path clears the current tab and re-applies it.

## Why

Roadmap task B1 (wire the uncalled), family TTextBox, TGUIItemView and TGUIForm. The ui-07 audit found every live text box read-only, so the text box's key, press, drag, caret and scroll ports never ran, and the item view's draw, hit and keys had no live caller: the manual painted and hit its topic list with its own code. The findings list adds that the journal takes no typing (F497), does not close on a right release (F489), and that its New and Erase buttons are drawn but not wired (F33).

## What the original does

REA on Exanima.exe 0.9.5, checked in the assembly where order or constants matter:

- The journal's opener builds a text box child at (0, 88), 228 × 412, with no border, the stretched-down style bits 0xc0000, the vertical scroll bar attribute 0x200 and the small-font flag 0x2000000 (so not read-only). Its margin is the GUI unit, and its Tab callback turns combat mode on. The record table holds three records (the opener passes the last index, 2): New (add an entry), Erase (remove one) and a combo box with the entry titles.
- The journal's mouse slot closes the journal on a right release (0x2002). The text box hands a right-button word to its parent.
- The journal's select writes the list's caption and loads the entry's text into the box. New stores the box's text, appends "New Entry", selects it and focuses control 3. Erase removes the selected entry, or blanks the only one.
- The text box constructor ORs the style bits 0x8c (the caret bit 4 among them). The base tick flips the caret-lit bit every fifth frame, and a press or a key restarts the blink. The draw shows the caret only while the lit bit is set.
- The manual's open builds the topic list as an item view (style 0x9100000) with a select callback. The callback shows the topic and focuses the page with the control −1. The page is a read-only text box with markup and a margin of 8. The show rescales both from the design unit 4, so the list's icon becomes four units (rows of five units: 30 pixels at 1080 lines, as retail shows) and the page's margin two units.
- The form's press stores the held record as its index plus one, and only when the record's answer has both bits of 5; the release compares that word with the record count.
- The item view's press, key and command slots call the select callback on a selection change. The window manager sends a press, then the click (0x2000) on the release, and focuses a form whose answer has 0x10. The focus follows the pointer.

## What Changes

- The journal's text box is live (`exa_game::gui_screens::JournalBox`): a held form under the journal in the window manager, with the faithful `TextBox`. While the focus is on it, the keys go to the box's key slot and the hotkeys do not run. A press places the caret, a drag selects, and the scroll bar and the wheel scroll it. The caret blinks through `form::caret_blink`, and Tab turns combat mode on.
- The journal's records gain the entry list (painted with the selected title). New and Erase run `Journal::add_entry` and `delete_entry` and reselect. A right release over the journal closes it.
- The manual's topic list is the item view (`Manual::shown_list`): hover, click and keys go through its slots, and the list's callback focuses the page. The page is the faithful text box (`help_system::page_text_box`), kept between frames, with its scroll eased by the box.
- The base form's press holds a record by its 1-based index when the record answers 5, as the original's press stores it (it kept the hit code with the answer's bits, which the release's count check refused, so no record's release ever reached its callback). The release and the drag compare the hit code's low word with it.
- The hand-written list rows (`help_live::list_row`), the arrow stepping and the hand ease are removed.

## Impact

- `exa_ui::base_form`, `exa_ui::game_gui`, `exa_ui::help_system`, `exa_game::gui_screens`, `exa_game::help_live`, `exa_game::hotkeys`, `exa_game::gui_paint`.
- Look: the journal's text box now draws its panel frame and scroll bar, writes in the small font and shows the entry list. The manual's rows are five units apart with the item view's selection draw. Both follow the original's words.
- Lockstep: GUI only. Tab in the journal pushes the flow's combat-mode step, as the Tab hotkey's does.

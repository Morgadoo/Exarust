# Design

The flow keeps returning named steps; the review corrects what those steps
say rather than how they run.

- The story start's record writes are expressed in the record's own terms:
  `ItemRef` for the off hand and `Carried` for the carried entry (from
  `exa_formats::characters`), so a host can apply them to a
  `CharacterRecord` directly. The power word and value reuse
  `create::creation_defaults` instead of new constants.
- Two new steps name the form flags the review identified:
  `EndMenuUnselected` (the end menu's no-row-selected flag, read by the
  menus' shared hit test) and `TitleLogo` (the main-menu form's logo flag).
  The main menu's help flag keeps `MenuHelp`; the duplicate
  `clear_help_flag` is dropped for the existing `help_closed`.
- The arena pause label is folded into `page_rows` although the original
  relabels the row just after the page is built: the visible result is the
  same.

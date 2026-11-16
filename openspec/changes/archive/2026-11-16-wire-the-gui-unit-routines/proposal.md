## Why

Roadmap tasks B1 and B4 (lane `guimain`): the GUI main unit's open rows
(RF_GUI_Main) and the key routes the guikeys lane left on Bevy's keys: the
hub's message box and window close, the save screen, the manual's Escape,
the end menu (which shares the main menu's key slot) and the arsenal view.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The menu toggle branches on the session-phase word: 0x40 the class
  select's cancel, 0x20 the arena unit's close of the loadout form (the
  form its constructor stores; the phase its show and commit write), 2 the
  creation form's cancel (it schedules the menu), 1 the menu's back
  (pages 1 and 8 only), 3/4/5/8 the in-game menu's show or hide, whose hide
  also closes the settings form and the manual.
- The message box's key slot (slot 37) passes over a word with bit 2,
  closes on 0x0d and 0x20, closes with -1 on 0x1b and calls no parent. No
  caller passes the Cancel flag (callers pass 0x41, 0xc1, 0x60), and the
  close-mark handler is never referenced.
- The arsenal view's key slot passes over word bit 2, closes on 0x1b,
  flips the inventory on the bound inventory key word and calls no parent.
- The item view's key slot sends a key it does not take (Escape) to its
  parent's key slot.
- The level switch sets the desk's drag distance from the screen height
  on every call, then fills the large atlas from `GlphCommon`: 19 copies
  of 64-pixel squares at fixed corners, each added with id 1.
- The inventory and journal openers clamp the new window (x within the
  root's width, y at least the unit).
- The pad's leave and enter checks read the selected record's state bit
  0x400 and style bit 0x800000.

## What changes

The hub's message box, the hub's forms, the save screen, the manual, the
end menu and the arsenal read the frame's key list; the menu toggle
branches on the phase; the large atlas takes the common glyph sheet; the
drag distance follows the window; new inventory and journal windows are
clamped; record text goes through the text setter; base forms answer the
pad's focus checks.

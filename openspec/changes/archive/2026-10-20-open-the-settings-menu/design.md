# Design

## Behaviour

The global settings pointer at `0x4f0170` decides the branch. When it is null the form is created with `0x100128220` (class `0x312ed0`), stored, then attached with `0x100029470` and `0x100029660`. When it exists the form is re-shown with `0x10002c8f0` and `0x10002ae50`. Both branches call the layout `0x10022c740` with zeros, set the entry at `0x6c` to 1 when the byte at `0x49a300` is set, and finish with `0x100029aa0` and −1.

The close helper calls `0x100128d30` only when the pointer exists. The reset helper writes 0 to the pointer's byte at `0x218` only when it exists.

## Model

`open_settings_menu(exists, select)` returns `SettingsOpen { created, shown, laid_out, selected, finished }`. `close_settings_menu(exists)` and `reset_settings_menu(exists)` return whether they acted.

## Open questions

The exact meaning of the attach calls does not change the branch.

## Docs

No published percentage moves. The headline stays 14.4% and 5.7%.

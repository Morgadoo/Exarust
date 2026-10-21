# Design

## Behaviour

The global settings pointer decides the branch. When it is null the form is created from its class, stored, then attached with the attach pair. When it exists the form is re-shown with the re-show pair. Both branches call the layout with zeros, set entry 1 when the resume flag byte is set, and finish with −1.

The close helper calls the form's close only when the pointer exists. The reset helper clears the form's flag byte only when it exists.

## Model

`open_settings_menu(exists, select)` returns `SettingsOpen { created, shown, laid_out, selected, finished }`. `close_settings_menu(exists)` and `reset_settings_menu(exists)` return whether they acted.

## Open questions

The exact meaning of the attach calls does not change the branch.

## Docs

No published percentage moves. The headline stays 14.4% and 5.7%.

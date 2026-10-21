# Design

## Behaviour

The function reads the menu's stored page. Page 1 calls the action with 0; page 8 calls it with 1. Both then call the session entry with 1. The input-readiness global gates the sound. Any other page falls through without a sound, an action or the session call.

## Model

`forward_main_menu(page, input_active)` returns `MenuForward { action, sound, session }`: the action is `Some(0)` on page 1, `Some(1)` on page 8 and `None` otherwise; the sound is the action's presence AND the input; the session flag is the action's presence.

## Open questions

The page meanings (resume and quit, by the labels around them) do not change the routing.

## Docs

`TMainMenu` is not separately rated, so no published percentage moves. The headline stays 14.4% and 5.7%.

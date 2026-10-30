## Why

The menu's CONTINUE rows (screen mode 12) logged "not wired", and a story save could be written but never loaded into the running game: Exarust's own story saves carried no player record, so nothing could be restored from them.

Phase 7 (game modes and menus) in docs/PLAN.md.

## What Changes

- The CONTINUE rows open the saves' character screen: the creation form's continue table (eleven records), the slot combo box with its list, Previous, Next, Delete with the `Delete Character` box, the story's checkpoint toggle, BACK and CONTINUE, the slot's character in the preview, its items and journal; drawn through the GUI path with the shown palette and the line skin.
- The callbacks are the original's select, refresh, step, pick, toggle, delete and continue; the manager's character-screen select (`SlotList::character_screen_select`) answers the availability and reads the slot with mask 0x100010.
- A story save writes the player's record into `#worldchrdb` and its placement into the player data; CONTINUE runs the story's full load: the record becomes the next player (body, skin, hair, name, worn and held items, stats), the world clock takes the saved seconds, and the story continues at the save's level and placement. The arena continues at the hub with the slot.
- The flow's save, drop and checkpoint steps run on the current story slot, so a story death drops the save and the slot comes back from its checkpoint.
- `--open-saves story|arena` and `--load-slot N[:save|:checkpoint]` drive it for scripted checks.

## Evidence

The creation form's constructor and its continue table, the continue callbacks, the delete box's constructor and table, the character screen's entry and refresh, the save manager's character-screen select and full load, read with REA in the charcreate and savegames packs.

## Out of scope

The save's level section and world databases beyond the player, the skills panel, item icons in the container, the reset-after-the-portal question.

## Why

Roadmap task B4 (lane `savegames2`, docs/roadmap/phase-b-close.md),
finding F543 and the save unit's remaining partial rows. The flow's
`Step::ClearStoryFlag` was never applied, so the story respawn and exit
never left the dead player's body in the level: the strip undressed the
live body, the level save left the player out, and a later visit found
nothing. Door and respawn loads always read the archive even when the
save held the level, the session's arrival words were set only for a
respawn, and every story level took the base indoor fog.

## Evidence

REA (Exanima.exe), read in the assembly:

- The death path strips the player's character: an unshared record is
  copied (written and read back), the copy takes `0x40` in its profession
  word, its role count is cleared and roles 1 and `0xC2` added, its yellow
  and red pool words are zeroed, it joins the world character database
  under the first free id and becomes the body's record. The record the
  player keeps loses its four hands, its worn count (the underwear rule
  follows) and its carried count, and its pool words become `0x7FFF`.
- The respawn and a stripped player's exit `and` the body's flag word with
  `0xFFFFEFFF` before the save; the exporter skips a node with `0x1000`;
  the story writer swaps the current player record in for the placement.
- The transition opens the save package, looks the destination's section
  up by name and imports it, the archive only when that failed; the
  respawn's loader prefers the save as well.
- The arrival word starts at all ones; a level exit stores its entrance,
  the transition stores the previous level and clears the word after the
  import; the respawn sets the session-start word and clears it after its
  load; the story reader sets it only for reconstruction state 2; every
  other load stores no previous level.
- The experience unit's level-enter hook picks one of the indoor preset's
  three fog variants by the locale name's bytes.
- The GUI reset's flag byte is the story's first-wake flag, and the crystal
  source clear sets the source count to 0 (bytes decoded at the routine's
  start).

## What Changes

- `exa_game::left_behind` (new): `leave_behind` (the strip),
  `LeftBody` (the flow's record of the body), `forget_on_level_change`.
- `exa_game::flow`: `Step::StripPlayer` runs the record strip (the body
  stays dressed); `Step::ClearStoryFlag` marks the body for the save.
- `exa_game::save_live`: the story save writes the stripped record as the
  player and the body into the level section; `prefer_saved_section`
  hands the save's section to every story level change.
- `exa_game::level_save`: `LevelSaveScene::left_character`.
- `exa_game::level_change`: `SessionArrival`, `ArrivalLoad`; the host's
  loads take their arrival words from it.
- `exa_world::level_lifecycle::LevelChangeReady::door`.
- `exa_world::environment::story_level_fog`, applied by the load.
- `exa_world::teleporters::Record::carried` (was a misread action list).

## Out of scope

- The header camera pair (F415): the view's target and zoom fields are not
  traced onto the chase camera (another lane's component).
- The menu scene load, the grid environment set-up, the GUI reset's
  first-wake clear (the story start sets the flag after its load; Exarust
  sets it before) and the arena's reload and hub return (arena unit).

## Why

The player controls and the player behaviour (the SG_PlayerControls unit
routines and TPlayerBehaviour) were partly in Exarust: the steering, the
footwork, the hand's drags, the reach, the idle camera and the light. The
original also walks to the cursor in combat mode, eases a gamepad's aim
cursor, keeps the hand busy through a door, wheel or cloth drag, gates the
idle camera by blocking screens and open windows, and keeps aim, contact,
timing and item-use rules that the running game will need.

Phase 3 (Bodies: skeleton, physics, control), the player-control lane
(playerctl); the master plan's M51 (the behaviour's aim, contacts and
auto-target) in part.

## What Changes

- Combat mode: the right button walks to the cursor as the original's
  combat branch does while it is held (the sprint latch kept, the run beyond
  3 m or by the Run latch, the look-ahead along the facing cutting the free
  walk and, within 100 cm, the run), the footwork yielding; the cursor shows
  the chevrons meanwhile (`steering::Steering::{combat_request,
  combat_walk}`).
- The gamepad's aim point `250·|aim| + 50` cm out eases by
  `(0.4·AimSpeed + 0.1)²` each behaviour tick from the mouse's point
  (`player::AimCursor`); the combat aim follows it.
- A press on a door, a wheel or cloth holds the hand until the release
  without keeping the movement request (`hand::Holding::Mechanism`).
- The idle camera's gates: a blocking screen turns it with only the
  ten-second test, an open window (the in-game menu) restamps the clock, a
  dialogue or the power bar keeps the player busy (`idle_orbit::Frame`).
- Tested rules with no caller yet: the aim point's bone lock and raised
  surface, the screen bone fraction, the 12 m contact list, the strongest
  engaged enemy, the screen-nearest character, the attack wind-up and
  recovery (`player_behaviour`); a pour's warmth, the cleaning and repair
  kits' targets and the timed list (`player_controls`); the start and end
  of play's activation centre (`PlaySession`); the grabbed box's limits
  (`hand::box_ok`); the light's second off rule
  (`player_light::weapon_puts_out`).
- Review of the lane's work: its heading follower, heading snap, body step,
  combat target and stance helper duplicated (wrongly) the steering's cursor
  follower and `exa_combat::stance`; its scene-node words, item copies, GUI
  item operations, script hooks, input gates, game modes, level attach,
  click and double-click handlers, auto-walk and reach region were invented
  or misread: all removed. The entering-combat cancel and the reach latch
  stay as reviewed before.

Evidence:
- The pack `context/re/packs/playerctl` (assembly of the 153 functions):
  the behaviour tick's combat branch, the aim cursor, the aim point and its
  bone helper, the contacts, the engaged test, the screen-nearest search, the
  attack timing, the controls' frame and blocked turn, the per-frame request,
  the use routines and timed list, the start and end of play, the equipment
  light switch; REA for the angle wrap and sign helpers and the GUI's mouse
  events. Notes in `context/re/player-control-hand.md`.

Out of scope:
- the click and double-click handlers' windows (item card, narrator,
  inventory, loot, talk) and the item cursor;
- the reach's grid-environment room and mesh-blocker tests;
- door, wheel and cloth bodies under the hand;
- the camera yaw follow behind the "pin the cursor" option;
- wiring the bone lock, contacts, engaged enemy, attack timing and item use
  into the running game.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: the combat walk, the pad aim ease, the mechanism hold,
  the idle camera's gates, and the behaviour's and controls' tested rules.

## Impact

- `exa_game`: `steering`, `player`, `cursor`, `hand`, `idle_orbit`,
  `player_light`, new `player_behaviour` and `player_controls`
- `docs/PLAYER.md`, `docs/UI.md`, `crates/exa_game/README.md`

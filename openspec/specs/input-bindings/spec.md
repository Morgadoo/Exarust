# input-bindings Specification

## Purpose
The player's key bindings: the original's 23-slot virtual-key table, how its modifier keys are read, and the file it is saved in.

## Requirements

### Requirement: Default bindings
The system SHALL provide the original's 23-slot virtual-key table: primary (left mouse), W, A, S, D, X, shift, alt, middle mouse, V, control, I, tab, K, R, F, J, T, C, P, Q, right arrow and left arrow. Shift, alt and control SHALL accept either side.

#### Scenario: Shipped table
- **WHEN** a player starts with default bindings
- **THEN** Cursor Look is V and Run is either shift

#### Scenario: Unbound key
- **WHEN** an action's code has no mapping
- **THEN** the action stays inactive

### Requirement: Binding file
The system SHALL save bindings as version 101, the controller-mode flag, the count 23 and the 23 little-endian rows of the saved table in the Controls page's order: scan codes (bit `0x100` when extended) and `0x8000` plus a mouse button's virtual-key code, row 21 left 0. Loading SHALL read the flag and then the count when the version is above 100, else the count with the flag off, and SHALL let the count's rows overwrite the shipped saved table from its first row.

#### Scenario: Round trip
- **WHEN** a controller binding set is saved and loaded
- **THEN** the controller flag and every slot's key are unchanged, except that a right control or alt reads as the left one

#### Scenario: Old file
- **WHEN** a file of version 100 holds the count after the version
- **THEN** it loads with controller mode off and the rows that follow

#### Scenario: Malformed file
- **WHEN** a file's count is 2, or the file ends before its rows
- **THEN** the first loads two rows over the shipped ones and the second loads no bindings, as no payload that ends before a word it needs loads

### Requirement: Saved rows to slots
After loading, the system SHALL clear row 21 and fill each slot from its row: a code with a `0xC000` bit gives its low 14 bits, any other its low byte's scan code mapped to a virtual-key code, the modifiers to their left keys' one-sided codes. Slot 0 SHALL keep the primary mouse button.

#### Scenario: Shipped rows
- **WHEN** the shipped saved table is loaded
- **THEN** every slot holds the shipped table's key, the modifiers their left keys' own codes

### Requirement: Gamepad action sets
The system SHALL read a connected gamepad, without linking Steam, through the original's menu set and its interact and combat gameplay sets, each action keeping its manifest name and kind and mapping to a documented default gamepad control. `Inventory` and `Skills` SHALL only be read in the interact set, `CrouchC` and `AltAction` only in the combat set.

#### Scenario: Interact-only actions
- **WHEN** the combat set is active and the inventory button is pressed
- **THEN** no inventory command is produced

#### Scenario: Shared actions
- **WHEN** the combat set is active and the move stick is pushed
- **THEN** it is read as `MoveC`, with the same default control as `MoveI`

### Requirement: Gamepad set layer
The gameplay set the player's combat mode chooses SHALL only be active while no menu form has the focus, a focused form activating the menus or the set it names, and the last form leaving SHALL bring the chosen gameplay set back and treat its held buttons as not pressed.

#### Scenario: Combat mode under a form
- **WHEN** combat mode turns on while a menu form has the focus, then the form closes
- **THEN** the menus stay active until the form closes, and then the combat set is active

### Requirement: Gamepad menu navigation
While a form has the focus, the navigation stick SHALL send a direction once its squared length is over 0.25 (vertical unless the horizontal axis is strictly larger; up or right only for a positive axis), at once, 400 ms later and then every 150 ms. Select, forward and back SHALL be sent only on their first pressed frame, and for three milliseconds after a set is activated input SHALL be ignored and held buttons treated as not pressed.

#### Scenario: Held up
- **WHEN** the navigation stick is held up past the dead zone
- **THEN** up is sent at once, again 400 ms later and then every 150 ms

#### Scenario: Held select
- **WHEN** select is held over several frames
- **THEN** it is sent on the first frame only

### Requirement: Gamepad menu cursor
Whenever a menu stick's squared length is over 0.1 the reader SHALL send its analog bit and its cursor scaled by a sixteenth of the frame's milliseconds, every frame; the scroll stick SHALL only be read while the navigation stick is inside 0.1.

#### Scenario: A gentle push
- **WHEN** the navigation stick is held at 0.4 to the right
- **THEN** each frame sends the analog bit with the scaled cursor and no direction

### Requirement: Gamepad menu pick
A directional pick at GUI unit `u` SHALL choose, from the current control's centre, a control whose offset along the stick's dominant axis times the normalised stick part is at least `u`: vertically the smallest rounded vertical distance within `80·u` pixels sideways; horizontally, with the vertical offset stretched eightfold and an alignment of at least 0.3, the smallest length over alignment.

#### Scenario: Too close
- **WHEN** the stick points down and the only control lies 3 pixels below at unit 4
- **THEN** nothing is picked; at 4 pixels it is

### Requirement: Gamepad movement and actions
In controller mode the pad SHALL move the player by the move stick in the camera's frame, drive the behaviour's screen cursor from the aim stick once its squared length is over 0.01, stab on `AltAction`, crouch on `CrouchC`, sprint on `Sprint` and swap weapons on `WepSwp`. Keyboard and mouse input SHALL keep working unchanged.

#### Scenario: No gamepad
- **WHEN** no gamepad is connected
- **THEN** the player is driven by the keyboard and mouse exactly as before

#### Scenario: Sprint
- **WHEN** `Sprint` is held in controller mode
- **THEN** the player sprints

### Requirement: Gamepad attack
The pad SHALL attack on a stand-in for Steam's emulated left mouse button (the right trigger, outside the action sets), as the left mouse button does: held, it holds the attack request, and its press is latched for the next tick. `Combat` SHALL NOT attack.

#### Scenario: Attack trigger
- **WHEN** the emulated left button is pressed in combat mode
- **THEN** an attack is requested as with the left mouse button

### Requirement: Gamepad camera
Past its 0.5 vertical and squared 0.1 horizontal dead zones, and with a squared pair over 0.01, the camera stick SHALL turn the camera by the orbit's rate per screen pixel times `(x·30, y·10)`: the yaw opposite to `x`, the pitch rising with `y` from the held pitch within its limits. Each frame `Zoom_In` is held SHALL lower the zoom by ten 1/120 steps of a fifth of its range, and `Zoom_Out` raise it, each held in the range.

#### Scenario: Full camera stick right
- **WHEN** the camera stick is held fully right on a 1920-pixel-wide window
- **THEN** the camera's yaw turns 0.075 rad a frame, as a rightward mouse orbit does

#### Scenario: Zooming in
- **WHEN** `Zoom_In` is held for one frame at zoom 1300 on the 800–2000 range
- **THEN** the zoom becomes 1280

### Requirement: Gamepad command word
Newly pressed gameplay buttons SHALL enter the player's command word once, `Look` and `CamMode` SHALL be held bits, and `Powers` held for more than 500 ms SHALL also set the power-tree command.

#### Scenario: Held powers
- **WHEN** `Powers` is held for 501 ms
- **THEN** the power-tree command is added

### Requirement: Gamepad commands
The pad's newly pressed commands SHALL act each frame, as the player controls' pad routine orders and gates them:
- `Powers` SHALL toggle the power bar only when the player has a power of its own;
- with the bar closed, `UsePower` SHALL use the picked power when powers are allowed;
- with the bar open, `UsePower` SHALL use it whatever the mask, or else a step right or left SHALL move the pick;
- `pause_menu` SHALL toggle the menu as F1 does.

The commands SHALL not test death.

#### Scenario: Inventory from the pad
- **WHEN** `Inventory` is pressed in the interact set during play
- **THEN** the inventory opens

#### Scenario: The bar's steps
- **WHEN** a step right and `UsePower` are pressed together with the bar open
- **THEN** the picked power is used and the pick does not move

#### Scenario: Pause
- **WHEN** `pause_menu` is pressed in play
- **THEN** the menu opens

### Requirement: Gamepad combat toggle
`Combat` SHALL toggle combat mode as the pad routine's own command, with no gate, after the power tree and before the weapon swap.

#### Scenario: Combat toggle
- **WHEN** `Combat` is pressed in play out of combat mode, and later pressed again
- **THEN** combat mode turns on and then off, and no attack is requested

### Requirement: Gamepad rumble
A hit SHALL raise both rumble motors by its amount, each held at 1, and each frame SHALL keep 0.6 of each (in double precision) and form 16-bit motor speeds (×65535, rounded half to even); as in the original, no vibration SHALL be sent.

#### Scenario: Decay
- **WHEN** both motors are at 1 and a frame passes
- **THEN** each is 0.6 and its speed word is 39321, and nothing is sent to the pad

### Requirement: Gamepad screen commands
`PowerTree` SHALL ask for the power tree when powers are allowed and the player has one; `WepSwp` and `Inventory` SHALL keep the key handler's gates; `Skills` SHALL ask for the skill screen with no permission test.

#### Scenario: The pad opens the inventory
- **WHEN** the pad's Inventory command is newly pressed while the player stands
- **THEN** the inventory screen opens as the key would open it

### Requirement: Gamepad focus stack follows the forms
A live form that shows (main menu, Settings, manual, skill screen, arsenal, hub window) SHALL push itself on the focus stack and activate the menu set; one that hides SHALL pop itself, and the last pop SHALL restore the gameplay layer. While the menu set is active the gameplay actions SHALL read as released.

#### Scenario: Menu open
- **WHEN** the in-game menu opens and Talk is pressed
- **THEN** the menu set is active and the gameplay reader sees no Talk

### Requirement: Gamepad menu words reach the top form
Each frame's menu key word SHALL go to the focus stack's top form only; `MenuExit` SHALL run the main menu's toggle over whatever form has the focus.

#### Scenario: Exit hook
- **WHEN** the menu is open and MenuExit is pressed
- **THEN** the menu closes

### Requirement: Gamepad unanswered words
A word the form's key slot leaves unanswered SHALL go whole to a focused control that asks for the keys; otherwise `MenuBak` SHALL go to the form's back slot when its attribute word has bit 2, a navigation step to the directional pick, and any other word, masked with 0xf322, to the focused control's key message.

#### Scenario: Settings back
- **WHEN** the Settings form has the focus and MenuBak is pressed
- **THEN** the form cancels and closes, leaving the menu on top

### Requirement: Gamepad pick of self-drawn forms
The directional pick SHALL place a form that draws its own items through its centre slot: a table view at its client's centre, selecting its first row when it has no selection.

#### Scenario: Board table picked
- **WHEN** the board is open and the navigation stick steps down
- **THEN** a visible table takes the pad's focus

### Requirement: Gamepad table commands
The focused table view SHALL take the remaining words through its command slot: a scroll step SHALL move its selection, and the company and lineup tables' activation words (the scroll stick right or left) SHALL move the selected fighter across as a double press does.

#### Scenario: Fighter joins
- **WHEN** the company table has the focus, a fighter is selected and the scroll stick is pushed right
- **THEN** the fighter joins the lineup

### Requirement: Gamepad skill link search
The skill screen's link search SHALL read the offset's `y` after normalising it, so a sideways step keeps the current row: the point's `y` moves by the normalised `y` times 0.1 beside the hit level and 0.25 in the scan.

#### Scenario: Sideways step
- **WHEN** the stick points right and the adjacent satellite lies up and to the right
- **THEN** that satellite is picked and the point's row stays

### Requirement: Gamepad look buttons
The pad's `Look` and `CamMode` buttons SHALL start and hold the controls' mouse capture as Cursor Look's key and the orbit's button do, and while the capture holds the aim stick's pair SHALL be stored unturned.

#### Scenario: Look button
- **WHEN** `Look` is held in controller mode
- **THEN** Cursor Look's capture starts

### Requirement: Saved rows kept as captured
The bindings SHALL keep the saved table's rows as read or captured and save them unchanged, with the controller word the file was read with.

#### Scenario: Right control captured
- **WHEN** right control is captured for a row and the bindings are saved
- **THEN** the row is saved as the extended scan code 0x11D while the slot reads the left control

### Requirement: Key cells named from the rows
The Controls page SHALL name each key cell from its saved row, keeping the extended bit, with an empty caption for the primary and secondary mouse buttons.

#### Scenario: Left mouse row
- **WHEN** a row holds the primary mouse button
- **THEN** its cell's caption is empty

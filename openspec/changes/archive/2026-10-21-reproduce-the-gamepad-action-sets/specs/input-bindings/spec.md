# Spec Delta

## ADDED Requirements

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
In controller mode the pad SHALL move the player by the move stick in the camera's frame, set the combat aim from the aim stick once its squared length is over 0.01, attack on the combat button, stab on `AltAction`, crouch on `CrouchC`, sprint on `Sprint` and swap weapons on `WepSwp`. Keyboard and mouse input SHALL keep working unchanged.

#### Scenario: No gamepad
- **WHEN** no gamepad is connected
- **THEN** the player is driven by the keyboard and mouse exactly as before

#### Scenario: Attack trigger
- **WHEN** the combat button is pressed in combat mode
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

### Requirement: Gamepad rumble
A hit SHALL raise both rumble motors by its amount, each held at 1, and each frame SHALL keep 0.6 of each (in double precision) and form 16-bit motor speeds (×65535, rounded half to even); as in the original, no vibration SHALL be sent.

#### Scenario: Decay
- **WHEN** both motors are at 1 and a frame passes
- **THEN** each is 0.6 and its speed word is 39321, and nothing is sent to the pad

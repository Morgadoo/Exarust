# Spec Delta

## ADDED Requirements

### Requirement: Gamepad combat toggle
`Combat` SHALL toggle combat mode as the pad routine's own command, with no gate, after the power tree and before the weapon swap.

#### Scenario: Combat toggle
- **WHEN** `Combat` is pressed in play out of combat mode, and later pressed again
- **THEN** combat mode turns on and then off, and no attack is requested

### Requirement: Gamepad attack
The pad SHALL attack on a stand-in for Steam's emulated left mouse button (the right trigger, outside the action sets), as the left mouse button does: held, it holds the attack request, and its press is latched for the next tick. `Combat` SHALL NOT attack.

#### Scenario: Attack trigger
- **WHEN** the emulated left button is pressed in combat mode
- **THEN** an attack is requested as with the left mouse button

## MODIFIED Requirements

### Requirement: Gamepad movement and actions
In controller mode the pad SHALL move the player by the move stick in the camera's frame, set the combat aim from the aim stick once its squared length is over 0.01, stab on `AltAction`, crouch on `CrouchC`, sprint on `Sprint` and swap weapons on `WepSwp`. Keyboard and mouse input SHALL keep working unchanged.

#### Scenario: No gamepad
- **WHEN** no gamepad is connected
- **THEN** the player is driven by the keyboard and mouse exactly as before

#### Scenario: Sprint
- **WHEN** `Sprint` is held in controller mode
- **THEN** the player sprints

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

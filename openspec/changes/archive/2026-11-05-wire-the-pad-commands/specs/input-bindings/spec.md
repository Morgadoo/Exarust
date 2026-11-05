# Spec Delta

## ADDED Requirements

### Requirement: Gamepad commands
The pad's newly pressed commands SHALL act each frame, as the player controls' pad routine orders and gates them:
- `Powers` SHALL toggle the power bar only when the player has a power of its own;
- with the bar closed, `UsePower` SHALL use the picked power when powers are allowed;
- with the bar open, `UsePower` SHALL use it whatever the mask, or else a step right or left SHALL move the pick;
- `PowerTree` SHALL ask for the tree when powers are allowed and the player has one;
- `WepSwp` and `Inventory` SHALL keep the key handler's gates;
- `Skills` SHALL ask for the skill screen with no permission test;
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

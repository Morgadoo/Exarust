# Spec Delta

## ADDED Requirements

### Requirement: Hotkeys by state
A dead player's keys SHALL only end an open dialogue. With the power bar open a digit SHALL pick a power ('0' the tenth), the use key SHALL use the picked one and the arrows SHALL step through them; with a dialogue open '1' to '9' SHALL choose an answer and the Talk key SHALL end it; with neither the Talk key SHALL talk when talking is allowed.

#### Scenario: Dead, talking, the bar
- **WHEN** a dead player presses Tab, then with a dialogue open; a talking player presses 1, 9, 0 and T; the power bar is open and 0, 2, Q and the arrows are pressed
- **THEN** nothing, then the dialogue ends; answers 0 and 8, nothing, the end; powers 9 and 1, the use, a step forward and back

### Requirement: Play hotkeys
Any other key SHALL then be tested in this order: use the picked power (powers allowed), toggle combat mode, the inventory (allowed, and in the modes hiding it only while collapsing), the weapon swap (allowed), the held light, the skills and the journal (allowed and not hidden), the power tree (powers allowed and some), and the power bar (the player's or else the companion's powers).

#### Scenario: The play keys and their gates
- **WHEN** Tab, I, R, F, K, J, T, Q, P and C are pressed in play with no powers, then P and C with two powers, then C with a companion's power, then the window keys with no permission
- **THEN** combat, the inventory, the swap, the light, the skills, the journal, talking, the use and nothing twice; the tree and the bar; the bar; and nothing for each window key while Tab and F still act

### Requirement: Entering combat mode
Entering combat mode SHALL let go of what the hand holds and clear the request's backpedal, keys' heading and turn-about bits. Tab SHALL toggle a living player's combat mode in the running game.

#### Scenario: Letting go
- **WHEN** combat mode is entered with a drag in hand and the request carrying move, backpedal, keys' heading, turn-about and run, and Tab is pressed by a living and by a dead player
- **THEN** the drag is let go, the request keeps move and run, the living player's combat mode turns on and the dead one's does not

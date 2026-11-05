# Spec Delta

## ADDED Requirements

### Requirement: Form base input
The game's windows SHALL take the pointer through the form base class's rules over the line skin's frame codes: a press on the title band drags a window, a release over its close mark closes it, and a pressed window comes to the front by the layer rule.

#### Scenario: Frame codes
- **WHEN** the pointer is off a form's client
- **THEN** the close corner and the title band four units down, and right of the client the upper arrow, the scroll rail and the lower arrow, answer it

#### Scenario: Moving the inventory
- **WHEN** the pointer presses the inventory's title band and moves 400 pixels right and 50 down before letting go
- **THEN** the inventory, its view, pack and records are 400 pixels right and 50 down and stay there until it closes

#### Scenario: Closing by the close mark
- **WHEN** the left button is pressed and released over a window's close mark
- **THEN** the window closes

#### Scenario: Raising
- **WHEN** a press on its title holds a window that is under another
- **THEN** that window draws above the other

### Requirement: Scroll bar input
A form with the vertical scroll bar SHALL scroll by its arrows, its rail and thumb, and the wheel, which goes to the focused form and moves it by whole notches of its step.

#### Scenario: Thumb
- **WHEN** the rail is pressed off the thumb and dragged
- **THEN** the thumb centres on the pointer and follows it, a snapping form keeping its fraction on its steps

#### Scenario: Held arrow
- **WHEN** the manual page's lower arrow is held
- **THEN** the page scrolls a line at the press, the next line about 700 ms later, then a line each tick until its end

#### Scenario: Wheel
- **WHEN** the wheel turns three notches down over the manual's page
- **THEN** the page's fraction moves three of its steps toward the end

### Requirement: Frame hover inks
The line skin SHALL draw the frame mark under the pointer in 255, 204, 153 at the GUI's alpha: the scroll bar's upper arrow, lower arrow or thumb in place of their idle ink, and the close mark drawn again after the frame.

#### Scenario: Hovered close mark
- **WHEN** the pointer rests on the inventory's close mark
- **THEN** the close mark draws in the hover ink after the frame

### Requirement: Default tooltip
A hovered record that asks for its tooltip SHALL show the default tooltip once the pointer has rested 400 clock units on it: a button shows its record's tooltip string and asks only when it has one, any other record its text; the box is the text's width plus four units, fades in over 256 units, stays 2500 and fades out, and a press hides it.

#### Scenario: Captioned buttons
- **WHEN** the pointer rests on the inventory's Primary button, which has no tooltip string
- **THEN** no tooltip shows

### Requirement: Title client top
A titled top-level form whose attribute word's 0x800030 bits are exactly 0x20 SHALL start its client at two units plus the border metric 2 under the line skin; the inventory's character view and records SHALL sit that far below its top, its pack staying at the client's bottom.

#### Scenario: Inventory at unit 4
- **WHEN** the inventory is laid out at unit 4
- **THEN** its client starts 10 pixels down, its character view at (8, 10) and its pack at (8, 456)

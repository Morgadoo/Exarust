# Spec Delta

## ADDED Requirements

### Requirement: Message box shown
A message box SHALL be shown on top of the hub's form stack with its title as the caption, its text in a label and its buttons, centred across the screen and placed down at its design-pixel top rescaled to the GUI unit, and placed again after a resize. Enter or Space SHALL close it, Escape SHALL close it with the result -1, Ok SHALL close it with 1 and Cancel with 0; while it is on top no other key or right release SHALL close a window under it.

#### Scenario: A hire without the money
- **WHEN** Hire is pressed on the hire page for a candidate the company cannot afford
- **THEN** a "Hire" box reading "Insufficient funds" with one Ok button shows over the loadout form, and Escape closes only the box

#### Scenario: A company without a name
- **WHEN** NEXT is pressed on the company dialog with the name field empty
- **THEN** a "Company Name" box reading "Please name your company" shows and the dialog stays open

### Requirement: Pointer position starts unknown
The GUI pointer's stored position SHALL start unknown on both axes, so that the first pointer read always counts as a move.

#### Scenario: First read at the origin
- **WHEN** the first pointer read gives (0, 0)
- **THEN** it is taken as a move to (0, 0)

### Requirement: Viewport action removal
Removing a viewport cursor action SHALL remove every registration whose handler code matches, whatever object it was registered for, and SHALL NOT match on the object alone.

#### Scenario: Two objects, one handler
- **WHEN** one handler is registered for two objects and another handler for the first, and the first handler is removed
- **THEN** only the other handler's registration remains

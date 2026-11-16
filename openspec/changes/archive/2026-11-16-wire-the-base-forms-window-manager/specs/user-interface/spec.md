## ADDED Requirements

### Requirement: Window layer insertion
The window manager SHALL add a form among its parent's children at its layer's place: a lower-layer form just above the last lower-layer sibling, an upper-layer form on top, any other just below the trailing upper-layer siblings; a form's child forms SHALL draw right after it.

#### Scenario: Plain window after an upper-layer one
- **WHEN** an upper-layer window and then a plain window are added to the root
- **THEN** the plain window draws under the upper-layer one

### Requirement: Screen toggles on the window on top
The inventory and journal keys SHALL close their window only when it is its task's active child by the active test, whatever the pointer focuses, and SHALL raise and focus a window that is open but not on top; the journal SHALL be found by its tag among the task's visible children.

#### Scenario: Journal over the inventory
- **WHEN** the journal is on top while the pointer focuses the inventory, and the journal's key is pressed
- **THEN** the journal closes

### Requirement: Remembered focus
A focus change SHALL remember the visible form it leaves with its selected control; the base form's focus to no form SHALL return to that form when it is still visible and not the focus, else to the root with control -1.

#### Scenario: Hidden remembered form
- **WHEN** the remembered form is hidden and the focus goes to no form
- **THEN** the root takes the focus with control -1

### Requirement: Focus on deactivation
When the game window loses the system's focus, each window manager SHALL deselect its focused form, remember it when visible, and move the focus to its root with control -1.

#### Scenario: Typing in the journal
- **WHEN** the window loses the focus while the journal's text box has it
- **THEN** the box loses the focus and a later focus to no form gives it back

### Requirement: Form close cascade
Closing a form SHALL close its child forms not closing yet, depth first, before it, and every closed form SHALL be hidden and deleted.

#### Scenario: The journal's text box
- **WHEN** the journal closes
- **THEN** its text box closes and is deleted with it

### Requirement: Freed form references
Hiding a closed form SHALL clear the window manager's references to it and its tree: the remembered focus, the pointer's target and capture, the tooltip request, a drop it sources and the modal slot; its focus SHALL pass to its parent with control -1 when the parent is visible, else to no form.

#### Scenario: Focused child
- **WHEN** a window whose child form has the focus closes
- **THEN** the focus is on the root with control -1

### Requirement: Record release landing bit
A held record SHALL hear its release with the release word and the landing bit always, and test the point itself.

#### Scenario: Slider let go off its track
- **WHEN** a held slider is released with the pointer off it
- **THEN** it reports its change

### Requirement: Activation raise
A press SHALL activate its window by the child raise rule within the window's parent.

#### Scenario: Lower-layer window pressed
- **WHEN** a lower-layer window under a plain one is pressed
- **THEN** it keeps its place under the plain window and takes the focus

### Requirement: Open screens follow the GUI unit
When the GUI unit changes, each open in-game screen other than a container popup SHALL take its place scaled as a free form's (`v / old · unit`), its layout's size, client and records at the new unit, and its item panels' entries `round(unit · place / old)`; the journal's text box SHALL be laid out again.

#### Scenario: 720 to 1080 pixels
- **WHEN** the window grows from 720 to 1080 pixels high with the inventory open at 8, 8
- **THEN** the inventory is at 12, 12 with its unit-6 layout and a pack entry at 40, 10 moves to 60, 15

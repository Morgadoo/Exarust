## ADDED Requirements

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

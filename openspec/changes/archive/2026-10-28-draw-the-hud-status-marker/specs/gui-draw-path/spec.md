## ADDED Requirements

### Requirement: HUD power marker and skill-ready alert
The HUD SHALL draw the power marker at `(W/2 + 53·unit, H − 8·unit)` and the skill-ready alert at `(W/2 − 60·unit, H − 8·unit)` as the skill-icon atlas's first cell, 8 units square, each over a rounded black halo filling the square from 3 to 5 units into the icon grown by 8 units at alpha `round(0.6 · pulse · 255)`.

#### Scenario: Marker at half pulse
- **WHEN** the GUI unit is 4, the marker sits at (100, 200) and the pulse is 0.5
- **THEN** the halo's quad spans x 80 to 152 in black at alpha 76 and the icon is one 32-pixel quad from (100, 200) tinted (140, 160, 255, 128)

### Requirement: HUD status icon inks
The marker's icon SHALL be tinted (140, 160, 255) at alpha `CVTSS2SI(pulse · 255)`, and the alert's icon SHALL be drawn outlined at the pulse, its face in the skin's seventh label ink and its four one-pixel copies in the third highlight ink.

#### Scenario: Alert at full pulse
- **WHEN** the alert is drawn at (40, 60) with the pulse 1
- **THEN** the face quad starts at x 40 in the seventh label ink and the first outline copy at x 39 in the third highlight ink

### Requirement: HUD status pulse clock
The marker and the alert SHALL share one pulse clock: the alert's first frame stamps it with the clock, the marker reads it as it is (zero before any stamp), a frame without a player draws neither and keeps the stamp, and only the HUD switched off clears it.

#### Scenario: The alert stamps the clock
- **WHEN** the marker draws alone at 1500 ms and the alert first shows at 3000 ms
- **THEN** the marker's pulse is 0.5 at 1500 ms and 0 at 3000 ms, the alert's pulse at 3000 ms is 1, and at 4000 ms it is 0

### Requirement: Compass interface fade
The compass's fade SHALL be the open power tree's window fade, and 0 while the tree is shut.

#### Scenario: Tree open at a quarter
- **WHEN** the power tree is open with a window fade of 0.25
- **THEN** the compass's fade is 0.25, an opacity of `round(0.75 · 192)` = 144

### Requirement: Form close cascade
Closing a form SHALL restore the controller's action set for style bit 0x20 even when already closing; a form not yet closing SHALL mark itself when it closes once; the task's active child SHALL run its callback, clear the task's active child and close the task; any other form SHALL clear the task's remembered child when it is this form, close the task's children it owns that are not closing, then its own children not closing, then hide, run its callback and redraw.

#### Scenario: Owned popup and child
- **WHEN** a window owning one popup in its task and having one child not closing is closed
- **THEN** the popup is closed and hidden first, then the child, then the window hides, runs its callback and redraws

### Requirement: Form maximise and restore steps
A maximise SHALL show the form, save its record, tell the window-state hook word 2 before the form changes, write the place, set the state's low bits to 2, resize, run the layout-changed virtual and redraw; a restore SHALL show the form and, with a record, write it back, resize, free it and refresh the control spans, then tell the hook word 0 with or without a record, clear the state's low bits, run the layout-changed virtual and redraw.

#### Scenario: Restore without a record
- **WHEN** a form without a maximise record is restored with a hook installed
- **THEN** it is shown, the hook is told 0, the state's low bits clear and the form redraws without moving

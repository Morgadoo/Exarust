# Spec Delta

## ADDED Requirements

### Requirement: Toggle switch answer and no-form cursor
Switching a toggle on or off SHALL answer the control's state low byte with the lit bit cleared. A hit-and-focus that finds no form SHALL focus no form with control 1 and set the cursor to 0.

#### Scenario: Switch on
- **WHEN** a control with state 0 in group 1 is switched on while another member is on and lit
- **THEN** the other member's on and lit bits clear and the answer is 0x20

### Requirement: Double-click clock difference
The double-click window SHALL compare the clock less the press stamp, both widened to 64 bits, with 450 as a signed difference.

#### Scenario: Clock behind the stamp
- **WHEN** the stamp is 1000 and the same button is pressed again at clock 100 at the same point
- **THEN** the press is a double click

### Requirement: Viewport delegates
The viewport's press and key SHALL go to the view's callbacks when installed and to the form's handlers otherwise, a key with its low two mask bits clear also going to the focused control; the release SHALL only reach the view's callback and answer 0; where the form's hit test finds no control the port's cursor code SHALL be stored whether or not the view has a hit callback; a redraw callback answering either low bit SHALL send the task message 7.

#### Scenario: Hit without a callback
- **WHEN** the form's hit test answers -1, the port's cursor code is 4 and the view has no hit callback
- **THEN** the cursor code 4 is stored

### Requirement: Viewport draw-slot refresh
The draw slot SHALL run only for a client at least a pixel each way; a client whose left edge is left of the view's or whose right or bottom edge passes it SHALL refresh; a refresh SHALL restart the counter and render a whole frame with the stereo shift.

#### Scenario: Bottom edge past the view
- **WHEN** the client's bottom is 81 under a view whose bottom is 80 and the counter is 2
- **THEN** the counter restarts at 0 and the frame renders whole

### Requirement: Viewport progressive samples
Without a refresh a counter below the level's count SHALL render, holding while the task's pointer is busy and the port does not draw through it and otherwise advancing with message 0x22 at the first sample and 0x23 after; a counter at the count SHALL render only until the still has accumulated. The sample shares SHALL be divided in double precision.

#### Scenario: First progressive sample
- **WHEN** level 2 (count 3) draws without a refresh from counter 0
- **THEN** the counter is 1, the task hears 0x22, the sample uses point 3 with share one third and the camera is not shifted

### Requirement: Main menu cues and title art
Selecting a new row SHALL play `MenuSel` at most once per 200 clock units and a press or confirm key SHALL play `MenuClick`, each as an audio-environment one-shot at the origin with gain 0.06, flags 0x4060 and pitch `draw·0.4 + 0.8` computed in double precision from the run-time library's unit draw. The title page SHALL draw `maintitle01.rfi` scaled by the row scale (each size rounded half to even), centred across, its top `round(10·row scale)` down, tinted (230, 230, 230, 153).

#### Scenario: Title place
- **WHEN** a 1024×256 title is placed at row scale 0.5 on a 1600-wide screen
- **THEN** it is 512×128 at (544, 5)

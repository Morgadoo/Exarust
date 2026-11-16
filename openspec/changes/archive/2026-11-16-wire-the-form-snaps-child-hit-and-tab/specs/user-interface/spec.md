## ADDED Requirements

### Requirement: Snap slots on show
Showing a base form SHALL rebuild the snap slots of its parent, of the form itself, and of each of its children that is a snap container (attribute 0x20) with children; the rescale walk SHALL rebuild the slots of a form that has any.

#### Scenario: The journal's text box
- **WHEN** the journal opens with its text box
- **THEN** the journal's snap slots cover its client around the box and none overlaps the box

### Requirement: Snap slot cut
A snap container's rebuild SHALL clear its state bit 0x20000 and cut the cells between its client's edges and its visible snapping children's clients, each widened by the child's title spacing reserve, into slots, taking from each free cell the longer free run and the row on a tie; a form without attribute 0x20 SHALL keep its slots.

#### Scenario: One child
- **WHEN** a 96-unit client holds one child whose client spans 12 to 28 on both axes
- **THEN** the slots are the top row, the left column, the right column and the cell under the child

### Requirement: Child form hit
The window manager SHALL test a base form's child forms only for a point on the form's client, the last child first, each visible child without the no-hit style bit whose rectangle holds the point strictly past its left and top edges, and SHALL take a child only when its own hit answers a nonzero code.

#### Scenario: A child's left edge
- **WHEN** the pointer is on a child form's left edge
- **THEN** the parent answers the hit

### Requirement: Tab on the hub's forms
A Tab on a hub form SHALL move the form's focus from its focused record to the next enabled tab stop, wrapping to the first tab stop before it, and Shift+Tab the other way; a text field it lands on SHALL take the key focus with its text selected, and the focus SHALL not follow the pointer again until the pointer moves.

#### Scenario: The emblem form
- **WHEN** Tab is pressed with the key focus on the company name
- **THEN** the background's picture number takes the key focus

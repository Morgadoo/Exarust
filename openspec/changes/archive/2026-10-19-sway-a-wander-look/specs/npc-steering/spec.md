# Spec Delta

## ADDED Requirements

### Requirement: Wander look
A look that is not wandering SHALL stay as it is. A wandering look SHALL copy the heading and start from the extracted angle. At the clock the offset and the roll SHALL stay, and the look SHALL ease toward the heading. Behind the clock the offset SHALL be the unit times 3.6 minus 1.8, the roll SHALL step, and the next timer SHALL add 4000 plus the high part of that step times 4000. An aim past half a turn SHALL wrap once. Posture 1 SHALL ease by 0.03. The flag call is not in this rule.

#### Scenario: At the clock the offset stays, behind it the offset is drawn
- **WHEN** a look that is not wandering is stepped behind its timer, a wandering look at heading 0 is stepped at its clock from extracted angle 1, the same look is stepped one millisecond later from unit 1, and a heading of 4 at posture 1 is stepped at its clock
- **THEN** the other look stays as it was, the on-time look is 0.91 with its offset and roll kept, the late look has offset 1.8, roll 134775814, timer 5126 and look 0.162, and the heading of 4 eases to about −0.0685

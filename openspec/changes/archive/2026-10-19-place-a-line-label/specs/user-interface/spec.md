## ADDED Requirements

### Requirement: Line label placement
The label SHALL align its measured width and line height from the style word and SHALL choose one ink from the state word. Exactly the listed style values SHALL select their arm; any other value SHALL use the default. One text call SHALL be counted.

#### Scenario: The style places the label
- **WHEN** a label of width 40, line height 24 and base 18 is placed at 10, 20 of size 100 by 80 at unit 5 with style 0x1000
- **THEN** x is -35 and y is 23
- **WHEN** style is 0x2000
- **THEN** x is 39
- **WHEN** style is 0x3000
- **THEN** x is 65
- **WHEN** style is 0x4000
- **THEN** x is 115
- **WHEN** style is 0
- **THEN** x is 13 and y is 23

#### Scenario: The vertical arms
- **WHEN** style is 0x100
- **THEN** y is 18
- **WHEN** style is 0x200
- **THEN** y is 73
- **WHEN** style is 0x300
- **THEN** y is 117
- **WHEN** style is 0x400
- **THEN** y is 100

#### Scenario: The state chooses the ink
- **WHEN** the state has bit 0x10
- **THEN** the first ink is used with no nudge
- **WHEN** the state has mode 0x10000 and bit 0x40
- **THEN** the coloured renderer is used and y gains one
- **WHEN** the state has bit 0x60
- **THEN** the third ink is used and y gains one
- **WHEN** the state is 0
- **THEN** the second ink is used with no nudge

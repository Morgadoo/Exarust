## ADDED Requirements

### Requirement: Skill selector draw
At place 10, 180 with a right edge of 110, unit 5 and a measured icon of 20, page 2 SHALL draw a title at 60 across, 90 wide, 100 high. Another page with index 3 SHALL draw the icon at 40, 80 and a label at 90 across, 90 wide, 80 high.

#### Scenario: The title page
- **WHEN** the page is 2
- **THEN** the title starts at 60 across with width 90 and height 100
- **AND** one text draw runs

#### Scenario: A selected skill
- **WHEN** the page is not 2, the raw word is 4 and the icon measures 20
- **THEN** the icon is at 40, 80 and the label starts at 90 across with width 90 and height 80
- **AND** two draws run
- **WHEN** the raw word is 9 or 0
- **THEN** only the begin and bind run

## ADDED Requirements

### Requirement: Tree text on screen
The power tree's texts SHALL be drawn with the GUI font atlas of the current interface level, reloaded when the interface unit changes, and its glyph cells SHALL sample that atlas as the GUI's own text draw does.

#### Scenario: A 720-pixel window
- **WHEN** the screen started at the default interface unit 5 and the window runs at unit 4, and the creation tree opens with the installed packs
- **THEN** the prompt's title lights the band an eighth down the window, and BACK and BEGIN are drawn

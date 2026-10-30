# Spec Delta

## ADDED Requirements

### Requirement: Overlay layout
The timed overlay SHALL draw one panel at (scale, scale), 80·scale wide and as tall as the sum over its lines of wrapped lines × line height + scale, at alpha 0x80 in style 3, and each line at x = 2·scale starting at y = 2·scale, advancing by its wrapped lines × line height + scale; each line's text box SHALL be 80·scale wide and 1000 pixels tall.

#### Scenario: Two lines
- **WHEN** lines wrapping to one and two font lines of 20 pixels are laid out at scale 4
- **THEN** the panel is at (4, 4) sized (320, 68) and the lines start at (8, 8) and (8, 32)

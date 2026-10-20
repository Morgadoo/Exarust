## ADDED Requirements

### Requirement: Path shortcut
The shortcut SHALL keep the first point, drop each point whose direct line from the last kept point to the following point is clear, keep it otherwise, and always keep the last point.

#### Scenario: Clear and blocked lines
- **WHEN** every line is clear on a four-point straight path
- **THEN** only the first and last points remain
- **WHEN** a line is blocked at a bend
- **THEN** the bend is kept and the smooth run after it is shortened
- **WHEN** the path has two points
- **THEN** it is returned unchanged

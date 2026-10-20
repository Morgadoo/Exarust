## ADDED Requirements

### Requirement: Mind-sight fade
The mind-sight overlay SHALL be idle at phase 0, SHALL force alpha 1 and start fading out past its end while shown, and SHALL move alpha by the elapsed milliseconds over 2 s in both fades. The fade-out SHALL end below 0.001.

#### Scenario: Shown, fading and finishing
- **WHEN** the overlay is shown before its end
- **THEN** alpha is 1 and it still draws
- **WHEN** the clock passes the end
- **THEN** it switches to fading out
- **WHEN** a full alpha fades for one second
- **THEN** alpha is 0.5
- **WHEN** it fades below 0.001
- **THEN** the overlay is idle and does not draw

### Requirement: Mind-sight pick
The pick radius SHALL be n·scale·0.5·40/width, and the pick SHALL take the nearest candidate whose distance over its radius is at most 1.

#### Scenario: Inside and outside
- **WHEN** two candidates lie within their radii
- **THEN** the nearer one is picked
- **WHEN** every candidate is farther than its radius
- **THEN** nothing is picked

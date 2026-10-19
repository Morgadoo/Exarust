## ADDED Requirements

### Requirement: Arena match begin
In-game menu command 4, while the mode is arena, SHALL remove the first unhealthy specialist, ask for a save that does not abort on low health, mark the hub busy, clear the world click hook, and set the session phase to transitioning. An empty roster SHALL still do that. Story mode, practice mode, another command, and another page SHALL leave the company and those flags unchanged. Closing windows, writing a save package, and the transition are not part of this step.

#### Scenario: Arena command 4 leaves the physician and arms the match
- **WHEN** an empty company is begun from in-game command 4 in arena, a hired physician at 699 is begun the same way, and that physician is offered to story mode, to practice mode, to command 1, and to another page
- **THEN** the empty company still marks the hub busy, clears the click hook, asks for the save, and sets the phase to transitioning, the physician is removed and the death count is 1, and the other four offers leave the physician and the flags unchanged

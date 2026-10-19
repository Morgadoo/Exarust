## ADDED Requirements

### Requirement: Dismissed child close
A dismissed child SHALL close with argument 0 unless it is already closed. An already closed child SHALL not run its close callback and SHALL not close its own children. A pinned child SHALL be marked closed and raise a notice, then still close its children. Children SHALL close from the first to the last before the parent's callback. The capture release, the task's other children, detaching, redraw, and the call after the two lists are not part of this step.

#### Scenario: A pinned window closes its first child only
- **WHEN** a match begin dismisses a pinned window whose first child has a callback and whose second child is already closed, and a second begin dismisses a window that is already closed and has an open child
- **THEN** the callback order is the first child then the parent, the second child does not run, the notice is raised, argument 0 is stored, the already closed window runs no callback, and its child stays open

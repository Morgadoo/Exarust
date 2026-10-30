## ADDED Requirements

### Requirement: Form client-area refresh
Showing a form SHALL recompute the client rectangle of the form and of every descendant, depth first with children in order, each parent before its children.

#### Scenario: A two-level tree
- **WHEN** a root with children 1 and 2, where 1 has child 3, is refreshed
- **THEN** the forms are measured in the order 0, 1, 3, 2

### Requirement: Freed form slot sweep
Freeing a form SHALL walk its tree parent first and, for each form, test the GUI's single-form slots in order: main, pressed, saved focus, the parent's active child, focused, tooltip, the task's pressed form, dragged, drop payload. Each matching slot SHALL clear, the task's pressed form with its press word, except the main and focused slots, which their follow-ups clear.

#### Scenario: A freed subtree
- **WHEN** form 1 is freed while it is the main form, its child 2 is focused under a visible parent and its child 3 owns the tooltip
- **THEN** the follow-ups are the focus fallback, the parent 1's focus call and the tooltip reset, in that order, and the main and focused slots are left to them

### Requirement: Freed form follow-ups
The sweep SHALL run the focus fallback for the main form, ask a focused form's visible parent to restore its focus or else clear the focus, reset the tooltip for its owner and reset the drop callback for the drop payload.

#### Scenario: A hidden parent
- **WHEN** a focused form under a hidden parent is freed
- **THEN** the focus is cleared

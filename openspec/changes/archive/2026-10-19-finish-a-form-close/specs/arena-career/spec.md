## ADDED Requirements

### Requirement: Form close remainder
A close SHALL release a held capture and clear style bit 0x20, even when already closed. A miss SHALL leave the bit. A hover on this child SHALL clear. Unclosed task children that name it SHALL close from first to last before its own children. A shown child SHALL hide, and the redraw word SHALL gain bits 0 and 1. The active form SHALL clear that pointer and SHALL not ask. Cursor restore and the later list call are not in this step.

#### Scenario: A captured window releases, hides, and redraws twice
- **WHEN** a match begin closes a captured shown window that is the hover and owns one sibling, and a second begin closes an already closed captured window, and a third begin closes the active form
- **THEN** the first drops the capture, clears the bit and the hover, closes the sibling, hides, and the redraw word is 3, the second drops its capture and does not ask, and the third clears the active pointer and does not ask

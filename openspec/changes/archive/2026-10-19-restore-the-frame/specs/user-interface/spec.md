## ADDED Requirements

### Requirement: Frame return
Every dispatch SHALL pass the frame to one call, restore the saved register, then set the stack to the frame and pop the frame. A held band, a wide form, and a style that is not that band each SHALL do so. The helper body is not in.

#### Scenario: A plain form restores the frame
- **WHEN** unit 5 dispatches a form that is not wide, with no style bits and a close mark of 0
- **THEN** the frame call count is 1 and the frame is restored

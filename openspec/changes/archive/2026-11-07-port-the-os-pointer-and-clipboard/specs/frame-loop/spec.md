# Spec Delta

## ADDED Requirements

### Requirement: Pointer warp hold
When the game warps the pointer to a point inside the window's client area, the system SHALL hold it there for the next four pointer moves: a move that lands 41 pixels or more from the target on either axis SHALL put the pointer back on the target and use up one move, and a move that lands nearer SHALL end the hold. Cursor Look's release SHALL warp the pointer back to where it was with this hold, except on Wayland, where the position is the lock's hint and no hold is kept.

#### Scenario: A stale move after Cursor Look
- **WHEN** Cursor Look is let go and the next pointer move still arrives 300 pixels from the restored position
- **THEN** the pointer is put back on the restored position, and a following move landing 40 pixels away ends the hold

### Requirement: Full-screen pointer confinement
Once a capture has ended (a mouse button let go in the focused window), a focused full-screen window SHALL keep the pointer inside it until it loses the focus; regaining the focus SHALL NOT confine it again until the next capture ends. Cursor Look's lock SHALL take precedence while it is held.

#### Scenario: Clicking in full screen
- **WHEN** a borderless full-screen window is clicked and the pointer is pushed toward another monitor
- **THEN** the pointer stays on the window until the window loses the focus

### Requirement: Clipboard text
The system clipboard SHALL take a text control's bytes as Windows-1252 text, and a read SHALL return the clipboard's text up to its first zero, each character through Windows-1252 and `?` for a character the code page lacks; a clipboard that holds no text or cannot be opened SHALL give nothing to paste. The clipboard SHALL be opened on its first use.

#### Scenario: A pasted name
- **WHEN** the clipboard holds "café €5" and a control reads it
- **THEN** it gets the bytes `63 61 66 E9 20 80 35`

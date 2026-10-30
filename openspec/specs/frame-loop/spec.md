# frame-loop Specification

## Purpose
The original's platform and frame layers as Exarust keeps them: the frame
tick's pacing, clocks, idle wait, timers, deferred calls and log presentation,
and the window's display modes, full screen, limits, toggle and focus rules.

## Requirements

### Requirement: Frame-rate cap
The system SHALL hold a capped frame until a target interval has passed since the previous frame's start: 32 ms while unfocused; with vertical sync the display's period less 1 ms for an automatic cap or one at or above the refresh rate, else the cap's period less one; without vertical sync the period of the refresh rate (at least 120 Hz) for an automatic cap, else the cap's, less one. Each period SHALL be `1000 / rate` rounded to nearest, ties to even.

#### Scenario: 60 frames a second
- **WHEN** the cap is 60 without vertical sync on a 144 Hz display and 10 ms have passed
- **THEN** the frame waits 6 ms more (interval 16 ms)

### Requirement: Unknown refresh rate
The system SHALL treat a refresh rate below 30 Hz as unknown (0), which without vertical sync gives the 120 Hz floor and under vertical sync holds nothing.

#### Scenario: Unknown under vertical sync
- **WHEN** the cap is automatic with vertical sync and the rate is unknown
- **THEN** no frame is held

### Requirement: Cap source
The cap SHALL be off unless the command line (`--max-fps`) or the settings file's `FPSLimit` gives one, the command line winning.

#### Scenario: No cap by default
- **WHEN** neither `--max-fps` nor `FPSLimit` is given
- **THEN** no frame is held

### Requirement: Game clock
The system SHALL measure game time as real time less a paused-time accumulator, adding the whole frame while the game is paused, so that game time stops during a pause.

#### Scenario: A paused frame
- **WHEN** a 25 ms frame passes while paused
- **THEN** the game clock does not advance

### Requirement: Idle wait
The system SHALL choose how long the message loop may wait after a tick from the redraw flags, bits 4–7 also counting as bits 8–11: no wait for an immediate redraw, 1 ms (32 ms unfocused) for an active interface, 32 ms (64 ms) for an animation, 32 ms (1024 ms) when idle; a minimised window SHALL always wait at least 32 ms and draw nothing.

#### Scenario: Minimised
- **WHEN** an active interface is minimised
- **THEN** the loop waits 32 ms and the frame is not drawn

### Requirement: Deferred calls
The system SHALL run each queued deferred call once a tick and keep the ones that ask to run again in their order, in a ring that holds at most 127 calls and drops a call pushed into a full ring; once the queued calls wrap past the ring's end only the first SHALL run in a tick.

#### Scenario: Ring full
- **WHEN** 131 calls are pushed into an empty ring
- **THEN** 127 are queued

### Requirement: Periodic timers
The system SHALL run each due timer in record order and re-arm it `interval × 100` from the frame time, reuse the first freed record for a new timer, and reschedule a timer `(interval − elapsed) × 100` from the frame time.

#### Scenario: Re-armed from now
- **WHEN** a 20-unit timer added at 1000 ms runs at 3000 ms
- **THEN** it runs next at 5000 ms

### Requirement: Log presentation
The system SHALL present a logged message by its level: 1–3 on the status line (sixteen entries, newest first), 4 and 5 on a timed overlay of at most fifteen `title : message` lines that stay two seconds plus 50 ms a byte and expire one a tick, 6 as an error box and 7 as a warning box; every level but 5 SHALL go to the log file.

#### Scenario: Overlay lifetime
- **WHEN** "Error" and "abcd" are logged at level 4 at game time 1000 ms
- **THEN** the line reads "Error : abcd" and stays until game time 3600 ms

### Requirement: Full screen
The system SHALL show full screen as a borderless window over the primary display, and as exclusive full screen at that video mode only when `ScreenWidth`/`ScreenHeight` name a size other than the desktop's that the display lists.

#### Scenario: Unset screen size
- **WHEN** full screen is chosen without `ScreenWidth`/`ScreenHeight`
- **THEN** the window is borderless over the desktop

#### Scenario: Another listed size
- **WHEN** `ScreenWidth = 1920` and `ScreenHeight = 1080` on a 2560 × 1440 desktop that lists 1920 × 1080
- **THEN** the window is exclusive full screen at 1920 × 1080

### Requirement: Window limits and toggle
The window SHALL not be resized below 640 × 480, its present mode SHALL wait for the display with vertical sync on (the original's swap interval 1; Exarust takes the adaptive interval, which presents a late frame at once, while its frames are slower than the display) and present at once with it off, and Alt+Enter SHALL toggle between the framed window and full screen, restoring the framed window's size and position.

#### Scenario: Back to the frame
- **WHEN** Alt+Enter is pressed twice on a 1600 × 900 window
- **THEN** the window is framed again at 1600 × 900 where it was

### Requirement: Focus release
The system SHALL release every held key and mouse button when the window gains or loses the focus.

#### Scenario: A key held across a switch
- **WHEN** the window loses the focus while W is held
- **THEN** W is no longer pressed

### Requirement: Display modes
The system SHALL list display modes at least 640 pixels wide, each size once with every refresh rate reported for it, ordered by area with the original's exchange sort.

#### Scenario: Duplicate rates
- **WHEN** a driver reports 1920 × 1080 at 60, 144 and 60 Hz
- **THEN** the list holds 1920 × 1080 once with 60 and 144 Hz

### Requirement: Hook lists and deferred cancellation
A method hook SHALL be inserted before the first entry with a strictly greater priority and removed by code and data; a procedure hook with a code already listed SHALL be refused. Dispatch SHALL drop every hook that answers false. A deferred call SHALL be removable by its data or procedure tag, the remaining calls keeping their order.

#### Scenario: Cancel the middle call
- **WHEN** five tagged calls are queued and the third's data tag is removed
- **THEN** the next run calls 1, 2, 4 and 5 in that order

### Requirement: Millisecond clock
The millisecond clock SHALL be `(counter - start) × 1000 / frequency` with a wrapping product and a truncating division.

#### Scenario: Truncation
- **WHEN** a 1 MHz counter reads 1 999 999 ticks after its start
- **THEN** the clock reads 1999 ms

### Requirement: Overlay layout
The timed overlay SHALL draw one panel at (scale, scale), 80·scale wide and as tall as the sum over its lines of wrapped lines × line height + scale, at alpha 0x80 in style 3, and each line at x = 2·scale starting at y = 2·scale, advancing by its wrapped lines × line height + scale; each line's text box SHALL be 80·scale wide and 1000 pixels tall.

#### Scenario: Two lines
- **WHEN** lines wrapping to one and two font lines of 20 pixels are laid out at scale 4
- **THEN** the panel is at (4, 4) sized (320, 68) and the lines start at (8, 8) and (8, 32)

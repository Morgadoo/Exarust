# Spec Delta

## ADDED Requirements

### Requirement: Offscreen game
`--offscreen <file>` SHALL run the game without an OS window: every camera that would draw to the primary window SHALL draw into one image of the `--size` given (1280 × 720 by default), interface included, and each frame SHALL be written to the file. Without the flag the game SHALL open its window as before.

#### Scenario: A frame in the file
- **WHEN** `exarust --offscreen /dev/shm/q.rgba play arena` has drawn its scene
- **THEN** the file holds a 1280 × 720 frame of the arena with the frame-rate readout

### Requirement: Offscreen frame file
The frame file SHALL hold a 64-byte header (magic `EXAGRID1`, a sequence number that is odd while the pixels are written, width, height, frame counter, a loading-screen flag) followed by the frame's unpadded opaque RGBA8 pixels.

#### Scenario: A finished frame
- **WHEN** a reader finds the same even sequence number before and after reading the pixels
- **THEN** the pixels are one whole frame

### Requirement: Offscreen input
An offscreen game SHALL take input lines on standard input as its window's keyboard, mouse button, wheel, motion and cursor events, log each forwarded key and button, and end when standard input closes.

#### Scenario: Forwarded key
- **WHEN** the line for F3 pressed and then released arrives on standard input
- **THEN** the game toggles its body overlay and logs the forwarded key

### Requirement: Grid view
`exarust grid <scene>…` SHALL open one window (2560 × 1440 by default) split into a 2 × 2 grid of camera viewports, each a quarter of the window's physical size and recomputed when the window is resized, each showing the newest frame of its own offscreen game: `play <scene>`, or the scene's words when it holds spaces.

#### Scenario: Four scenes
- **WHEN** `exarust grid exanima01 arenao3 exanima06 arena` runs
- **THEN** one window shows the four levels live, quadrant 1 top left to quadrant 4 bottom right

### Requirement: Grid launches
The grid SHALL launch its games one at a time, each once the previous one has shown a frame and the GPU reports enough free memory, SHALL show a quadrant's status until its first frame, and SHALL show a game's exit status once it ends, launching it again on Enter while its quadrant is active or, with `--restart`, by itself.

#### Scenario: A game that ends
- **WHEN** a quadrant's game exits with code 2
- **THEN** the quadrant shows "exited with code 2" with the game's last log line, and Enter while it is active launches it again

### Requirement: Grid input
The grid SHALL forward the keyboard, mouse buttons, wheel, mouse motion and cursor to the quadrant last clicked only, the cursor in that game's pixels.

#### Scenario: Input to one quadrant
- **WHEN** quadrant 2 is clicked and F3 is pressed
- **THEN** only quadrant 2's game shows the body overlay

### Requirement: Grid control
`exarust grid-swap <n> <scene>` SHALL replace quadrant n's game in the running grid without touching the others, and `exarust grid-shot <file.png>` SHALL save the grid window as composed.

#### Scenario: Swap
- **WHEN** `exarust grid-swap 4 arenahub` runs against a grid whose quadrant 4 plays arena
- **THEN** quadrant 4's game ends and arenahub loads in its place while quadrants 1 to 3 keep running

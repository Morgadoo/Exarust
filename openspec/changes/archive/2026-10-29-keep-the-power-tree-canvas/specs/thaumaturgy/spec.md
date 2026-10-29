## ADDED Requirements

### Requirement: Kept power tree canvas
The running power screen SHALL keep its canvas between redraws and redraw only the pixels inside the boxes of the batches that differ from the last draw, and the kept canvas SHALL hold the same bytes as rasterising the whole draw. The canvas SHALL be handed to the image node with straight alpha and SHALL draw over the HUD's layers.

#### Scenario: The open tree redraws the wheel
- **WHEN** the open tree is drawn 33 ms after a previous draw at 1280x720
- **THEN** fewer than a quarter of the window's pixels are redrawn and the canvas equals a whole redraw

#### Scenario: An unchanged draw uploads nothing
- **WHEN** a redraw's batches equal the last draw's
- **THEN** no pixel is redrawn and the image is not uploaded again

#### Scenario: Straight alpha over the HUD
- **WHEN** a canvas pixel holds premultiplied colour (64, 32, 0) at alpha 128
- **THEN** the image node receives (128, 64, 0) at alpha 128, in a layer above the HUD's

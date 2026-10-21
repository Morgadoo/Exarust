# Spec Delta

## ADDED Requirements

### Requirement: Player light switch
The player's own light SHALL burn while the player wears an item that sees in the dark, and SHALL go out while either hand holds an item with a held light's section; the switch SHALL be read whenever what the player wears or holds changes, making or removing the light.

#### Scenario: Helm and torch
- **WHEN** a night-vision helm is worn with empty hands, then with a torch in the off hand, and nothing is worn
- **THEN** the light burns, then goes out, and stays off

### Requirement: Player light frame
The light SHALL be an omni light of colour (1, 1, 1), radius 1200 cm, multiplier 2 and softness 4 that falls off with distance, made with intensity 0.01. Each frame while under 0.4 less the single-precision epsilon it SHALL gain 0.0005 per millisecond (double precision, then single), clamped to 0.4. It SHALL hang 80 cm above the head, or where a 10 cm ball swept up from the head first meets the level when that is nearer.

#### Scenario: Rising and hanging
- **WHEN** a 16 ms frame passes from 0.01, a frame from 0.399, and the head is 150 cm up with no ceiling, then with a ceiling the ball meets 35 cm up
- **THEN** the intensity is 0.018, then 0.4; the light hangs at 230 cm, then at 185 cm

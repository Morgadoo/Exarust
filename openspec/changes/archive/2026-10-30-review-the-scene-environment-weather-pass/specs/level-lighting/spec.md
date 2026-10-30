## ADDED Requirements

### Requirement: Frame scratch and clear colours
Each frame the scene environment SHALL clear with the level's background under a sky and keep the horizon colour as its scratch colour; without a sky and with fog it SHALL take the fog colour as the scratch and clear with it only when the fog clears, else with the background; during a flash it SHALL clear with the scratch times the flash tint, halved, with an alpha of 1. The rain effect SHALL take the scratch colour.

#### Scenario: Outdoor arena
- **WHEN** a level with a sky and a (0.5, 0.25, 0, 1) background is applied
- **THEN** the clear colour is that background linearised and the scratch is the record's sky colours at weight 0.5 times the fog's sky colour, times 2, linearised, with alpha 1

#### Scenario: Flash with the default tint
- **WHEN** a fogged scene without a sky flashes with the constructor's zero tint
- **THEN** it clears opaque black

### Requirement: Sky dimming luminance
The cloud dimming SHALL compute each colour's luminance as `(x·0.299 + y·0.587) + z·0.114` in single precision.

#### Scenario: Order-sensitive channels
- **WHEN** the channels have the bit patterns 1051251352, 1061803639 and 1050360922 and the mix is 1
- **THEN** every channel becomes the float with bits 1058574577, not 1058574578

### Requirement: Legacy grid ambient
The legacy grid build SHALL, unless the level has terrain, set the ambient to the 0.6275 grey through the ambient setter and the global light to 0.75 with strength 0.75, leaving the sun light's colour alone, and in every case set an opaque black background and a mid-grey fog colour.

#### Scenario: Menu scene without terrain
- **WHEN** a legacy grid level with ambient (0.5, 0.4, 0.3) and no terrain is lit
- **THEN** its ambient is 0.6275 grey linearised (0.3516) and its sun colour stays 0.75

### Requirement: Rain objects
A rain request above zero SHALL create the rain sound and the rain effect when they do not exist, and the weather tick SHALL drive only a created sound: its volume and pitch from the effect while it rains, a start when its volume passes 0.01 and a stop request when a started sound falls below 0.01.

#### Scenario: No request
- **WHEN** the scene ticks with rain but no request was made
- **THEN** no sound starts

# Spec Delta

## ADDED Requirements

### Requirement: Card gates
A weapon card's material swatch SHALL add its height only when more than two striking rows (thrust included) show. A weapon's points row SHALL show while the skill's low five bits are set and SHALL read those bits, less one but not below 0 when the item flags' `0x30` bits are `0x10`, times one half. The value row SHALL follow its own gate. A shield with the fixed flags SHALL show no rank, points or value row.

#### Scenario: Swatch, points and a fixed shield
- **WHEN** a weapon shows slash and thrust only with a swatch, flags 0x10 and skill 0x65, and a fixed shield is shown with both gates open
- **THEN** the swatch adds no height, the points are 2, and the shield has no rank, points or value

### Requirement: Card title tint
Only the weapon and apparel cards SHALL tint the title, and only while the rank gate is off. The weight SHALL be a double-precision product narrowed to single, and each blended channel SHALL round half to even.

#### Scenario: Half way
- **WHEN** a channel blends from 0 to 255 by 0.5 with the rank gate off
- **THEN** it is 128

### Requirement: Card refresh slots
Refreshing an open weapon card SHALL write each non-zero slash, crush and pierce byte times 0.05 into the next striking bar from 0xd03, in that order.

#### Scenario: A zero slash
- **WHEN** slash is 0, crush 3 and pierce 40
- **THEN** 0xd03 holds 0.15 and 0xd04 holds 2

### Requirement: Panel mouse order
The panel's mouse event SHALL do nothing for the base form's bit or without the item bit, then SHALL start a drag for the 0x4000 bit, then SHALL treat 0x8000 as the double click and 0x2000 as the card. A double click on a use-class-1 item SHALL close the card. The card SHALL be delayed for a use section of kind 0, 1 or 2 (kind 2 running the use first) and built at once for a higher kind or no section.

#### Scenario: Drag wins
- **WHEN** a press carries 0x4000, 0x8000 and 0x2000 over an item that allows dragging
- **THEN** the drag starts

### Requirement: Icon render plan
The icon render SHALL project through the packed transform as `(x·a + y·b) + (z·c + w)`, SHALL take the extents' span in single precision and widen it by 0.0001 in double, and SHALL frame the scene's depth along the matrix's second row starting from the first kept node's stored depth. While the alpha global is set the screen-quad pass SHALL replace the downsample and the halo.

#### Scenario: Paired sums
- **WHEN** the point (1e8, 1, −1e8) goes through a row of ones with translation 1
- **THEN** the projected value is 0

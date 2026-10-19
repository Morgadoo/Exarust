## ADDED Requirements

### Requirement: Skin frames
At place 10, 20 of size 100 by 80, style bits 0x30 SHALL select one frame: 0x10 from 10, 20 to 110, 100 with texture 0x9c then 0xb8, and 0x20 the same rectangle with 0xb4 then 0x9c. Style bits 0xc0 SHALL independently select a second frame inset by one on every side, with 0x40 using 0x9c then 0xb8 and 0x80 using 0xb4 then 0x9c. Any other value in a group SHALL select nothing.

#### Scenario: Each group selects its frame
- **WHEN** the style is 0x10
- **THEN** one frame is 10, 20 to 110, 100 with textures 0x9c and 0xb8
- **WHEN** the style is 0x20
- **THEN** one frame is the same rectangle with textures 0xb4 and 0x9c
- **WHEN** the style is 0x40
- **THEN** one frame is 11, 21 to 109, 99 with textures 0x9c and 0xb8
- **WHEN** the style is 0x80
- **THEN** one frame is the inset rectangle with textures 0xb4 and 0x9c

#### Scenario: Both groups run
- **WHEN** the style is 0x50
- **THEN** the first frame is 10, 20 to 110, 100 and the second is 11, 21 to 109, 99
- **WHEN** the style is 0, 0x30 or 0xc0
- **THEN** no frame is selected

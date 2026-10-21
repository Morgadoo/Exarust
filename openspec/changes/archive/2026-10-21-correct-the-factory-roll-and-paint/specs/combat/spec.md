## ADDED Requirements

### Requirement: Factory area colours
The system SHALL roll a factory area's colour, when its candidate does not fix one, by the material's colour class as the item randomiser does: steel 3 and iron 4, painted steel the item palette's metal colour, material 15 one of three colours and copper one of six by one draw, gems and resin none, and keep it as the lifted palette colour stored in bytes.

#### Scenario: Copper alloy
- **WHEN** a factory area rolls copper alloy with the generator in state 1
- **THEN** its colour is palette index 0x5D

#### Scenario: Random set 8
- **WHEN** a layer draws from random set 8
- **THEN** it chooses among materials 1, 2, 3, 4, 17, 13 and 20

### Requirement: Factory part placement
The system SHALL attach each assembled part at its parent's attach point, read from the parent component's stored box, moved by the parent's scale and band deforms, scaled by the parent's recipe scale and moved to the parent's position; a part without a parent SHALL keep its recipe position. Parts SHALL turn about X, then Z, then Y.

#### Scenario: Box corner
- **WHEN** a part at (1, 2, 3) attaches by mode 1 to a parent scaled 3 at the origin whose box minimum X is −1
- **THEN** it is placed at X 0 and Y 6

### Requirement: Factory paint job
The system SHALL paint each assembled part with its masks' area materials (an empty area as SteelGen), the areas' lifted colours and finish levels, the item's quality and dirt bytes over 255 and its damage remapped as 2/(2 − d) − 1, repeating library textures by the part's atlas region and shifting them by four offsets drawn from the item's second seed.

#### Scenario: Finish level
- **WHEN** an area's finish byte is 0x8B
- **THEN** its layer paints with wear weight 0.75

## MODIFIED Requirements

### Requirement: Weapon stats
The system SHALL derive a weapon's stats from its item's weapon section. For a factory weapon it SHALL apply the recipe's variant bonuses and deform rules to the template's bytes, then scale them by the item's condition. The haft, head taper, head radial and blade bend rules SHALL use the curve 1 + x (or 1/(1 − x)) without the deform's amount, and the flipped side's deforms SHALL change no byte.

#### Scenario: Longer blade
- **WHEN** a blade part's Y-scale deform grows the blade by a factor 1.2
- **THEN** the weapon's weight byte is multiplied and its balance byte divided by 1.2, rounded

#### Scenario: Haft with a small amount
- **WHEN** a haft's Y-scale deform of amount 0.5 has the value 0.5 and the template's weight and balance are 40 and 80
- **THEN** the weapon's weight is 44 and its balance 67

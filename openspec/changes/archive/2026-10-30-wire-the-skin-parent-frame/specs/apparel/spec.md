# Spec Delta

## MODIFIED Requirements

### Requirement: Apparel matching with parent points
A garment seam group SHALL start at its first joint's distance, parent group points SHALL only lower that bar, and a later joint SHALL be taken only when strictly nearer than the bar; a parent point SHALL never be taken. The group's centre SHALL be carried into the parent's space by the child node's transform before each parent comparison.

#### Scenario: Shadowed joint
- **WHEN** joint 0 is 50 cm away, joint 1 10 cm and a parent point 1 cm
- **THEN** the group keeps joint 0

#### Scenario: Placed child
- **WHEN** a group centre at the origin, joint 0 at 50 cm, joint 1 at 10 cm and a parent point at 20 cm along x are matched for a child placed 0.2 m along x
- **THEN** the group keeps joint 0, and without the placement it takes joint 1


## ADDED Requirements

### Requirement: Skin parent frame
The skin build SHALL hand each part's children its group centres as their parent points, and a part that keeps a shipped skin stream or finds no group SHALL hand them none.

#### Scenario: Shipped stream
- **WHEN** a part keeps a shipped skin stream
- **THEN** it hands its children an empty frame

### Requirement: Paint job condition
The paint job SHALL convert each appearance byte as byte × 1/255 in double narrowed to float, and SHALL remap damage as 2/(2 − d) − 1 and dirt as (d² + d)·0.5 in float.

#### Scenario: Condition bits
- **WHEN** quality, damage and dirt are 0x70, 0x40 and 0x52
- **THEN** the condition's bits are 0x3ee0e0e1, 0x3e12f110 and 0x3e599654

### Requirement: Layer mask names
A paint layer's mask texture SHALL be the stem, `m` and the single character of the mask byte plus `'0'`.

#### Scenario: Mask ten
- **WHEN** a layer of stem `shirty_a` has mask 10
- **THEN** its mask texture is `shirty_am:`

### Requirement: Effect mesh lookup
The effect lookup SHALL first take a skinned child of the node in reverse child order and only when none is skinned SHALL search the children's subtrees in forward order.

#### Scenario: Earlier subtree
- **WHEN** neither child is skinned and both have a skinned child
- **THEN** the first child's skinned child is found

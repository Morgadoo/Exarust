# Spec Delta

## ADDED Requirements

### Requirement: Card icon
A plain card SHALL place its icon by dividing the span minus the icon size by 2, and by dividing the content minus the icon size by 2 and then adding four times the unit. A stats card SHALL place it at thirteen times the unit minus the icon size shifted down one bit, and at the unit. When eleven times the unit is below the picture width, that horizontal place SHALL add the picture width minus eleven times the unit.

#### Scenario: A plain centre and a wide stats shift
- **WHEN** a plain card of span 268 and content 160 uses an icon of 160 and a unit of 4, a plain card of span 269 and content 100 does the same, and a stats card uses a picture of 40, 44, 45, and 160
- **THEN** the first place is 54, 16, the second is 54, -14, pictures 40 and 44 stay at -28, 4, picture 45 is -27, 4, and picture 160 is 88, 4

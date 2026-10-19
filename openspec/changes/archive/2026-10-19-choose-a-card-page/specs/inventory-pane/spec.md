# Spec Delta

## ADDED Requirements

### Requirement: Item card page
An item card SHALL be absent for an id below 1, a container, a missing model, and a model mark of 0. A map SHALL be a map and have no value. Otherwise the value SHALL be the coin word divided by 4, unless the high bit is set, which keeps the word. A weapon that is not withheld SHALL be the weapon page. An armour type from 1 through 6 SHALL be the apparel page. A shield that is not withheld SHALL be the shield page. Any other item SHALL be the plain page.

#### Scenario: A weapon, a coat, a shield, and a map
- **WHEN** an id of 0, a container, a map, a missing model, and a mark of 0 are chosen, a weapon of 7 coins is chosen, the same weapon keeps its full word, a withheld weapon of armour 6, 0, and 7 is chosen, and a plain item of 3 coins is chosen
- **THEN** the first five have no stats value, the map is a map, the weapon value is 1 and then 7, armour 6 is apparel, armour 0 is a shield, armour 7 is plain, and 3 coins print 0

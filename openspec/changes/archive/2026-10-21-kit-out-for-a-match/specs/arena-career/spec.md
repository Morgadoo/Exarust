## ADDED Requirements

### Requirement: Match loadout
A fighter SHALL take into a match the loadout of half its skill levels, at most the match's tier, or for a Captain's entrants after the first at most two under the tier, that cap compared in 64 bits. A match with a tier below 0 SHALL take the first loadout with both hands empty. The match copy's fighting skill SHALL be ten times one more than the levels under ten levels, else 100.

#### Scenario: Tiers and wings
- **WHEN** levels 7 at tier 5, 12 at tier 2, 12 at tier −1, a Captain's wing of 12 at tier 4, of 3 at tier 4 and of 12 at tier 1 are asked, and the fighting skill of 0, 4, 9 and 14 levels
- **THEN** they take ranks 3, 2, no gear, 2, 1 and −1, and the skills are 10, 50, 100 and 100

### Requirement: Loadout repair
A fighter whose loadouts are not six SHALL have them rebuilt from its kit, all six trimmed to their ranks, raising "Loadout corrupted, attempting save recovery.". After a skill level is gained, a level sum that is even and under 11 SHALL copy the loadout of the rank below into the new rank.

#### Scenario: Two loadouts
- **WHEN** six loadouts and two loadouts are repaired with a rank-2 item, then levels 4, 5 and 12 are gained and levels 4 without a gain and 0 with one
- **THEN** six stay, two become six with the rank-2 item only from rank 2 and empty hands; 4 copies rank 1 into rank 2 and the others copy nothing

### Requirement: Outfit value
An outfit SHALL be worth the sum of its carried items' and set hand items' value words, each priced as a shop line.

#### Scenario: Copper, silver and gold
- **WHEN** an outfit carries a 7-copper item, a 2-silver item and an item without a value and holds a 1-gold item
- **THEN** it is worth 175 copper

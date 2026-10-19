# arena-career Specification

## Purpose
The arena career: a company's money, who it can hire, and what a standard match pays.

## Requirements

### Requirement: New career
A new company SHALL start with 343 copper. That total SHALL split into gold, silver and copper at 12 to 1.

#### Scenario: Seed
- **WHEN** a company is created
- **THEN** it holds 343 copper, which is 2 gold, 4 silver and 7 copper

### Requirement: Roster
A company SHALL hire at most seven combatants, and at most one merchant, one physician and one trainer.

#### Scenario: Caps
- **WHEN** seven combatants and one physician are hired, then one more of each
- **THEN** both further hires are refused

### Requirement: Match
Entering a standard match SHALL subtract its fee from the company's copper. A win SHALL add the prize. Experience SHALL be the template amount times the share of opponent health dealt, and half again when a trainer is hired. A loss SHALL not add the prize.

#### Scenario: Duel
- **WHEN** a company with a combatant enters a duel and loses after dealing all of the health, then hires a trainer, enters again and wins after dealing half
- **THEN** the first result has paid the fee and credited 1000 experience, and the second has added the prize and 750 experience

### Requirement: Shop trade
The shop SHALL refuse two empty baskets, and SHALL refuse a buy total that exceeds the sell total by more than the company's copper. A confirmed trade SHALL change the copper by that difference, recording a payment as spent and a receipt as earned.

#### Scenario: Pay and receive
- **WHEN** a company with 343 copper is offered an empty trade, a buy of 400, a buy of 40, then a buy of 10 against a sell of 25
- **THEN** the empty trade and the buy of 400 are refused, the buy of 40 leaves 303 copper and 40 spent, and the last trade leaves 318 copper and 15 earned

### Requirement: Experience scale
The experience pool SHALL be the template amount times one plus a quarter of the tier, times the share of opponent health dealt, and half again when a trainer is hired. That pool SHALL be split among the company's combatants by their contribution counts.

#### Scenario: Two fighters
- **WHEN** two combatants win a duel at tier 0 after dealing all the health, with contributions 1 and 3, then a trainer is hired and they win a duel at tier 4 after dealing all the health, with equal contributions
- **THEN** the first result credits 250 and 750, and the second adds 1500 to each

### Requirement: Special matches
The board SHALL offer Challenger, Elimination, Valiance, Valour, Captain, Brawl and The Beast with their recorded fees, prizes, sides and experience.

#### Scenario: The Beast
- **WHEN** the special board is read
- **THEN** Challenger asks 256 copper for a prize of 768 and 1000 experience, and The Beast asks 512 copper for a prize of 196608, three a side, and 10000 experience

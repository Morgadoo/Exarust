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
- **THEN** Challenger asks 12 copper for a prize of 36 and 1000 experience, and The Beast asks 24 copper for a prize of 432, three a side, and 10000 experience

### Requirement: Day and season
Ending a day SHALL add 1 from 0, pay each wage, and rest every employee. Wages SHALL be 2 for a merchant, 12 for a physician, the six-skill sum for a trainer, and half that sum, at most 5, plus 1 for a combatant. Rest SHALL add 6553, or 9830 with a physician, and stop at 65535. Day mod 7 of 1 SHALL give the week to id 1 with at least 3 of 6 events and no tie. A multiple of 7 SHALL drop an entry still under 3276. Four won weeks SHALL grant four rewards.

#### Scenario: Wages, a dropped entry, and a swept season
- **WHEN** a company with one unskilled combatant, entered and at health 3275, ends days through a sweep of every week, a second company ends day 1 on a 3–3 tie, a fighter at 1000 rests one night, a fighter at 60000 rests one night, pools of 58981 and 58982 rest one night, and a physician's company rests a fighter at 1000
- **THEN** the first night costs 1 copper and takes week 0, day 7 keeps the entry at health 49146, day 22 grants four rewards, the tie takes no week, 1000 becomes 7553, 60000 and 58982 become 65535, 58981 becomes 65534, and the physician's fighter becomes 10830

### Requirement: Trainer lesson
With a trainer hired, a night SHALL add 1000 divided by one plus the sum of the six skill levels to the first skill whose level is under 5 and whose technique at that level the trainer knows. The trainer knows a slot below the trainer's own level. A sum that reaches a new multiple of 5000 SHALL snap to it. A merchant, a physician, and a company with no trainer SHALL gain nothing.

#### Scenario: A known technique gains the lesson
- **WHEN** a combatant training technique 7 rests with a trainer who knows 7, the same combatant rests with a trainer who knows 3, a merchant and a physician training 7 rest with a trainer who knows 7, a combatant at 4500 rests with a matching trainer, a combatant whose other five skills are level 1 rests with a matching trainer, and a combatant whose first skill is level 5 rests while the second skill trains technique 9
- **THEN** the first word is 1000, the unmatched word stays 0, the merchant and the physician stay 0, 4500 snaps to 5000, the ranked word is 166, the level-5 word stays 25000, and the second skill becomes 166

### Requirement: Hire price
A hire SHALL cost 12 copper. Fewer than 12 SHALL refuse the hire and leave the copper and the roster unchanged. A full combatant roster or a second specialist SHALL refuse without spending. A hire that joins SHALL subtract 12 and add 12 to the amount spent.

#### Scenario: Eleven copper hires nobody
- **WHEN** a company with 11 copper tries to hire, and a company with 12 hires a combatant, then a physician, then a second physician
- **THEN** the first hire is refused with an empty roster and 11 copper, the combatant leaves 0 copper and 12 spent, and the second physician is refused with the spent total still 24

### Requirement: Item price
A shop line's low fourteen bits SHALL be its count. Both high marks SHALL price that count at 1440 copper each, the higher mark alone at 144, the lower mark alone at 12, and no mark at 1. A buy basket SHALL be the sum of those prices. A sell basket SHALL be that sum divided by 4, dropping the remainder. A confirmed trade SHALL use those two totals.

#### Scenario: Marks, a quartered sale, and the trade
- **WHEN** a line of 7 is bought and sold, a lower-mark count of 2, a higher-mark count of 2, and a both-mark count of 2 are bought, two lines of 3 are sold, a line with every bit set is bought, and a company with 343 copper buys a line of 40 and then buys a line of 10 against a sold line of 100
- **THEN** 7 buys for 7 and sells for 1, the marked buys are 24, 288 and 2880, the two lines of 3 sell for 1, the full line buys for 23591520, the line of 40 leaves 303 copper and 40 spent, and the last trade leaves 318 copper and 15 earned

### Requirement: Tier scale
A tier SHALL multiply a match's fee and its prize by one more than the tier. Tier 0 SHALL charge the recorded copper.

#### Scenario: A higher tier doubles the stake
- **WHEN** a company hires one combatant and enters Challenger at tier 1, then wins
- **THEN** it has paid 24 and holds 379 copper, with 72 earned

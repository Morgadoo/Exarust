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

### Requirement: Season rewards
A swept season SHALL grant that season's four item ids. Season 0 grants 1175, 1174, 1177 and 1176. Season 1 grants 1167, 1165, 1168 and 1169. Season 2 grants 1171, 1170, 1173 and 1172. Season 3 grants 1178, 1179, 1180 and 1181. The season is the day divided by 28, wrapped to four, so day 134 grants season 0 again. A season the company did not sweep SHALL grant nothing.

#### Scenario: Four rows, then the first again
- **WHEN** a company sweeps every week through day 134, and another company loses every week through day 22
- **THEN** the first holds 1175, 1174, 1177, 1176, then 1167, 1165, 1168, 1169, then 1171, 1170, 1173, 1172, then 1178, 1179, 1180, 1181, then 1175, 1174, 1177, 1176 again, and the second holds none

### Requirement: Listed price
A listed price SHALL be the square of one more than half of six skill levels, and that half stops at 5, times 12, plus four fifths of the gear rounded to the nearest copper. The gear SHALL be the sum of the shop line prices. No skills and no gear SHALL cost 12. A purse below the price SHALL refuse before a full roster is considered. A full roster or a second specialist SHALL refuse without spending. A hire that joins SHALL subtract the price and add it to the amount spent.

#### Scenario: Rank, gear, and a short purse
- **WHEN** an empty candidate is priced, a skill total of 2 is priced, a skill total of 10 and of 12 are priced, a gear line of 7 is priced, a two-silver line is priced, a company with 47 copper hires the skill total of 2, a full roster with no copper hires, that roster is given 100 copper and hires again, and a company hires the skill total of 2, then a physician, then a second physician
- **THEN** the prices are 12, 48, 432, 432, 18 and 31, the short purse stays at 47 with an empty roster, the empty purse is refused with the roster still full, the second try is refused with 100 copper and the spent total still 84, the skill total of 2 leaves 295 copper and 48 spent, and the second physician is refused with the spent total still 60

### Requirement: Unwatched draw
A counted team SHALL score (count times 0.5 plus 0.5) times its strength sum. An aside fighter SHALL be left out, and mode 16 SHALL leave out the player's team. A weight SHALL be the square of that score over the largest, times 100, rounded to the nearest even integer. The roll SHALL become roll times 0x08088405 plus 1. The high half of that word times one more than the weight total SHALL pick the first team whose weights reach it. No counted fighter SHALL step the roll and name no team.

#### Scenario: A pair outscores a single until the roll passes it
- **WHEN** teams of strength 4, then 2 and 2, then 10 are drawn from rolls 1, 4 and 12, that list is drawn in mode 16 for team 1 from roll 1, a strength of 100 set aside is drawn from roll 1, and an empty list is drawn from roll 1
- **THEN** the winners are teams 1, 2 and 3 at rolls 0x08088406, 0x20221015 and 0x6066303d, mode 16 names team 2 at roll 0x08088406, the aside draw names team 1 at that roll, and the empty list names nobody at that roll

### Requirement: Losing bracket
A slot whose id is above 0 SHALL add its strength to the team. The team SHALL score (count times 0.5 plus 0.5) times that sum. The first group SHALL keep a forced id above 0 without stepping the roll. Any other resolution SHALL step the roll by times 0x08088405 plus 1 and keep the first team whose rounded weights pass the high half of the weight total times that word. Every other counted id SHALL become 0. More than one group SHALL say so.

#### Scenario: A forced team stays and the next group is drawn
- **WHEN** width 1 and two teams, ids 5 and 9 then 3 and 7, are cleared with force 5 from roll 1, one group of ids 1 and 9 is cleared from rolls 1 and 100, a width of 2 with a zero id of strength 100 beside id 1 against two ids of 3 is cleared from roll 4, and an empty list is cleared from roll 7
- **THEN** the two groups leave ids 5, 0, 3 and 0, step the roll to 0x08088406, and report another group, roll 1 leaves id 1, roll 100 leaves id 9 at roll 0x235391f5, the zero id stays 0 while id 1 is cleared and both ids of 3 stay at roll 0x20221015, and the empty list reports no other group with roll 7

### Requirement: Rival field
A tournament field SHALL take 16 companies from the 19, numbered 1 through 19, and SHALL not repeat one. Each draw SHALL step the roll by multiplying by 0x08088405 and adding 1. The high half of how many remain times that word SHALL choose the bag slot. The id SHALL be one more than the value there. A slot that is not last SHALL close by copying each later value down one.

#### Scenario: Roll 0 and roll 1 leave three companies out
- **WHEN** a field is drawn from roll 0 and another from roll 1
- **THEN** roll 0 yields 1, 2, 17, 6, 8, 14, 9, 4, 11, 12, 3, 13, 5, 19, 7, 15 and ends at 0x4b157bf0, and roll 1 yields 1, 17, 5, 7, 14, 8, 4, 10, 11, 2, 13, 3, 18, 6, 12, 19 and ends at 0xead32bb1

### Requirement: Daily match choice
Six daily matches SHALL be chosen from seven templates. Each try SHALL step the roll by multiplying by 0x08088405 and adding 1, and the high half of that word times 7 SHALL choose. A limit above 0 that is already reached SHALL refuse it. A slot before the template's earliest slot SHALL refuse it. A second step SHALL refuse it when the mask shares a bit with the roll's top three bits.

#### Scenario: Rolls 0 and 1 agree, and roll 4 does not
- **WHEN** the daily board is filled from rolls 0, 1 and 4
- **THEN** rolls 0 and 1 both choose 0, 1, 4, 1, 2, 3 and end at 0x1210d4e5, and roll 4 chooses 0, 1, 6, 4, 4, 1 and ends at 0xb761a41e

### Requirement: Counter roll
A performance counter SHALL replace the day's roll before the six templates are chosen. Only the counter's low 32 bits are kept, and those bytes are reversed. Counter 0x04000000 SHALL choose the roll-4 row and end at 0xb761a41e. Counter 0 SHALL choose the roll-0 row and end at 0x1210d4e5. Bits above the low 32 SHALL not change that row.

#### Scenario: The counter replaces a roll of 99
- **WHEN** a roll of 99 is replaced by counter 0x04000000, by counter 0, and by counter 0x100000000 plus 0x04000000
- **THEN** the first and the third choose 0, 1, 6, 4, 4, 1 and end at 0xb761a41e, and the second chooses 0, 1, 4, 1, 2, 3 and ends at 0x1210d4e5

### Requirement: Daily fee scale
A placed match SHALL copy the template, then multiply the fee and the prize by one more than the slot. Bit 2 SHALL keep the slot as the tier. Without it the tier SHALL be the slot times one minus the fourth power of the unit, rounded nearest even. A negative tier SHALL become -1. Kind 8 SHALL show 0 and 5. Bit 4 SHALL step the roll and store one more than the high half of that word times the slot span. The spans are 8, 11, 10, 14, 4 and 4.

#### Scenario: Slot 3 quadruples the coins and kind 8 shows 5
- **WHEN** a fee of 3 and a prize of 12 are placed at slot 3 with bit 2, the same template of kind 8 is placed at slot 3, a tier of -4 is placed at slot 2 with bit 2, a clear flag at slot 4 is placed with units 0 and 1, and bit 4 at slot 0 is placed from roll 0x10000000
- **THEN** the fee is 12 and the prize is 48, the tier is 3, kind 8 shows 0 and 5, the negative tier is -1, unit 0 leaves tier 4, unit 1 leaves tier 0, and the extra is 3 at roll 0x50000001

### Requirement: Unit draw
A unit generator SHALL fill 624 words from a seed, then twist before the first word. Mixing shifts by 11, 7, 15 and 18. Seed 5489 SHALL yield words 0xd091bb5c and 0x22ae9ef6. The unit SHALL be that word times 2 to the power -32. A mark that differs from the stamp SHALL seed from the mark and reverse the mark.

#### Scenario: Seed 5489 and a mark that does not match
- **WHEN** a generator is seeded with 5489 and two words are drawn, and another seeded with 99 has its mark set to 5489 and its stamp left at 0
- **THEN** the words are 0xd091bb5c and 0x22ae9ef6, the first unit's bits are 0x3fea12376b800000, the index is 2, and the mismatched draw yields 0xd091bb5c with the mark and the stamp reversed

### Requirement: Plain field copy
A record copy SHALL copy the bytes before each plain field, the field itself, and the tail through the record size. A short buffer, a field that starts before the previous end, or a field past the size SHALL be an error. The source SHALL stay unchanged.

#### Scenario: Gaps, a field, and the tail
- **WHEN** eight bytes 0x11 through 0x88 are copied with a field at 1 of width 2 and a field at 5 of width 1, a three-byte buffer is copied as size 8, and a field at 0 follows a field at 1
- **THEN** the copy is 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, the source is unchanged, and both bad copies are errors

### Requirement: Specialist leave
Before a match the company SHALL walk the roster from the start and skip every combatant. The first specialist with health under 700 SHALL be removed and the death count SHALL rise by one. Health from 700 through 64999 SHALL be removed without that rise. Health of 65000 or more SHALL stay. A later specialist SHALL stay on that pass. An empty roster SHALL remove nobody. Releasing a kit and unlinking a world character are not part of this step.

#### Scenario: The first unhealthy specialist leaves
- **WHEN** an empty company is asked, a hired physician at health 699 is asked, a hired merchant at 700 is asked, a hired trainer at 65000 is asked, a hired combatant at 0 stands before a hired physician at 699, and a hired physician at 100 stands before a hired merchant at 100
- **THEN** the empty company removes nobody, the physician at 699 is gone and the death count is 1, the merchant at 700 is gone and the death count stays 0, the trainer at 65000 stays, the combatant at 0 stays while that physician is removed, and only the physician of the pair is removed

### Requirement: Arena match begin
In-game menu command 4, while the mode is arena, SHALL remove the first unhealthy specialist, ask for a save that does not abort on low health, mark the hub busy, clear the world click hook, and set the session phase to transitioning. An empty roster SHALL still do that. Story mode, practice mode, another command, and another page SHALL leave the company and those flags unchanged. Closing windows, writing a save package, and the transition are not part of this step.

#### Scenario: Arena command 4 leaves the physician and arms the match
- **WHEN** an empty company is begun from in-game command 4 in arena, a hired physician at 699 is begun the same way, and that physician is offered to story mode, to practice mode, to command 1, and to another page
- **THEN** the empty company still marks the hub busy, clears the click hook, asks for the save, and sets the phase to transitioning, the physician is removed and the death count is 1, and the other four offers leave the physician and the flags unchanged

### Requirement: Marked window dismiss
A match begin SHALL walk two child lists from the last child to the first, before the specialist leave. A child whose flags masked by 0xce100000 equal that mask SHALL be dismissed. Any other child SHALL stay. A count of 0 or less SHALL dismiss nobody, and a child past the count SHALL stay. A command that does not begin SHALL dismiss nobody. The method on a dismissed child, and the call after the two lists, are not part of this step.

#### Scenario: Marked children go before the physician leaves
- **WHEN** in-game command 4 in arena walks a first list of an unmarked child, a child flagged 0xce100000 and a later marked child with count 2, then a second list of a fully flagged child and a child flagged 0xce000000, and the same lists are offered to story mode, and a marked child is offered with count 0
- **THEN** the dismissed names are the 0xce100000 child and then the fully flagged child, the later marked child and the 0xce000000 child stay, the physician is removed, story dismisses nobody, and count 0 dismisses nobody

### Requirement: Dismissed child close
A dismissed child SHALL close with argument 0 unless it is already closed. An already closed child SHALL not run its close callback and SHALL not close its own children. A pinned child SHALL be marked closed and raise a notice, then still close its children. Children SHALL close from the first to the last before the parent's callback. The capture release, the task's other children, detaching, redraw, and the call after the two lists are not part of this step.

#### Scenario: A pinned window closes its first child only
- **WHEN** a match begin dismisses a pinned window whose first child has a callback and whose second child is already closed, and a second begin dismisses a window that is already closed and has an open child
- **THEN** the callback order is the first child then the parent, the second child does not run, the notice is raised, argument 0 is stored, the already closed window runs no callback, and its child stays open

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
The experience pool SHALL be the template amount times one plus a quarter of the tier, converted once to the nearest even integer, times the share of opponent health dealt (every body's in the Fray), converted again, and half again (truncated) when a trainer is hired. That pool SHALL be split among the company's combatants by their contribution counts, each share the pool times the contribution over the total divided in double and stored as single, converted to the nearest even integer.

#### Scenario: Two fighters
- **WHEN** two combatants win a duel at tier 0 after dealing all the health, with contributions 1 and 3, then a trainer is hired and they win a duel at tier 4 after dealing all the health, with equal contributions
- **THEN** the first result credits 250 and 750, and the second adds 1500 to each

#### Scenario: Unrounded tier factor
- **WHEN** a tier 3 match of 5000 experience ends with the opponents missing 130 of 200 health and contributions 2 and 3
- **THEN** the pool is 5688, the shares are 2275 and 3413, a trainer makes the pool 8532, and the Fray's 160 of 400 makes it 3500

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

### Requirement: Form close remainder
A close SHALL release a held capture and clear style bit 0x20, even when already closed. A miss SHALL leave the bit. A hover on this child SHALL clear. Unclosed task children that name it SHALL close from first to last before its own children. A shown child SHALL hide, and the redraw word SHALL gain bits 0 and 1. The active form SHALL clear that pointer and SHALL not ask. Cursor restore and the later list call are not in this step.

#### Scenario: A captured window releases, hides, and redraws twice
- **WHEN** a match begin closes a captured shown window that is the hover and owns one sibling, and a second begin closes an already closed captured window, and a third begin closes the active form
- **THEN** the first drops the capture, clears the bit and the hover, closes the sibling, hides, and the redraw word is 3, the second drops its capture and does not ask, and the third clears the active pointer and does not ask

### Requirement: Cursor restore
Releasing a capture that is not the last SHALL publish the remaining top's code and name. Bit 0x8000 on that code SHALL pass its low byte as the shape and SHALL not mark the default. Any other code SHALL pass shape 0 and mark the default. An empty list SHALL publish code 0 and no holder, and SHALL not mark the default. The shape table is not part of this step.

#### Scenario: The lower capture is published
- **WHEN** a match begin releases a top capture and leaves a code of 0x15, another leaves a code of 0x8007, and a third releases the only capture
- **THEN** the first publishes code 0x15, the lower name, and shape 0 with the default marked, the second publishes code 0x8007 and shape 7 without the default, and the third publishes code 0 and no holder without marking the default

### Requirement: List follow
After the two lists, argument 1 SHALL run when a focus is set or when the mask shares bit 0. The mode word SHALL keep bit 1 and then set bit 0, and the stamp SHALL copy the clock. A ready hand list SHALL copy the ten stored words. A focus SHALL latch and clear the pending byte. No focus and a mask of 0 SHALL change nothing. The virtual call is not part of this step.

#### Scenario: Mask 1 copies the ten words
- **WHEN** a match begin follows the lists with mask 1, clock 99, mode word 6, and a ready hand list, and another begin uses mask 0 and no focus, and a third uses a focus with mask 0
- **THEN** the first copies 0x101, 0x20101, 0x102, 0x105, 0x10106, 0x107, 0x2010b, and three zeros, the mode word is 3, and the stamp is 99, the second leaves the hands unset, and the third latches and clears the pending byte

### Requirement: Cursor shape
Shape 0 SHALL show the default image when it differs and is not zero, mark the default, and stamp the clock. Shape 1 and up SHALL use the record one before that index. A different non-zero image SHALL be shown once. The same image SHALL not be shown again. A hook on that record SHALL run. A shape past the table SHALL leave the current image. Drawing the image is not part of this step.

#### Scenario: Shape 7 shows its image and runs its hook
- **WHEN** a match begin restores shape 7 with image 42 and a hook, the same image is restored again, shape 0 restores default image 9, and shape 7 is restored with an empty table
- **THEN** the first shows 42 and runs the hook, the second does not show again and still runs the hook, shape 0 shows 9 and marks the default, and the empty table leaves the current image

### Requirement: Focus call
A focus SHALL be invoked once before the hand words are copied. A session mark that is not already 0 SHALL become 0 after that copy. A mask without a focus SHALL not invoke it. The focus method body and the rest of the session-mark path are not part of this step.

#### Scenario: A focus is invoked before the hands
- **WHEN** a match begin follows the lists with a focus, a session mark of 1, and a ready hand list, and another begin uses mask 1 and no focus
- **THEN** the focus was invoked once, the hands are copied, the session mark is 0, and the mask-only begin does not invoke the focus

### Requirement: Control bank
When a focus clears a session mark that is not already 0, a flag SHALL become 0 and a code SHALL become 4. Side 0 SHALL copy sixteen words from the first bank when its flag is not 0, and then clear that flag. Any other side SHALL raise a notice and copy the second bank when the flag is 0, and then set the flag. A refused copy SHALL leave the words. An install after the copy SHALL be counted. The install body is not part of this step.

#### Scenario: A cleared mark copies one bank
- **WHEN** a match begin follows a focus whose mark is 1, on side 0 with a set flag, and another begin uses side 1 with a clear flag, and a third finds the mark already 0
- **THEN** the first copies the first bank and sets the code to 4, the second copies the second bank and raises the notice, and the third leaves the code and the words

#### Scenario: A matching flag refuses the copy
- **WHEN** side 0 finds the flag already 0, and a begin with no focus uses mask 1
- **THEN** the words stay, the focus still counts an install, and the mask-only begin copies nothing

### Requirement: Install body
A ready index below 1 SHALL return when the current index is already 0. Otherwise it SHALL clear that index, show a fallback when it differs and is not zero and a block word is 0, arm, and stamp the clock. A higher index SHALL return when it matches the sign-extended current index. Otherwise it SHALL store the index, show the record before it on that rule, and run its hook. A zero value, the same value, a block, and an index past the table SHALL not show. The platform call is counted.

#### Scenario: Index 1 shows its record and runs its hook
- **WHEN** a match begin installs index 1 with value 42 and a hook, the same index is installed again, index 2 has value 0 and a hook, and a block word refuses value 42
- **THEN** the first shows 42 and runs the hook, the second does not show or run it again, the zero value runs the hook without a show, and the block word leaves the shown value

#### Scenario: Index 0 shows the fallback
- **WHEN** a match begin installs index 0 while the current index is 5, the fallback is 9, and the clock is 12, and another begin is not ready
- **THEN** the current index is 0, fallback 9 is shown, the armed byte is set, the stamp is 12, and the unready begin leaves the index

### Requirement: Platform draw
A shown value SHALL be passed with the two platform words. The same value, a zero value, and a block word SHALL not pass it. The indirect call is counted. Its target is not called.

#### Scenario: Value 42 is drawn with the two words
- **WHEN** a match begin shows value 42 with owner 7 and extra 8
- **THEN** those three words are the draw, and the show is counted once

### Requirement: World side
An open world SHALL set the side to 1 when its byte is 0, and to 0 otherwise, before the later select. A lock SHALL leave that side. Otherwise a force SHALL use 0, and a differing byte SHALL be stored, select its bank, clear a span and mask the flags when it is not 0, and be written back. Callback registration is not part of this step.

#### Scenario: A world byte selects the bank
- **WHEN** a match begin clears a mark with world byte 0 and a set flag, byte 5 and full flags, byte 5 forced, and byte 0 locked
- **THEN** byte 0 copies the first bank and installs twice, byte 5 copies the second bank and masks the flags, the force leaves byte 5 and selects the first bank once, and the lock leaves byte 0 while selecting the second bank

### Requirement: Session aside
Passing 1 SHALL store 1, set a flag to 1, and set a code to 0 when the mark is not already 1. An open world SHALL clear a quiet byte and the flags, and store code 4 on the child. Three notice bytes SHALL become 0. Index 0 SHALL be installed and neither bank copied. A hold byte SHALL become 0, a view word SHALL gain bit 0x10, and a view code SHALL become 0. A mark that is already 1 SHALL change nothing.

#### Scenario: Raising the mark installs index 0
- **WHEN** the mark is 0, the current index is 5, and the fallback is 9, and the raise is repeated
- **THEN** the mark is 1, fallback 9 is drawn, the bank is unchanged, and the second raise leaves the code

### Requirement: Hook list
A cleared mark SHALL register code 0x8001 when the hold byte is 0, and SHALL register nothing when it is not. A code with a clear high nibble SHALL gain bit 0x1000. A non-zero world byte SHALL register the wide hook at code 0x2001 and at the small code, then drop the plain and lab hooks. Raising the mark SHALL drop the plain, lab, wide, and gate hooks, drop the play hook, and insert the watch hook at key 4. An empty list starts one below 0.

#### Scenario: A cleared mark registers the gate hook
- **WHEN** a match begin clears a mark with hold 0 and gate hook 11, another begin holds the byte, and a third uses world byte 5
- **THEN** the first list is code 0x8001, the held begin registers nothing, and byte 5 stores the wide hook at 0x2001 and at 0x1002 before the gate hook

#### Scenario: Raising the mark drops the hooks
- **WHEN** the primary list holds the plain, lab, and gate hooks and the ordered list holds the play hook
- **THEN** the primary list is empty and the ordered list is the watch hook at key 4

### Requirement: Action set
A shown value SHALL activate the action set with the two platform words. The same value SHALL not activate it again. The Steam library is not loaded.

#### Scenario: Value 42 activates the action set
- **WHEN** a match begin shows value 42 with owner 7 and extra 8
- **THEN** the action set is activated once and those three words are the draw

### Requirement: Gate hook
The gate hook SHALL clear two result words and set the result flags to 1. A set mid byte with an alternate hook SHALL count that hook. A clear mid byte with a closed world SHALL leave the flags at 1 and SHALL not count the alternate hook.

#### Scenario: A set mid byte counts the alternate hook
- **WHEN** the gate hook runs with empty results, and again with a set mid byte and an alternate hook
- **THEN** both runs set the flags to 1, the first counts nothing, and the second counts the alternate hook

### Requirement: Watch hook
A watch without bit 0x2000 SHALL store a stamp of 0. With that bit, a target, and a ready byte, a held byte of 0 SHALL copy the clock. Code 1 SHALL clear bit 0x800000 when it is set and count a release. Code 2 SHALL set that bit and count a press. A pending count above 0 SHALL count a refresh. A leave with a focus SHALL clear the focus and enter the session when the mark is not 0. The hook SHALL return 1.

#### Scenario: A watch stamps, releases, and enters
- **WHEN** a watch without the bit has stamp 9, a ready watch with code 1 and the mark bit has clock 12 and a pending count, and a leave has a focus and mark 2
- **THEN** the first stamp is 0, the second stamp is 12 and the mark bit is clear, and the leave clears the focus and enters the session

### Requirement: Cursor pick
A clear mid byte SHALL count a pick when the world is open, the busy byte is 0, the alternate hook is 0, the hold byte is 0, and a set bank side has a world aim. A taken item SHALL publish the notice owner and OR the flags with 5. A set kind SHALL count a kind call. An actor whose parent equals the world SHALL count an open. Kind 0xa00 SHALL count a chest. A named row SHALL count an open. Bit 0x800 SHALL not. A set hold byte or a busy byte SHALL not pick. The raycast is a supplied result.

#### Scenario: A clear gate picks a taken item
- **WHEN** the gate hook runs on an open world with a taken item and notice owner 6, again with the hold byte set, and again with a busy byte
- **THEN** the first ORs the flags with 5 and publishes owner 6, and the other two do not pick

#### Scenario: An actor parent counts an open
- **WHEN** an actor parent equals the world, a kind word is 0xa00, a named row is set, and a row also has bit 0x800
- **THEN** the world parent and the named row each count an open, the kind word counts a chest, and bit 0x800 counts nothing

#### Scenario: A set kind counts a kind call
- **WHEN** an open world picks an item whose kind is set and that is not taken
- **THEN** the kind call is counted once and the flags stay 1

#### Scenario: A set bank side needs a world aim
- **WHEN** the gate hook runs on an open world with bank side 1 and no world aim, and again with a world aim
- **THEN** the first does not pick and the second counts one pick

### Requirement: Click hook
The click hook SHALL clear its flags. An open world whose aim bits are above 0x428c0000 SHALL copy the aim place. A miss with the hold byte set SHALL clear the aimed word. A taken item SHALL set flags that include 0x11. Aim bits equal to 0x428c0000 SHALL not copy the place. The rest of the click path is not part of this step.

#### Scenario: A fast aim copies and a taken item marks the click
- **WHEN** a click runs on an open world with aim bits above 0x428c0000, a miss, and the hold byte, and again with aim bits equal to that value and a taken item
- **THEN** the first copies the place and clears the aimed word, and the second sets flags 0x11 and does not copy

### Requirement: Hover hook
The hover hook SHALL publish a follow hook and the self owner, clear the hover byte and the hover mark, and store the flags 0x15.

#### Scenario: A hover stores flags 0x15
- **WHEN** the hover hook runs with owner 3, byte 4, and mark 6
- **THEN** the flags are 0x15, the follow hook is published for owner 3, and the byte and the mark are 0

### Requirement: Focus before hands
A focus begin SHALL count one call before the hand words are copied. The hands SHALL be set and the stamp SHALL copy the clock. The focus method body is not part of this step.

#### Scenario: A focus begin counts one call
- **WHEN** a focus begin copies the hands at clock 5
- **THEN** the focus is counted once, the hands are set, and the stamp is 5

### Requirement: Row walk
When the named check is clear, the gate SHALL open on the first record whose kind is 0x10010a or 0x10010d and whose link is the world. A different kind or a different link SHALL not open. Two matching records SHALL open once. A set named check SHALL open without a matching record.

#### Scenario: Kind 0x10010a linked to the world opens once
- **WHEN** an actor lists another kind, then kind 0x10010a linked to the world, then that kind again
- **THEN** the gate opens once

#### Scenario: A different link does not open
- **WHEN** kind 0x10010d is linked elsewhere, and then that link becomes the world
- **THEN** the first run does not open and the second opens once

#### Scenario: A named check opens without a matching record
- **WHEN** the named check is set and the only record has another kind
- **THEN** the gate opens once

### Requirement: Kind table
An item id of 0 SHALL not count a kind call. Any other id SHALL search the high table when bit 0x80000000 is set, else the wide table when bit 0x40000000 is set, else the plain table. A high or wide id SHALL drop those two bits before the search. A found row whose span holds 0x20000 SHALL count one kind call. A missing key, a size of 0, a short span, or a span without that mark SHALL not. A step of 0 stops the walk.

#### Scenario: A plain id counts when the span holds the mark
- **WHEN** an open world picks item 7 whose plain row holds 0x20000, and another pick of 7 does not
- **THEN** the first counts one kind call and the second counts none

#### Scenario: The high bit chooses the high table
- **WHEN** id 0x80000005 has an unmarked plain key 5 and a marked high key 5, and id 0xC0000005 has those tables reversed
- **THEN** the first counts one kind call and the second counts none

#### Scenario: The wide bit drops itself before the search
- **WHEN** id 0x40000009 has a marked plain row of that whole id and an unmarked wide key 9, and the wide row is then marked
- **THEN** the first counts none and the second counts one kind call

### Requirement: Kind release
A cursor below 0, or at the view width or height, SHALL clear the hit and SHALL not count a kind call. A cursor inside SHALL keep a supplied hit. A marked item SHALL then release: a count above 0 drops and becomes 0, and a count of 0 or less clears a pointer, counting a close when the byte is 1. With no matching pane a marked span counts a build. The first pane tagged 0xce1c2013 for that item is shown, and a later pane is not. The ray distance and child layout are not part of this step.

#### Scenario: A cursor on the view edge does not release
- **WHEN** the cursor is at width 8 on a view of 8, the item is marked, and the held count is 2
- **THEN** the hit is clear, the kind call is not counted, and the count stays 2

#### Scenario: A marked item drops a count and shows the first pane
- **WHEN** a count of 3 is released, a pointer with byte 1 is released, and a view corner shows the second of two tagged panes
- **THEN** the count becomes 0 with one drop, the pointer is cleared with one close, and pane 11 is shown once

### Requirement: Scene walk
When the scene has sectors, a sector that shares no camera bit SHALL be skipped. A body SHALL be visited only when its flags include 0x89, it shares a camera bit, and it lacks bit 0x4000000. A clear probe SHALL record the bits of 0.1, and any other probe the bits of 5. The ray SHALL start at 4000 along z. A centre past that length SHALL miss. A closer hit SHALL shorten the ray so a farther centre is left. A meeting hit SHALL replace the stored hit. An empty scene SHALL leave the supplied hit.

#### Scenario: The last positive body is the hit
- **WHEN** one sector shares no camera bit and another holds a skipped body, a probe of 1 with three hits, a body without 0x89, and a clear probe with two hits
- **THEN** the hit is the last body, the stored index is 1, and the pads are the bits of 5 then of 0.1

#### Scenario: A farther centre is left
- **WHEN** a centre at 100 is followed by a centre at 500, and another scene holds only a centre at 5000
- **THEN** the first hit stays at 100 with reach 100, and the centre at 5000 misses

### Requirement: Pane size
A new pane SHALL double the row one before the slot and cap it when greater. Bit 0x1000 SHALL choose the wide slot. Inner height SHALL be the unit times 6 plus the rows. Each other inner size SHALL be twice the margin plus that span word times the unit. Width SHALL be twice the pad plus the first inner size. Height SHALL be the unit times 2 plus the other sizes and the pad.

#### Scenario: Slot 1 sizes the pane
- **WHEN** a marked item builds with unit 2, margin 5, pad 6, span words 3 and 4, row 9, and a cap of 100
- **THEN** the rows are 18, the width is 28, the height is 58, and the item is stored

### Requirement: Pane place
The place SHALL sit half a width left of the cursor. Below 0 it SHALL become 0. Past the screen it SHALL become the screen width minus the pane. The other place SHALL be at least the unit. The order SHALL be the base plus 0x100. The item id SHALL be stored.

#### Scenario: The place is clamped to the screen
- **WHEN** the cursor is 100 on a screen of 200, and again 30 on a screen of 20, with the same pane width of 28
- **THEN** the first place is 86 and the second is -8

### Requirement: Child layout
A stretch child SHALL grow by the pane's change from its previous size. An anchor child SHALL move by that change. A centre child SHALL be placed at half the spare. Nested children SHALL not be visited.

#### Scenario: Three children take their places
- **WHEN** a pane of width 28 and height 58 lays out a centre child of 10 by 4, an anchor child at x 4, and a stretch child of width 10
- **THEN** the centre sits at 9, 27, the anchor moves to 32, and the stretch width is 38

### Requirement: Arena match templates
The system SHALL provide the six standard, seven ranked and five tournament templates with the original's names, ids, flags, teams, fighters, rounds, cash, XP, slot limits and descriptions.

#### Scenario: Three templates
- **WHEN** template 3 is read
- **THEN** it is Skirmish with 3 teams, 1536 cash and 3900 XP
- **WHEN** template 11 is read
- **THEN** it is The Beast with 196608 cash, 10000 XP, minimum slot 2 and 1 per pass
- **WHEN** the name of id 11 is looked up
- **THEN** it is the lower-case "beast"

### Requirement: Arena money and prizes
The system SHALL pack cash as three base-12 bytes, pay `unpack(cash)·(slot+1)`, and compute the XP pot as `trunc(xp·(1+0.25·slot))`, scaled by the survivors' health, doubled by half for a manager and split by damage.

#### Scenario: A duel
- **WHEN** Duel is fought at slot 3
- **THEN** its pot is 1750
- **WHEN** half the health remains with a manager
- **THEN** the award is 1312
- **WHEN** two fighters dealt equal damage
- **THEN** they split the total equally

### Requirement: Arena ranks and wages
The system SHALL compute rank points as `1+Σxp/5000`, the loadout tier as `min((rank−1)/2, 5)`, one award as `min(total/rank, 5000)`, and wages as rank−1, 2, 12 or tier+1 by class mask.

#### Scenario: Ranks and wages
- **WHEN** the pools hold 5000, 4999 and 10000
- **THEN** the rank points are 4
- **WHEN** the class mask is 0x50000000
- **THEN** the wage is 12

### Requirement: Arena condition and calendar
The system SHALL recover 0x1999 or 0x2666 condition (saturating), treat below 0xCCC as unfit, start a week on day 1 mod 7, end it on day 0 and divide the season into four weeks.

#### Scenario: Recovery and phases
- **WHEN** condition 1000 recovers with a physician
- **THEN** it is 10830
- **WHEN** the day is 27
- **THEN** it is week 3

### Requirement: Arena rank roll
The system SHALL roll a template's rank to −1 when unranked, to the slot when flag 2 is set, and to `trunc(slot·(1−r⁴))` otherwise; Challenger SHALL open a 0–5 window and others the slot to the rolled rank.

#### Scenario: Ranked and unranked
- **WHEN** Elimination is rolled at slot 3 with r 0.5
- **THEN** its rank is 2 and its window is 3–2
- **WHEN** Brawl is rolled at slot 4
- **THEN** its rank is −1
- **WHEN** Challenger is rolled at slot 0
- **THEN** its window is 0–5

### Requirement: Arena generation
The system SHALL accept a template only when its per-pass cap is not reached, the slot is at least its minimum and the 0–7 roll misses the reject mask; draws SHALL use the engine generator and retry.

#### Scenario: Filters
- **WHEN** Valiance is generated at slot 0
- **THEN** it is rejected
- **WHEN** its cap of 2 is already used
- **THEN** it is rejected
- **WHEN** Brawl rolls 1 against its mask 3
- **THEN** it is rejected

### Requirement: Arena settlement
The system SHALL pay only on a win, log 0x20000/0x40000/0x30000 for a win/practice/other, count a Beast win and always count the match played; the season armour set SHALL need the last week with all four wins.

#### Scenario: Win and season
- **WHEN** a Beast match is won
- **THEN** it pays, logs 0x20000 and counts the beast
- **WHEN** week 3 has all four weekly wins
- **THEN** the last season set is awarded

### Requirement: Arena loadout tiers
The system SHALL provide the six loadout tiers with 8, 11, 10, 14, 4 and 4 rows, including the multi-variant Axe, War Hammer and Spiked Mace rows, and the point limits 5, 10, 15, 20, 30 and 40.

#### Scenario: Tiers and variants
- **WHEN** tier 2 is read
- **THEN** it has 10 rows and the Axe row has variants 19, 419 and 213
- **WHEN** tier 5 is read
- **THEN** it repeats tier 4's four polearms
- **WHEN** the point limit of tier 5 is read
- **THEN** it is 40

### Requirement: Variant selection
The system SHALL draw one of a row's non-zero variants with the engine generator.

#### Scenario: A three-variant axe
- **WHEN** the generator draws index 1 from the Axe row
- **THEN** the chosen variant is 419

### Requirement: Rank roll rounding
The system SHALL roll a template's rank with round-to-nearest-even, not truncation.

#### Scenario: Elimination at slot three
- **WHEN** Elimination is rolled at slot 3 with r 0.5
- **THEN** its rank is 3 and its window is 3–3

### Requirement: Company profile order
The system SHALL store each company's three build bytes in memory order `[+8,+9,+10]`.

#### Scenario: Fitten Gang
- **WHEN** the second company is read
- **THEN** its bytes are 0x42, 0x8C, 0x01

### Requirement: Arena instance roll
The system SHALL copy a template into a match instance, roll its rank from the MT unit draw with nearest-even rounding, set the Challenger window and draw a weapon row for flag-4 templates on the LCG.

#### Scenario: Elimination and Challenger
- **WHEN** Elimination rolls at slot 3 with r 0.5
- **THEN** its rank is 3, its window 3–3 and its cash is the slot-scaled packed value
- **WHEN** Challenger rolls at slot 0
- **THEN** its window is 0–5
- **WHEN** Brawl rolls at slot 4
- **THEN** its rank is −1

### Requirement: Arena daily board
The system SHALL generate six daily rows from the seven ranked templates with the cap, minimum-slot and reject filters.

#### Scenario: Seed 12345
- **WHEN** the daily board is generated from seed 12345
- **THEN** the template ids are 8, 13, 8, 11, 13, 12

### Requirement: Arena tournament board
The system SHALL draw the weekly prize around the eight-index history, generate six tournament rows and shuffle the 19 companies into 16 slots on the same LCG.

#### Scenario: Shuffle
- **WHEN** the companies are shuffled from seed 0
- **THEN** the order is 1, 2, 17, 6, 8, 14, 9, 4, 11, 12, 3, 13, 5, 19, 7, 15

### Requirement: Arena roster
The system SHALL provide six 0x30-byte roster rows with two entrants, the 8-byte spawn profile, the unfit filter and the 2..6 round-id assignment.

#### Scenario: Profile and unfit
- **WHEN** tier 3 is spawned for company 15 unarmed
- **THEN** the profile is 0x0B, 3, 100, 10, 10, 4, 92, 0
- **WHEN** an entrant at 0x0CCB is filtered
- **THEN** it is removed and a 0x0CCC entrant is kept

### Requirement: Arena resolution
The system SHALL tally the rows by team id, award a player tally of 3+ without a tie, record company wins as the shuffled id+2, and grant the season set at `(day % 28) % 4` on week 3 with all four flags.

#### Scenario: Champion and company
- **WHEN** the rows are [1,1,1,2,3,2] on day 27
- **THEN** the player wins, the prize and last season set are granted, and championships and seasons rise
- **WHEN** the rows are [2,2,2,1,3,4]
- **THEN** no champion is recorded and week one holds company 15+2
- **WHEN** the rows tie at two each
- **THEN** no champion is recorded

### Requirement: MT19937 stream
The system SHALL provide the standard MT19937 generator and a unit draw of `next_u32 · 2⁻³²`.

#### Scenario: Known vector
- **WHEN** the generator is seeded with 5489
- **THEN** its first five outputs are 3499211612, 581869302, 3890346734, 3586334585 and 545404204

### Requirement: Arena rank stream
The system SHALL advance the MT stream only for a rank roll, not for flag-2 or unranked templates.

#### Scenario: Prerolled and rolled
- **WHEN** Challenger rolls at slot 0
- **THEN** the MT stream is untouched
- **WHEN** Elimination rolls at slot 3
- **THEN** exactly one unit draw is consumed

### Requirement: Opponent weapon
The system SHALL draw a tier's weapon row with the LCG and then one of its non-zero variants, with the row 0-based.

#### Scenario: Seed 12345
- **WHEN** tier 1 draws a row from seed 12345
- **THEN** the row is 5
- **WHEN** tier 2's row 1 draws a variant from seed 12345
- **THEN** the variant is 419

### Requirement: Opponent level
The system SHALL set a company opponent's level to 60000 + rand(9999) and a generic opponent's to 50000 + rand(9999).

#### Scenario: Bases
- **WHEN** a company opponent is spawned from seed 12345
- **THEN** the level is 60000 + the first rand(9999)
- **WHEN** a generic opponent is spawned
- **THEN** the level is 50000 + the first rand(9999)

### Requirement: Arena draw
A match SHALL step the roll by `× 0x08088405 + 1`. When the high half of the word times 6 is above the variety count plus 2 the arena SHALL be indoor and the count SHALL rise by one: the Beast SHALL step again and take arena5 when the high half of the word times 2 is 0, else arena7; any other match SHALL draw one of the seven indoor arenas not among the last five. Otherwise the count SHALL fall by one and one of the five outdoor arenas not among the last three SHALL be drawn.

#### Scenario: Arenas from a zero roll
- **WHEN** a fresh draw starts from roll 0, six matches are drawn in a row with their tracks, the Beast draws from rolls 16, 18 and 2, and a start reads counter 0x12345678
- **THEN** roll 0 draws arenao5 with count −1, outdoor list 4, 0, 0 and roll 0xdc6dac1f; the six matches draw arenas 11, 10, 8, 3, 7, 5 and tracks 2, 3, 4, 9, 5, 1 and end at roll 0x2630fbe0 with count −2; the Beast takes arena5 at roll 0x4e5b0596, arena7 at roll 0xf72555c8, and arenao4 from roll 2; the start draws arena5 and Msc_Arena3 and ends at roll 0x80b2188e

### Requirement: Unrepeated draw
An unrepeated draw SHALL step the roll, take the high half of the word times the count, and draw again while that value is listed. The list SHALL move one place along, its last entry SHALL fall off, and the value SHALL go first. The lists and the variety count SHALL start at zero, so index 0 is refused until it moves out.

#### Scenario: The first outdoor draw
- **WHEN** a fresh outdoor list draws from roll 0
- **THEN** the list becomes 4, 0, 0

### Requirement: Match music
A match SHALL step the roll and take the high half of the word times 11 until it is none of the last five tracks; the list SHALL move along and the track SHALL go first. The Beast SHALL play Msc_ArenaO, or Msc_ArenaO_DnB with the alternative music option, at 0.3 without stepping the roll or moving the list.

#### Scenario: A track skips the last five
- **WHEN** the last five tracks are 0 to 4 and the roll is 0, and the Beast starts from counter 0x10000000
- **THEN** Msc_Tourney01 at 0.15 plays, the list becomes 9, 0, 1, 2, 3 and the roll is 0xdc6dac1f, or Msc_Tourn01_DnB with the option; the Beast plays Msc_ArenaO at 0.3 and the list stays at zeros

### Requirement: Track table
The eleven tracks SHALL be Msc_AreanNov 0.1, Msc_AreanExp 0.15, Msc_ArenaBossN 0.1, Msc_ArenaBossE 0.07, Msc_Arena2 0.1, Msc_Arena3 0.15, Msc_Arena4 0.15, Msc_Fortitude 0.1, Msc_OreSum 0.15, Msc_Tourney01 0.15 and Msc_Tourney02 0.12. With the alternative music option they SHALL be the DnB tracks at 0.15, Msc_Fortitud_DnB at 0.2.

#### Scenario: Volumes
- **WHEN** the volumes of Msc_ArenaBossE, Msc_Tourney02 and Msc_Fortitud_DnB are read
- **THEN** their bits are 0x3d8f5c29, 0x3df5c28f and 0x3e4ccccd

### Requirement: Outdoor bed
An outdoor arena SHALL be silent when its rain is over 0.01 compared in double. Otherwise a time of day under 0.5 SHALL play NtrAN at 0.3. By day arenao1, arenao3 and arenao5 SHALL play NtrA at 1. Any other outdoor arena SHALL play NtrW at 1 when the wind is over 0.1 compared in double, with gain the double 0.2 times the wind squared and pitch the double 0.6 plus half the wind, else stay silent.

#### Scenario: Rain, night, steady and wind
- **WHEN** arenao1 has rain 0.02, rain 0.01 at time 0.4, and time 0.5; arenao2 has wind 0.05, wind 0.1 and wind 0.5
- **THEN** the rain is silent, 0.01 is not rain and plays the night bed, the day plays NtrA, wind 0.05 is silent, wind 0.1 plays NtrW with gain bits 0x3b03126f and pitch bits 0x3f266666, and wind 0.5 with 0x3d4ccccd and 0x3f59999a

### Requirement: Daily match limit
The board SHALL run a match while fewer than three were fought today. A match back at the hub SHALL count one more, except on a day that is a multiple of 7 with any fighter entered in the week's events.

#### Scenario: Three a day
- **WHEN** two and three matches were fought, and matches return on day 7 with entries, day 7 without and day 8 with
- **THEN** the board runs at two and not at three, day 7 with entries counts none, and the other two count two

### Requirement: Match rounds
A match SHALL start with no extra rounds and one life; Elimination SHALL set four extra rounds and Reserve two extra rounds and three lives. A won round SHALL lead to another fight while the round less the losses, in 64 bits, is below the extra rounds, else the match SHALL be won. A lost round SHALL count one loss and lead to another fight while the losses are below the lives, else the match SHALL be lost.

#### Scenario: Elimination and Reserve
- **WHEN** a standard match ends its round, Elimination wins five rounds or loses its third, and Reserve loses twice then wins three times
- **THEN** the standard match is won or lost at once; Elimination continues four times and is won on the fifth win and lost on a loss; Reserve continues four times, stands at round 4 with two losses, and is won on the next win; a third loss loses it; a dead manager goes to the menu

### Requirement: Continue
Continue SHALL go to the main menu when the manager is dead. It SHALL start the next round and count it when the round less the losses is below the extra rounds and the state is another fight, and otherwise go back to the hub.

#### Scenario: After a won match
- **WHEN** a won Elimination continues, and a Reserve match continues with a dead manager
- **THEN** the first goes back to the hub and the second to the main menu

### Requirement: Match tick
A match SHALL tick only once its start stamp plus 1000 ms, widened to 64 bits, is not after the game clock. A tick SHALL count every fighter on the player's team, the player's team still standing and everyone else still standing; a dead fighter SHALL stand for nobody, and a dead company fighter SHALL be resolved once. The round SHALL be lost when none of a non-empty player's team stands, else won when nobody else stands.

#### Scenario: Who stands
- **WHEN** a match started at 500 is asked at 1499 and 1500, a stamp at the top word is asked at 0, and a scene holds the player, a dead foe and a standing foe, then the standing foe dies, then the player dies
- **THEN** 1499 does not tick, 1500 ticks, the top stamp does not; the first count is 1, 1, 1 with no end, then the round is won, then the player is resolved once and the round is lost

### Requirement: Fall resolution
A fallen fighter SHALL be dead when its health widened to double is below 0.01, or with a physician hired when its health over its maximum (divided in single) widened to double is below −0.15. A dead manager (id 0) SHALL end the career; a dead hired fighter SHALL be logged with the name's sixteen bytes and counted as a death. A fighter that is not dead but below 0.01 SHALL be logged as saved with its id. NaN SHALL be neither.

#### Scenario: Dead, saved, survived
- **WHEN** health 0.01 and 0.0100001 fall without a physician, −0.0375 and −0.0374 of 0.25 with one, −0.03 and −0.04 with one at id 2, 0 at id 0, 0.2, and NaN
- **THEN** 0.01 is dead and 0.0100001 is not, −0.0375 is dead and −0.0374 is not, −0.03 is saved and −0.04 dies, id 0 ends the career, 0.2 and NaN survive; a death named Ash logs words 0x00687341, 0, 0, 0 and the death count is 1

### Requirement: Health word
After a match a record SHALL keep 0 when the health is not above 0, else the health over its maximum times 65535 converted in single to the nearest even 64-bit integer, kept to its low sixteen bits. Read back, the health SHALL be the maximum times the word times the double 1/65535, in double, stored as single.

#### Scenario: Half health
- **WHEN** health 0.125, 0.25, 0, −0.1, NaN and 0.5 of 0.25 are written, and words 32768 and 65535 are read at 0.25
- **THEN** the words are 32768, 65535, 0, 0, 0 and 65534, and the reads are bits 0x3e000080 and 0x3e800000

### Requirement: Forfeit and return
A forfeit SHALL do nothing when the manager is dead, and otherwise log the forfeit with the template id outside a tournament. Back at the hub outside a tournament, a win SHALL pay the prize, log the win and count a Beast win as a beast defeated; a forfeit and a loss SHALL be logged. Tournament or not, a win SHALL count one more match won; a forfeit SHALL count one more forfeit and every finish but a win one more loss.

#### Scenario: A Beast win, a forfeit, a tournament win and a loss
- **WHEN** the Beast starts and is won for 432, a duel is forfeited, a tournament event is won and a duel is lost
- **THEN** 432 is paid and logged as a win of 11 with one beast fought and defeated, the forfeit is logged and counts one forfeit and one loss, the tournament win pays and logs nothing but counts a win, and the loss is logged; the records read 2, 2, 1

### Requirement: Company records
A Beast match SHALL count one more beast fought when it starts. The records SHALL be listed as Matches won, Matches lost, Matches forfeited, Tournaments won, Tournaments lost, Seasons won, Beasts fought, Beasts defeated, Total Hires, Deaths, Fired, Total earned, Total spent.

#### Scenario: Beasts fought
- **WHEN** the Beast and a duel start
- **THEN** one beast is fought, and the tenth label is Deaths

### Requirement: Hire list
A candidate SHALL seed the roll from the counter's byte-reversed low word and step it: the high half of the word times 20 at 0 SHALL be a physician, else after a step times 15 at 0 a trainer of level `round(u² × 2) + 4`, else after a step times 10 at 0 a merchant, else a combatant of level `round(u² × 5)`, rounding ties to even, `u` the unit draw. Physicians and trainers SHALL be drawn older.

#### Scenario: Four professions
- **WHEN** candidates are drawn from counters 0, 23, 10 (units 0.5, 0.75, 0.9) and 3 (units 0.5, 0, 0.999999)
- **THEN** counter 0 is a physician at roll 1, 23 a merchant at 0x176dac1f, 10 trainers of levels 4, 5 and 6 at 0x02088406, and 3 combatants of levels 1, 0 and 5 at 0x536dac1f

### Requirement: Hire list renewal
A new career SHALL list ten candidates. Each day the oldest candidate SHALL leave, then one more than the high half of the stepped roll times 2 SHALL come while fewer than eight are listed, else one, newest first.

#### Scenario: Renewals
- **WHEN** lists of ten and five are renewed from rolls 0, 0 and again 0x80000000, and an empty list from roll 0
- **THEN** the ten keep ten with one new first, the five lose one and gain one at roll 1, then lose one and gain two at roll 0x80000001, and the empty list loses nobody and gains one

### Requirement: Trainer snap
A trainer's skill experience SHALL snap down to its level times 5000, and below level 5 the technique at that level SHALL be forgotten.

#### Scenario: Six skills
- **WHEN** a trainer holds 4500, 5000, 12345, 27000, 0 and 65535 with every technique set
- **THEN** it holds 0, 5000, 10000, 25000, 0 and 65000, and forgets the techniques at levels 0, 1, 2 and 0 of the first, second, third and fifth skills

### Requirement: Ranked loadouts
A combatant SHALL keep six loadouts copied from its kit. The loadouts of the ranks below its level SHALL empty both hands and drop the apparel whose rank, the top three bits of its rank byte, is above the loadout's rank; other items SHALL stay.

#### Scenario: A level-3 combatant
- **WHEN** a level-3 combatant's kit holds two weapons, apparel of rank bytes 0x40, 0x1f and 0xe0 and one other item, and a level-0 combatant holds the same kit
- **THEN** ranks 0 and 1 keep the rank-0 apparel and the other item, rank 2 adds the rank-2 apparel, those three have empty hands, ranks 3 to 5 keep the whole kit, and the level-0 combatant trims none

### Requirement: Arsenal shelves
Apparel SHALL go on the armour shelf when its layer type is 3 through 6, else on the clothing shelf; an item that is not apparel SHALL go on the weapons shelf when it is a weapon, else the shields shelf when it is a shield, else nowhere; an unknown item SHALL go nowhere. Stowing SHALL add the item and its flags to the shelf's first box. Releasing SHALL clear the mask from the first entry, box by box, that holds the item with a bit of the mask set.

#### Scenario: A sword stowed and released
- **WHEN** a sword is stowed with 0x600, released with 0x200 twice, then with 0x600, and a copy in the fourth box is released with 0x200
- **THEN** the weapons shelf's first box holds the sword with 0x600, then 0x400, the second release finds nothing, then 0, and the fourth box's copy is cleared

### Requirement: Running match
In play mode an arena level SHALL draw its track as a match start does, from the counter roll and the remembered last five, and play it at the track's volume. With an opponent the match tick SHALL run on the game clock: it SHALL wait for the first second, end the round when the opponent or the player is dead, and show end screen 3 when the player's fall is fatal, else 2.

#### Scenario: A duel in the running game
- **WHEN** the match starts at 500 ms with the foe dead and is updated at 500, 1499 and 1500 ms, and a match with the player dead is updated at 1500 ms
- **THEN** the stamp is 500, 1499 still fights, 1500 wins with no end screen; the dead player's match is lost, the player is resolved once, end screen 3 shows and nothing is logged

### Requirement: Staff ids
The manager SHALL be fighter 1 with id word 0x10000000. Each employee SHALL have the id word of its place plus one, with 0x10000000 added for a recruited fighter. A combatant or a recruit SHALL count as a fighter.

#### Scenario: Ids and fighters
- **WHEN** one combatant is hired, and eight fighters are recruited
- **THEN** there are 2 fighters and the combatant's id word is 1; the eight recruits make 9 fighters and the third's id word is 0x10000003

### Requirement: Hire
A hire SHALL be refused with "Insufficient funds" when the purse holds less than the price, with "You do not have room for any more fighters." when a combatant finds eight fighters, and with "You already have a merchant in your employ." (or a physician, or a trainer) when one is already employed. Otherwise the price SHALL leave the purse and join today's spending, the candidate SHALL leave the hire list, one more hire SHALL be counted and the joining SHALL be logged with the name.

#### Scenario: A combatant for 20
- **WHEN** a purse of 20 hires a combatant named Ash for 21, then for 20, seven combatants fill the company, an eighth is asked for, and two trainers are asked for
- **THEN** 21 is refused for funds, 20 leaves 0 with 20 spent, one hire and a joining logged with 0x00687341; the eighth combatant finds no room, the first trainer is hired and the second is refused

### Requirement: Hire kit
A hired combatant's kit SHALL be every set hand item and every worn item but item 8 of its six loadouts, each once, in order, and SHALL go to the arsenal with flags 0x400. A specialist's own items SHALL go with flags 0x200.

#### Scenario: Two loadouts
- **WHEN** a kit is built from loadouts with hands 5 and none and items 8, 9, 10, then hands 6 and 5 and items 10, 11, and the combatant Ash with hand 5 and items 8, 9 is hired
- **THEN** the kit is 5, 9, 10, 6, 11; Ash's kit is 5, 9 and the weapons shelf holds 5 and 9 with 0x400

### Requirement: Recruitment
A recruitment SHALL cost 12 copper, refused with "Insufficient funds" below that, and SHALL NOT check the room. The recruit's six loadouts SHALL copy its outfit and its kit SHALL go to the arsenal with 0x400; one more hire SHALL be counted and the joining logged.

#### Scenario: Nine fighters
- **WHEN** 11 copper recruits, then 108 copper recruits eight times with an outfit of hands 3 and 4 and items 8 and 7
- **THEN** 11 is refused, the eight recruits make nine fighters and leave 12, and a recruit's kit is 3, 4, 7

### Requirement: Leaving
An employee who leaves SHALL release a fighter's kit from flags 0x400, or a specialist's items, hands and worn items without flag 0x100 from 0x200. A merchant SHALL empty his stock and his figure SHALL leave the hub, as SHALL a physician's figure. The id SHALL leave the tournament entries and the log's saves, and later ids SHALL move down by one. A dismissal SHALL log the name and count one more fired.

#### Scenario: The second of three leaves
- **WHEN** of three combatants the second leaves while an entry holds ids 2 and 3 and the log saves 2, 3 and 1, and a merchant is dismissed
- **THEN** the entry holds id 2 alone, the saves are 2 and 1, the first two kit entries are released; the merchant empties his stock, one is fired and the dismissal is logged

### Requirement: Order list
The manage table's order list SHALL grow by the employee count on a hire or a recruitment only when it already holds an entry. A leaving SHALL make it one shorter: each entry but the last equal to the leaver's number SHALL close up, the scan moving past the entry that took its place, and later numbers SHALL stay.

#### Scenario: Removals
- **WHEN** 2 leaves 1, 2, 3, 4; 9 leaves 1, 2, 3; 1 leaves 5; 2 leaves 2, 2, 3
- **THEN** the lists are 1, 3, 4; 1, 2; empty; and 2, 3

### Requirement: Report title
The report's title SHALL be "Day" and the day from 1 before day 7, else "Week", the week, "Day" and the day of the week from 1.

#### Scenario: Titles
- **WHEN** days 0, 6, 7 and 20 are titled
- **THEN** they read Day 1, Day 7, Week 1 Day 1 and Week 2 Day 7

### Requirement: Report log lines
A won, lost or forfeited match SHALL read "You have won", "You have lost" or "You forfeited", the match in lower case after "a " or, before a vowel, "an ", and "match."; the Beast SHALL read "beast". A death, save, dismissal, joining and leaving SHALL read the name and " has died.", " was saved from lethal injury.", " has been dismissed.", " has joined the company." and " has left the company.".

#### Scenario: Lines
- **WHEN** the log holds a won Elimination, Ash joining, the manager Boss saved and a leaving
- **THEN** the lines read "You have won an elimination match.", "Ash has joined the company.", "Boss was saved from lethal injury." and the leaver's name with " has left the company."

### Requirement: Report fighters and hire list
After the log, newest first, each fighter and then the manager SHALL be listed when ready to train (no skill below level 5 with its next technique set, and a level sum under 15) as " has trained a new technique.", with " has reached " and the rank of half the sum and " rank." when the sum is even and under 11. The hire list's merchants, trainers and physicians SHALL follow, then "The tournament takes place today.", "begins tomorrow." or "begins in N days.".

#### Scenario: Day 3
- **WHEN** on day 3 the log holds a lost duel and a forfeited doubles, Ash has levels 2 and 2, the manager level 1, and the list holds a merchant and two trainers
- **THEN** the report reads the forfeit, the loss, Ash trained and Novice rank, the manager trained, "A merchant is available for hire.", "2 trainers are available for hire." and "The tournament begins in 4 days."

### Requirement: Fixed match places
A one-on-one match (Duel, Challenger, Elimination, Reserve, Pugilism) SHALL place its opponent at the spawn spot plus (0, 5, −200) cm facing +z and the company's fighter at the spawn spot plus (0, 0, 200) facing −z. Pugilism's opponent SHALL have no gear tier.

#### Scenario: A duel on arenao3's spot
- **WHEN** a duel of level 3 and tier 2 is laid out at spawn (900, 0, 100), pugilism likewise, and template 99
- **THEN** the opponent of level 3 and tier 2 on team 2 with flags 0x12 stands at (900, 5, −100) facing +z, the company at (900, 0, 300) facing −z; pugilism's opponent has tier −1; template 99 lays out nobody

### Requirement: Ring places
A ring place SHALL turn a fraction of a turn (the single-precision turn times the fraction) into the direction (sin, 0, cos) of the engine's sine and cosine, stand at the spawn spot plus the direction times −350, and face along the direction. A fray's place SHALL use the index times the turn times a sixth.

#### Scenario: Half a turn and none
- **WHEN** fractions 0.5, 0, −0.05 and 0.05 are placed
- **THEN** 0.5 stands 350 behind the spot facing −z, 0 stands 350 ahead facing +z, and ∓0.05 stand mirrored about 108.16 to either side

### Requirement: Team match layouts
Doubles SHALL place opponents at ∓0.05 and the company at 0.55 and 0.45. Valiance SHALL place two opponents a level and a tier lower at ∓1/15 and the company at 0.5. Valour SHALL place three two lower at −1/12, 0 and 1/12; Captain the same with the middle one at the match's level and the company at 0.5, 7/12 and 5/12. Skirmish SHALL place three at the match's level and the company at 7/12, 0.5 and 5/12, and the Beast its beast at the fixed opponent place with the company the same.

#### Scenario: Lowered opponents
- **WHEN** Valiance, Valour, Captain, Skirmish, the Beast and Doubles are laid out at level 4 and tier 3
- **THEN** Valiance's opponents are level 3 tier 2, Valour's level 2 tier 1, Captain's middle opponent level 4 and its second entrant at 7/12, Skirmish's first entrant at 7/12 and second at 0.5, the beast at the opponent's place, and Doubles' first entrant at 0.55

### Requirement: Fray layout
A fray or a brawl SHALL place six fighters a sixth of a turn apart: the company's fighter fourth, each other an opponent in a free for all (flags 0x10) on team 1 shifted by its index plus 3.

#### Scenario: Six teams
- **WHEN** a fray and a brawl are laid out at the origin
- **THEN** both are the same six places, the fourth is the company's, the teams are 8, 16, 32, 128 and 256, and the first stands at (0, 0, −350)

### Requirement: Next round
A multi-round match SHALL bring a fresh opponent of its level and tier while no round is lost or its fighter still stands; otherwise the opponent SHALL stay and the entrant at the place of the losses SHALL come in.

#### Scenario: Fresh or reserve
- **WHEN** the next round follows no loss with the fighter dead, one loss with it standing, and two losses with it dead
- **THEN** the first two bring a fresh opponent and the third brings entrant 2

### Requirement: Running match layout
The running match SHALL take its rounds from its template (Elimination four extra rounds, Reserve two and three lives). In play mode `--arena-match <id>` SHALL put the player on the company's first place and a duellist on each opponent place; other entrants and the beast SHALL NOT be spawned.

#### Scenario: Template rounds
- **WHEN** templates 7, 5 and 1 begin a match
- **THEN** 7 has four extra rounds, 5 three lives, and 1 the single round

### Requirement: Tournament teams
A tournament event SHALL group its entry slots in runs of the team size: from the first slot holding an id, the team SHALL be the run it falls in, its id the first held id and its members the slots with a positive id; the next team SHALL start after the run, until the event's team count is formed or the slots end.

#### Scenario: Two teams of two slots
- **WHEN** slots 1, 1, 0, 5, 0, 0, 7, 7 form two teams of two slots, slots 0, 0 form any, and a team size of 0 is asked
- **THEN** team 1 holds slots 0 and 1, team 5 holds slot 1, team 7 is cut off, and the last two form none

### Requirement: Tournament ring
The ring's radius SHALL be the teams times the team size times 50 plus 100, at most 350. Team `g` of `G` with `n` members SHALL spread them over `2 (n − 1)/(n + 1)` (in double, stored as single) from `g/G × 6` less half the spread, one every spread over `n − 1`. A member at `a` SHALL face the direction of the turn times `(3 − a) × 0.1666666` in double and stand the radius against it.

#### Scenario: A lone member and a full ring
- **WHEN** the two teams above are placed, and two teams of three slots
- **THEN** the radius is 300, the lone member stands at (0, 0, −300) facing +z, the first member sits at a = −1/3, and the larger event stops at a radius of 350

### Requirement: Tournament opponents
A team id under 2 SHALL bring a generic fighter of level 5 at the event's tier, any other the company's fighter; a team SHALL be hostile while at most two teams fight, else every team SHALL fight for itself.

#### Scenario: Generic and company
- **WHEN** team 1 of two and team 6 of three are asked at tier 3
- **THEN** team 1 brings a level-5 fighter hostile on team 1, and team 6 a company fighter in a free for all

### Requirement: Company chunk
The company chunk SHALL hold the name, a second string, the 28-byte emblem record, three words, one unused word and sixteen record words; a string SHALL be a word count and its bytes. Reading SHALL skip the unused word, writing SHALL store 0 there, and a payload that ends inside a field SHALL be an error.

#### Scenario: Ash Company
- **WHEN** a company named Ash Company with emblem bytes 52 and 81, three matches won and 343 copper spent is written, framed, read, read with the unused word set, cut one byte short, and a string count of 0xffffffff is read
- **THEN** the payload is 127 bytes starting with the count 11, the frame starts with the id bytes 00 00 dc a0, both reads give the company back, and the short and oversized payloads are errors

### Requirement: Day chunk
The day chunk SHALL hold today's matches, copper spent and earned, a count of special matches with each one's template id, rank, flags and an unused word, a word beside the log, and the log's count and twenty-byte records. A negative count SHALL read no entries.

#### Scenario: Two specials and a record
- **WHEN** a day with two specials and one log record is written and read, cut to 20 bytes, and a count of −1 is read
- **THEN** the payload is 76 bytes and reads back, the cut payload is an error, and −1 reads no specials

### Requirement: Tournament chunk
The tournament chunk SHALL hold the week's prize word, 0x120 bytes of event rows, sixteen rival company words, six events of four words, four week words and eight prize history words. A field whose first company is 0 SHALL ask for a new field.

#### Scenario: A week in progress
- **WHEN** a tournament with prize 4, company 17 first, an event row byte, the last event Elimination and a history id 1172 is written and read, and cut to 100 bytes
- **THEN** the payload is 500 bytes and reads back without asking for a field, an empty field asks for one, and the cut payload is an error

### Requirement: Hub music
Entering the hub SHALL step the roll and play Msc_Hub01 when the high half of the word times 3 is under 2, else Msc_Hub02 (the DnB tracks with the alternative music option), at 0.1. It SHALL step again and start the track at none, 41, 126.8 or 152 seconds by the high half of the word times 4. In play mode the hub SHALL draw its music from the counter roll.

#### Scenario: Two hub entries
- **WHEN** the hub is entered from rolls 0, 0xf0000000 (with the option), 4, 2 and 1
- **THEN** roll 0 plays Msc_Hub01 from its start and ends at 0x08088406; 0xf0000000 plays Msc_Hub02_DnB from 41 seconds and ends at 0x78088406; 4, 2 and 1 start at 41, 126.8 and 152 seconds

### Requirement: Hub click
A hub click SHALL open nothing while the hub is busy, outside the arena mode or while a confirmation is open. An object named ArenaShop (its sixteen bytes compared with a to z matching capitals) SHALL open the shop; an ArenaMach object SHALL open the match board when its side value is above 0 and the manage window otherwise; an employed merchant who is not helpless SHALL open his stock.

#### Scenario: Clicks
- **WHEN** the shop, a lower-case shop name, the machine at sides 1, 0 and NaN, a name without the first byte, a merchant standing and down, and a stranger are clicked, and the shop is clicked while busy, in mode 4 and while confirming
- **THEN** both shop names open the shop, side 1 opens the board, 0 and NaN the manage window, the bare name nothing, the standing merchant his stock, the downed merchant and the stranger nothing, and the gated clicks nothing

### Requirement: Hub fall
While the hub's session runs, a manager whose condition is dead SHALL end the session; health under 0.01 compared in double SHALL first wipe the career's saves.

#### Scenario: Falls in the hub
- **WHEN** a dead manager has health 0, 0.01, 0.02 and NaN, a collapsing manager has 0, and the session is not running
- **THEN** 0 and 0.01 wipe, 0.02 and NaN only end the session, and the others do nothing

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

### Requirement: Next tournament event
The board's Begin on the tournament page SHALL start the first of the week's six events with no winner and an entrant, and nothing when none is left. While the manager warning is armed and the manager is a participant, Begin SHALL disarm it and show the warning once.

#### Scenario: Events 2 and 4 entered
- **WHEN** no event is entered, then events 2 and 4 are, then event 2 is won, then event 4, and the warning is asked without and twice with the manager
- **THEN** nothing starts, then event 2, then event 4, then nothing; the warning shows once

### Requirement: Day gate
The manage window SHALL let the day end on a tournament day with entries only when no entered event is still pending, and on any other day only once a match was fought.

#### Scenario: A tournament day
- **WHEN** day 3 has no match and one match, day 7 has two pending events and three matches, day 7 has both events won, and day 8 has no match
- **THEN** day 3 ends only after its match, day 7 waits for its events whatever the matches and then ends, and day 8 does not end

### Requirement: Health bar
The company tables SHALL fill a fighter's health bar with its health word times the double 1/65535, stored as single.

#### Scenario: Full and half
- **WHEN** the words 65535 and 32768 are drawn
- **THEN** the bars are bits 0x3f800000 and 0x3f000080

### Requirement: Manage window rows
Each roster row of the Manage Company window SHALL show its record's name, a rank label and a health bar. The label SHALL be "Merchant", "Trainer" or "Physician" for those professions, else the rank name with the mark "+" while the fighter is ready to train. Recruit SHALL be disabled at eight fighters (the manager counted) and Hire at ten employees.

#### Scenario: Labels and gates
- **WHEN** a merchant, a ready Novice and a Master who is not ready are listed, with seven and then eight fighters, and nine and then ten employees
- **THEN** the labels are Merchant, Novice with "+", and Master; Recruit is open at seven and closed at eight, Hire open at nine and closed at ten

### Requirement: Roster name ink
A roster name SHALL be drawn in red 255, green 210, blue 100 with a fifth of the skin's alpha times three for the manager; 220, 180, 160 with a third of the alpha times two for a merchant, trainer or physician; 160, 180, 240 with the same alpha for a hired combatant; and in the skin's own text colour for a recruit.

#### Scenario: Alpha 255 and 100
- **WHEN** the alpha is 255 for the manager and a hired combatant, and 100 for a specialist
- **THEN** the alphas are 153, 170 and 66

### Requirement: Report line click
A click on a report line SHALL open the match board's tournament tab for the tournament line; the hire list on the first candidate of the line's profession (or the first candidate when none) for a merchant, physician or trainer line; and the roster on its first row, remembering the fighter id in the line code's top byte, for a training or rank line. Log lines SHALL open nothing.

#### Scenario: A hire list of combatant, trainer, merchant
- **WHEN** the merchant line, the trainer line, the physician line and the training line 0x03140000 are clicked
- **THEN** the hire list opens on candidates 2, 1 and 0, and the roster opens remembering fighter 3

### Requirement: Arsenal form selection
The arsenal form SHALL clamp its selection to its rows. Except on an event's lineup page, Next SHALL be disabled on the last row, and on the first row Previous SHALL be disabled and Dismiss disabled too; off the first row Previous SHALL be enabled, and Dismiss enabled on the roster and specialist pages.

#### Scenario: Three employees
- **WHEN** row 0 is shown, then row 7 is asked for
- **THEN** row 0 disables Previous and Dismiss; row 7 shows row 3 with Next disabled and Previous and Dismiss enabled

### Requirement: Arsenal form steps
A step to another row SHALL load that row's record (a candidate on the hire page, the manager on row 0, else the employee one place up) and SHALL be shown when committed; on the lineup page a step SHALL only be remembered. A step to the row shown SHALL do nothing.

#### Scenario: Next from row 1
- **WHEN** Next is pressed from row 1 of three employees and the step committed
- **THEN** employee 1 is loaded and the commit shows row 2

### Requirement: Arsenal form dismissal
Confirming the dismiss dialog ("Are you sure you want to dismiss " with the name and "?") SHALL load the row above and, when committed, SHALL dismiss the employee of the row shown, rebuild the rows and show the row above.

#### Scenario: Dismiss from row 2
- **WHEN** row 2 is shown and the dismissal confirmed and committed
- **THEN** employee 0 is loaded, employee 1 is dismissed and row 1 is shown

### Requirement: Arsenal form pages
A specialist's page SHALL filter the arsenal by flag 0x200, hide the rank controls and show no loadout tab. The roster, hire and lineup pages SHALL filter by 0x400 and return to the kept tab, the roster and hire pages showing the rank controls. A record SHALL open the specialist's page for a merchant, trainer or physician and the roster page otherwise; the lineup page SHALL keep its page and the hire page SHALL not change it. The hire page SHALL open on tab 5.

#### Scenario: A trainer, then a fighter
- **WHEN** the kept tab is 3 and a trainer's record, then a fighter's, are opened
- **THEN** the trainer's page filters by 0x200 with tab −1, and the fighter's by 0x400 with tab 3

### Requirement: Rank slider and points
The rank slider SHALL name the rank of its value times five, rounded to even in 64 bits and cut to 32 (a NaN names rank 0). The points line SHALL read "Points: " followed by the loadout cost plus one halved, " / " and the tab's loadout limit; below tab 0 the line SHALL not change and the stored cost and limit SHALL be 0 and 100.

#### Scenario: Values and tabs
- **WHEN** the slider is at 0.3, 0.5 and 0.7, and a cost of 7 is shown on tab 0
- **THEN** the ranks are 2, 2 and 4, and the line reads "Points: 4 / 5"

### Requirement: Record texts
A record's title SHALL be its name followed by its profession for a merchant, trainer or physician, or its rank name, in brackets. A candidate SHALL show "Profession: " with its profession and the profession's description. The skills panel SHALL be shown up to a trainer's role.

#### Scenario: Bors
- **WHEN** Bors is a trainer, then an Adept combatant
- **THEN** the titles are "Bors (Trainer)" and "Bors (Adept)"

### Requirement: Company status
Company Status SHALL show the company's name, renown and reputation "Unknown", morale "Good", the staff with the manager, and as masters the manager and the fighters whose rank is above 4, or "None".

#### Scenario: Two masters
- **WHEN** the manager is rank 5 and the staff are a combatant of rank 5, a trainer of rank 5 and a combatant of rank 4
- **THEN** the staff is 4 and the masters 2

### Requirement: Roster drag
A roster row dragged onto another SHALL take that row's place; dropped on no row it SHALL go last, and it SHALL never take the manager's row. The rows between SHALL close up, and every id that moved SHALL be remapped in the log's "saved" records, the tournament entries and the order list's entries up to the staff's count.

#### Scenario: D onto row 2
- **WHEN** of A, B, C, D the row of D is dropped on row 2, with the order list 4, 1, 2, 7, the log saving 4 and 2, and an entry of ids 4 and 3
- **THEN** the staff is A, D, B, C, the order 2, 1, 3, 7, the saves 2 and 3, and the entry 2 and 4

### Requirement: Arsenal sale
An item that leaves the company SHALL leave the arsenal. A unique item (either top id bit set) SHALL leave every box and every holder's record and list. Another item SHALL lose one copy from its shelf: scanning boxes from the last and each from its end, the first copy flagged 0x200, else the first copy found.

#### Scenario: A flagged copy in an earlier box
- **WHEN** box 1 holds the sword flagged 0x200 and flagged 0x400, and box 3 holds it flagged 0x400
- **THEN** box 1 keeps only the copy flagged 0x400 and box 3 keeps its copy

### Requirement: Holders after a sale
After a copy is sold each holder in turn, the manager first, SHALL keep as many copies as were left; the copies past that, from the end of his list, SHALL close up with the list keeping its length, and the allowance SHALL fall by the copies removed. A holder left with fewer than two copies SHALL have the item taken from his loadouts past what he keeps.

#### Scenario: One copy, two holders
- **WHEN** the only copy is sold while the first holder lists the item, 5 and the item, and the second lists the item once
- **THEN** the shelf is empty, the first list is 5, 5 and the item, the second list keeps its entry and both holders' loadouts lose the item

### Requirement: Day's lessons
At the end of the day the first trainer on the staff SHALL teach the manager, then every fighter in staff order. Without a trainer there SHALL be no lessons.

#### Scenario: A trainer, a merchant and a recruit
- **WHEN** the staff are a combatant, a trainer, a merchant and a recruit
- **THEN** the trainer teaches the manager, the combatant and the recruit

### Requirement: Tournament event start
Starting a tournament event SHALL give each event before the first one the company entered, that has no winner and no entrant, the company `2 + high(roll × 6)` with the roll stepped by `× 0x08088405 + 1`. The event SHALL run its rounds less one as extra rounds from round 0, its bracket holding `sides << (rounds − 1)` teams of its team size, team `t` holding id `t + 1`, each slot of strength 1 with its member index.

#### Scenario: Roll 1
- **WHEN** the events are open, won by 5, open, entered, open, open, and the roll is 1
- **THEN** the winners are 2, 5, 7 and the rest stay open, and the roll ends at 0xdc6dac1f; an elimination's bracket holds ids 1 to 16 and a conquest's 16 slots of 8 teams of two

### Requirement: Tourney forfeit
Forfeiting the entire tourney ("Do you wish to forfeit the entire tourney?") SHALL empty every event that has no winner.

#### Scenario: Six events
- **WHEN** events with counts 2, 1, 1, 0, 2, 1 and winners 0, 4, 0, 0, 1, 0 are forfeited
- **THEN** the counts are 0, 1, 0, 0, 2, 0

### Requirement: Hub arrival
Arriving at the hub SHALL, when a physician is on the staff, remove the weapon-kind object lying within 10 cm in x and z of (−2100, 0, 900) cm, as the physician's hire does, and SHALL build the tournament board when it holds no prize.

#### Scenario: A physician and an empty board
- **WHEN** the staff holds a physician and the board's prize is 0, then a merchant and a prize of 7
- **THEN** the object is removed and the board built, then neither

### Requirement: Rematch lineup
The automatic rematch SHALL enter, from the second employee on, those whose class is under 4, until the match's team size is reached or the staff ends.

#### Scenario: Two wanted
- **WHEN** the classes are 2, 8, 1, 2, 2 and two are wanted
- **THEN** employees 2 and 3 enter

### Requirement: Arsenal form boxes
A pressed shelf or box button SHALL store the form's view in the box it showed, then load the same box of the chosen shelf, or the chosen box of the shelf, sorting it first when it was marked and clearing the mark. HIRE SHALL hire the record shown only on the hire page.

#### Scenario: Shelf 3, then box 4
- **WHEN** box 0 of shelf 3 is marked, shelf 3 is pressed from shelf 0 box 0, then box 4
- **THEN** shelf 0 box 0 is stored and shelf 3 box 0 loaded sorted, then shelf 3 box 0 is stored and box 4 loaded unsorted

### Requirement: Event lineup
The board SHALL open an event's lineup only for an event with an entrant, on the first entrant's row, at the loadout tab of the event's place, with the entrant's two weapon words. Closing it SHALL write the record's two hand words back to the first entrant with the second word's item id cleared.

#### Scenario: Event 2
- **WHEN** event 2's first entrant is 3 with weapon words 0x11 and 0x22, and the lineup closes with hands 0x500000007 and 0x900000008
- **THEN** it opens on row 3 at tab 2 with 0x11 and 0x22, and the entrant keeps 0x500000007 and 0x900000000

### Requirement: Tournament settled on the return
Back at the hub after a match on a day that is a multiple of 7, each event before the first one the company entered that has no winner and no entrant SHALL take the company `2 + high(roll × 5)`, the roll stepped by `× 0x08088405 + 1`, after the day's match count.

#### Scenario: Roll 1 on the return
- **WHEN** the events are open, won by 5, open, entered, open, open, and the roll is 1
- **THEN** the winners are 2, 5, 6 and the rest stay open, and the roll ends at 0xdc6dac1f; six open events with roll 1 take 2, 6, 3, 3, 5, 3 where the start's pass gives 2, 7, 3, 3, 6, 3

### Requirement: Running hub session
When the arena hub's level is loaded in arena mode, or by the play mode that loads it, the game SHALL run the hub entry once, play its track and install its hooks, and each frame a dead manager SHALL end the session. Outside those modes the hub's systems SHALL do nothing.

#### Scenario: Entry and fall
- **WHEN** the hub level loads in arena mode with roll 0 and the alternative music, a second frame runs, and the manager is dead with health 0
- **THEN** the entry plays msc_hub01_dnb at 0.1 once and the fall ends the session with a wipe

#### Scenario: Other modes
- **WHEN** the plugin runs three frames in story mode with the hub level present
- **THEN** the hub is not entered and nothing panics

### Requirement: Running hub clicks
A click that is not a drag SHALL reach the hub's click hook with the hub's busy flag, the mode and whether a hub window is open, and SHALL open the answered window. The hook SHALL compare the stored sixteen name bytes, including the library's leading 0xAA, of the scene node or prop the click picked.

#### Scenario: Clicks in the hub
- **WHEN** an ArenaShop node, an ArenaMach node at side 1 and at side -1, a standing and a downed merchant are clicked, and the shop is clicked while busy
- **THEN** the shop opens the trade window, side 1 the board, side -1 Manage Company, the standing merchant his stock, and the downed merchant and the busy click nothing

#### Scenario: The play view's hub
- **WHEN** the hub is loaded and the flow is in the practice stand-in or the menu, then in practice without the hub, then in story mode with the hub
- **THEN** the hook's mode is the arena's for the first two and practice and story for the others

#### Scenario: The installed hub's scenery
- **WHEN** arenahub is loaded from the installed game and a ray is aimed at a triangle of the ArenaShop node and of an ArenaMach node
- **THEN** each ray's pick names that scene node

### Requirement: Hub busy flag
Opening the match board or Manage Company SHALL make the hub busy and closing either SHALL clear it; the trade window SHALL leave the flag alone.

#### Scenario: Board and manage
- **WHEN** the board is opened, then Manage Company is opened and closed with Escape
- **THEN** the hub is busy while each is open and not busy after the close

### Requirement: Arena round transition
A round's end SHALL set the match state (8 next fight, 5 lost, 4 won) and open transition screen 3 while the state is the next fight, else screen 2, and SHALL award experience either way.

#### Scenario: Elimination rounds
- **WHEN** an Elimination match wins four rounds, the round counter rising after each, and then wins a fifth
- **THEN** the first four open screen 3 with state 8 and the fifth opens screen 2 with state 4

### Requirement: Arena end screen Continue
The end screen's Continue SHALL send a dead manager to the main menu, SHALL start another round while the state is the next fight and the round less the losses is below the extra rounds (a tournament raising the bracket's next fight), and SHALL otherwise settle the match and return to the hub, saving the manager at 300, a half turn and flag 0x100.

#### Scenario: Continue branches
- **WHEN** an Elimination match at round 3 continues in the next-fight state outside and inside a tournament, and at round 4
- **THEN** round 3 starts round 4 (the tournament with the bracket), and round 4 returns to the hub

### Requirement: Arena next round bodies
The next round SHALL keep the company's fighter and generate a fresh opponent at the match's level and gear tier, unless the last round was lost and the fighter is dead; then the entrant at the place of the losses SHALL come in against the opponent who won, and a fresh opponent SHALL be generated only when no opponent body is left.

#### Scenario: Reserve entrant
- **WHEN** the second loss leaves the fighter dead and the opponent standing
- **THEN** reserve entrant 2 comes in, the opponent is kept and no opponent is generated

### Requirement: Arena dead-body pass
Before the end screen and the return, every company body whose health widened to double is below 0.01 and whose fall is not yet resolved SHALL be resolved once; NaN health SHALL never be resolved.

#### Scenario: Single 0.01
- **WHEN** a company body has health 0.01 in single precision and a physician is employed
- **THEN** its fall is resolved and logged as saved

### Requirement: Arena match entry fee
The match start SHALL subtract the event's entry fee from the purse, stopping at 0, and add the whole fee to the spending; a short purse SHALL not refuse the match.

#### Scenario: Short purse
- **WHEN** a Duel with a 3 copper fee starts with 2 copper
- **THEN** the match launches, the purse is 0 and the spending rose by 3

### Requirement: Tournament prize history
The weekly prize draw SHALL refuse a prize found in any of the eight history slots, then rotate the history and store the prize first.

#### Scenario: Eighth slot
- **WHEN** seed 13 first draws prize 8 with the history 1 to 8
- **THEN** that draw is refused and a prize outside the history is stored first

### Requirement: Arena roster health gauge
The roster's health gauge SHALL keep the cell's left, take the top less six units plus half of five units (rounded down) plus one, the width less one unit and a thickness of two units, filled by the record's word times the double 1/65535 stored as single.

#### Scenario: Base unit
- **WHEN** a cell at (100, 50) of width 60 draws a full word at unit 4
- **THEN** the gauge is at (100, 37), 56 long and 8 thick, filled to 1

### Requirement: Scene body generator
The game SHALL build an arena fighter, a company entrant or a hub NPC from a spawn plan on the placed characters' spawn path, and SHALL remove a body named by its root or its body model together with its root, subtree and physics capsule. Frames without a command SHALL not read the game's tables.

#### Scenario: Spawn and despawn
- **WHEN** a generic opponent plan at level 2, tier 1 is sent with the game's packs and three frames run, and then a despawn names the built body model
- **THEN** one body is built with its physical body, the root and model are gone after the despawn, and the physics world holds one body fewer

### Requirement: Live arena rounds
A match's first round SHALL stand every layout slot except the player's own entry; the end screen's Continue SHALL keep a living company fighter and the opponent that won, clear the dead, and bring the reserve or a fresh opponent; the match's end SHALL settle the result into the career and ask for the hub with the saved return placement.

#### Scenario: Reserve against the winner
- **WHEN** an Elimination match at round 2 with one loss continues, with a standing opponent and a dead company fighter
- **THEN** round 3 starts, the dead fighter is despawned, the opponent stays and one reserve body is asked for

### Requirement: Hub bodies
When the hub is entered, in arena mode or under the play view's practice stand-in, the game SHALL place the manager and ask once for the shopkeeper at -320, 0, -110 cm facing 15 degrees and one body per employed merchant or physician at their planned spots; story mode SHALL leave the hub alone.

#### Scenario: Practice stand-in
- **WHEN** the hub level and the player are present with the flow in practice mode, and again in story mode
- **THEN** in practice mode the manager is placed and the hub bodies are asked for; in story mode neither happens

### Requirement: Drawn arena end screen
When a round ends with a transition screen, the game SHALL draw the arena's single-row end menu through the GUI path, `NEXT FIGHT` for screen 3 and `EXIT;` for screen 2 at scale 0.9, and SHALL run the Continue only from that menu's row (Enter or a pointer release over it); the arena pause page's FORFEIT SHALL settle a fighting match as forfeited and return to the hub, and SHALL be ignored when the manager is down.

#### Scenario: Space no longer continues
- **WHEN** a won match's end screen is open and Space is pressed, then Enter
- **THEN** Space does nothing and Enter settles the match and asks for the hub

### Requirement: Tournament next fight
A tournament's Continue after a won round SHALL resolve the bracket with the player's team kept and every other group's winner drawn, and SHALL stand the first surviving teams in slot order, as many as the event has sides, with the role flag hostile below three teams and free-for-all otherwise.

#### Scenario: A duel event stays a duel
- **WHEN** the four-round duel tournament continues after its first won round, with eight teams surviving
- **THEN** exactly one opponent is raised, generated from its record, with the hostile role, and the player is placed on the ring

### Requirement: Generated arena records
A generic or hire-list arena opponent SHALL be built from its own generated character record (random NPC pass, then the level or profile outfit pass, all draws from one sequence) with personality −100, 100, 0 and, for a hire-list opponent, the event's drawn weapon in the main hand, through the placed characters' merged-mesh spawn path.

#### Scenario: Hire personality
- **WHEN** a hire-list fighter is made from a tier 3, company 2 plan without a weapon row
- **THEN** its profile flags are 0x0b with company bytes 66, 140, 1 and its personality is −100, 100, 0, 0

### Requirement: Beast record
The Beast SHALL be spawned from its own record: the Ogre race, a combat skill of 100, the tier's main-hand item (0x44 for tiers 1 and 2, 0x1f6, 0x207 or 0x21e for tiers 3 to 5) and, for tiers 3 to 5, its four worn pieces in order, a voice pitch of 0.9 + 0.17 times the smoothed first draw, voice 0, 50000 experience spent on the profession grid, and personality −100, 100, 0.

#### Scenario: A tier 4 Beast
- **WHEN** a tier 4 Beast record is written
- **THEN** its race byte is 3, its main hand holds 0x207, it wears 0x210, 0x20e, 0x20f and 0x213, and its skill rows hold 50000 experience in all

### Requirement: Clock reseed of generated bodies
Before each generated opponent or Beast record the shared random sequence SHALL restart from the performance counter's low 32 bits with their bytes reversed, and later draws SHALL continue from where the generation left it.

#### Scenario: A row-armed hire side
- **WHEN** a hire side armed from an event row is generated
- **THEN** its weapon is the row slot picked by the first step after the random NPC pass, before the outfit pass

### Requirement: Rival company per tournament team
A tournament side of team 2 or above SHALL wear the hire profile's forced outfit of the company the week's field names at slot team − 2, with the event's tier as the profile's level byte, in the first round, the launch and the bracket's later fights; the field SHALL be drawn when its first company is 0 and saved with the tournament.

#### Scenario: Team 6 of a drawn field
- **WHEN** the field begins 15, 17, 4, 1, 13 and team 6 is generated at tier 2 for an event that arms its fighters
- **THEN** its profile flags are 0x0f, its level byte is 2 and its company bytes are 97, 20, 0

### Requirement: Hub machine side
The side an ArenaMach click compares SHALL be the x of the clicked node's world bounding centre, and only a side above zero SHALL open the board; zero, negative zero, a negative side and NaN SHALL open Manage Company.

#### Scenario: Centre, not origin
- **WHEN** ArenaMach nodes with their origin at x 5 and bounding centres at x −10 and at x 10 are clicked
- **THEN** the first opens Manage Company and the second the board

### Requirement: Hub first stock
A hub click on the shop or on the merchant whose shelf is empty SHALL build that shelf's first stock from the object database: the shop with the default kinds and any rank, the merchant with its stock type and specialisation. A shelf that has items SHALL be left as it is.

#### Scenario: One listed apparel record
- **WHEN** the live stock world holds one listed apparel record of rarity 0 and the shop is clicked twice
- **THEN** the first click shelves 18 to 20 copies of that record and the second leaves the shelf unchanged

### Requirement: Merchant chunk words
The save's merchant chunk SHALL load and write the merchant's stock type and specialisation as the career's, 0 and −1 in a new career.

#### Scenario: Round trip
- **WHEN** a career with a weapons merchant (stock type 0xc000) of rank 3 is written and applied to a new career
- **THEN** the chunk holds 0xc000 and 3 and the new career's merchant has them

### Requirement: Weighted draw conversion
Both weighted team draws SHALL convert each weight with the 64-bit conversion (ties to even) and keep its low 32 bits, so a NaN or infinite weight SHALL add 0. A scene pick that no running weight reaches SHALL keep the first team.

#### Scenario: Out-of-range weights
- **WHEN** NaN, infinity, 2^31 and −3e9 are converted
- **THEN** they give 0, 0, the 32-bit minimum and 1294967296

### Requirement: Return health write-back
The return to the hub SHALL write each company body's health word into its record: participant 0 into the manager, participant n into roster member n − 1, and nothing for a participant the roster no longer has. Company bodies SHALL carry the participant id the numbering pass gives.

#### Scenario: Manager and roster
- **WHEN** the manager returns at half health, roster member 2 at none, and participant 9 at a tenth
- **THEN** the manager's word is 32768, member 2's is 0, member 1 keeps 65535, and two words are written

### Requirement: Rank-up kit copy
A lesson that raises a fighter's level sum to an even sum from 2 through 10 SHALL copy kit sum/2 − 1 over kit sum/2.

#### Scenario: Crossing a rank
- **WHEN** a lesson takes a fighter from 9600 to 10000 points in one skill with kit 0 holding word 9
- **THEN** kit 1 holds word 9

### Requirement: Rival week company
A rival champion of a week SHALL record the company at its own team's field slot plus 2, not the first tallied team's.

#### Scenario: Champion tallied second
- **WHEN** the rows are [3,2,2,2,4,5] with field company 15 at slot 0
- **THEN** no player champion is recorded and the week holds company 15+2

### Requirement: Combat score
Every fighter SHALL keep a combat score starting at 0. A wound SHALL add 3 to the striker's score; a clash or block SHALL add 1 to the struck fighter's score only while it is parrying.

#### Scenario: Wounds and guards
- **WHEN** a fighter lands two wounds, and another takes a clash and a block while parrying and a block while not
- **THEN** the first scores 6 and the second 2

### Requirement: Live round experience
At every round's end the arena SHALL compute the experience pool from the scene's bodies (the match's slot as the tier) and grant each company body with a record its share as a skill lump, clearing that body's score. A company fighter who died that tick SHALL get nothing.

#### Scenario: Manager credited
- **WHEN** a slot-3 Duel ends with the opponent missing half its health, the manager having scored 3 and a roster fighter having died
- **THEN** the manager's trained skill gains 875 and the roster fighter's nothing

### Requirement: Bracket end
A tournament's end or forfeit SHALL draw the scene's winning team, clear the brackets again with no forced team while a pass resolves more than one group, and give the first event without a winner the first surviving team.

#### Scenario: Four teams
- **WHEN** teams 5, 9, 3 and 7 of strengths 4, 10, 4 and 2 end with team 5 drawn and roll 1
- **THEN** only team 3 survives, the roll ends at 0x33dc589c and the first open event takes team 3

### Requirement: Arena reset
Loading an arena slot SHALL first clear the shop's shelf, the merchant's stock (kinds 0, rank −1), the company's records and day tallies, the day, today's matches, the log, the roster, the purse, the week verdicts, the stash, the arsenal and the staff, keeping the company's name and the tournament entries.

#### Scenario: Loaded after play
- **WHEN** a career with a hire, stock, day 9 and an entry is reset
- **THEN** the roster and stock are empty, the day and purse are 0, the merchant rank is −1, and the name and entry stay

### Requirement: Auto rematch
With the auto-match switch on, 40 timer ticks of 100 ms after the return to the hub the last board match SHALL start again with the recruits and combatants after the manager, as many as a side holds.

#### Scenario: Lineup
- **WHEN** a duel pair rematches with a roster of combatant, merchant, combatant
- **THEN** pool slots 1 and 3 stand, 4000 ms after the return

### Requirement: Hub windows from their control tables
Manage Company, Company Records, Company Status, the loadout form and the dismiss and yes/no dialogs SHALL be built from their constructors' record arrays (18, 27, 12, 28, 3 and 3 records), laid out at unit 4, rescaled to the live unit and placed by the form's placement bits, and a record with a callback SHALL run the arena's rule for it when a press and release land on it.

#### Scenario: Manage Company at unit 4
- **WHEN** Manage Company opens on a 1280 × 720 screen at unit 4
- **THEN** the form is 708 × 468 at 286, 126, Manage sits at 40, 330 (120 × 28) and Recruit at 40, 370

#### Scenario: Dismissing a fighter
- **WHEN** the loadout form steps Next to an employee, Dismiss is pressed and the dialog's Dismiss is pressed
- **THEN** the dialog asks "Are you sure you want to dismiss " the name "?", the employee leaves the company and the list shows the manager again

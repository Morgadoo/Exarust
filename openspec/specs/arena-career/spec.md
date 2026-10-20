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

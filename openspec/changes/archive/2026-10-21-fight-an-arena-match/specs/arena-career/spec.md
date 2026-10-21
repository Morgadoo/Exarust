## ADDED Requirements

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

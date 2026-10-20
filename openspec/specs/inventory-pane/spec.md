# inventory-pane Specification

## Purpose
The inventory pane: where an icon sits, when a drop is refused, and which icon a point hits.

## Requirements

### Requirement: Drop
A drop from outside SHALL place the icon at the cursor minus the grab, clamp it inside the pane, and add its bulk. A drop from the same pane SHALL only move that icon. An item larger than the pane's limit SHALL be refused.

#### Scenario: Place and move
- **WHEN** an icon of bulk 2 is dropped from outside near the far corner of a 100 by 80 pane, then dropped again from the same pane
- **THEN** it is clamped inside, the bulk stays 2, and the second drop only changes its position

### Requirement: Capacity
A drop from outside SHALL be refused when the bulk would pass the capacity. A drop from the linked pane SHALL not be refused for capacity. A container SHALL NOT be dropped onto the pane that shows it.

#### Scenario: Linked and full
- **WHEN** a bulk of 2 is dropped into a pane of capacity 1 from the linked pane, then a bulk of 1 is dropped from outside, then the pane's own container is dropped
- **THEN** the linked drop lands, the outside drop is refused, and the container is refused

### Requirement: Drag and hit
Ending a drag SHALL remove the icon. The hit SHALL be the top-most icon whose rectangle holds the point.

#### Scenario: Top icon
- **WHEN** two overlapping icons are dropped and the point lies in both, then the lower icon's drag ends
- **THEN** the hit was the upper icon, and the lower icon is gone

### Requirement: Marked drops
A pane SHALL refuse an item that shares a bit with its reject mask. It SHALL refuse an item whose flags, masked by the required mask, are not the required value. An own-only pane SHALL accept only an id it already lists with the low bit set. A limit of 0 SHALL not refuse for size. A capacity of 0 SHALL not refuse for bulk.

#### Scenario: A mark, a list, and a zero limit
- **WHEN** a marked item is dropped, an item without the required bit is dropped, an unlisted id is dropped into an own-only pane, a listed id with the low bit is dropped, and a large item is dropped into a limit of 0 and a capacity of 0
- **THEN** the marked item is refused, the missing bit is refused, the unlisted id is refused, the listed id lands, and the large item lands

### Requirement: Item card page
An item card SHALL be absent for an id below 1, a container, a missing model, and a model mark of 0. A map SHALL be a map and have no value. Otherwise the value SHALL be the coin word divided by 4, unless the high bit is set, which keeps the word. A weapon that is not withheld SHALL be the weapon page. An armour type from 1 through 6 SHALL be the apparel page. A shield that is not withheld SHALL be the shield page. Any other item SHALL be the plain page.

#### Scenario: A weapon, a coat, a shield, and a map
- **WHEN** an id of 0, a container, a map, a missing model, and a mark of 0 are chosen, a weapon of 7 coins is chosen, the same weapon keeps its full word, a withheld weapon of armour 6, 0, and 7 is chosen, and a plain item of 3 coins is chosen
- **THEN** the first five have no stats value, the map is a map, the weapon value is 1 and then 7, armour 6 is apparel, armour 0 is a shield, armour 7 is plain, and 3 coins print 0

### Requirement: Weapon bars
A weapon card SHALL show a slash, crush, pierce, or thrust row only when that byte is above 5. The shown bar SHALL be that byte times 0.05.

#### Scenario: Five stays hidden and six is shown
- **WHEN** slash is 5, crush is 6, pierce is 0, and thrust is 20
- **THEN** slash and pierce are absent, crush is 0.3, and thrust is 1

### Requirement: Apparel bars
An apparel card SHALL always show coverage as that byte times 0.05. It SHALL show impact, slash, crush, pierce, and encumbrance only when that byte times 0.05 is above 0.25.

#### Scenario: A quarter stays hidden
- **WHEN** coverage is 5, impact is 5, slash is 6, crush is 0, pierce is 20, and encumbrance is 5
- **THEN** coverage is 0.25, impact, crush, and encumbrance are absent, slash is 0.3, and pierce is 1

### Requirement: Card icon
A plain card SHALL place its icon by dividing the span minus the icon size by 2, and by dividing the content minus the icon size by 2 and then adding four times the unit. A stats card SHALL place it at thirteen times the unit minus the icon size shifted down one bit, and at the unit. When eleven times the unit is below the picture width, that horizontal place SHALL add the picture width minus eleven times the unit.

#### Scenario: A plain centre and a wide stats shift
- **WHEN** a plain card of span 268 and content 160 uses an icon of 160 and a unit of 4, a plain card of span 269 and content 100 does the same, and a stats card uses a picture of 40, 44, 45, and 160
- **THEN** the first place is 54, 16, the second is 54, -14, pictures 40 and 44 stay at -28, 4, picture 45 is -27, 4, and picture 160 is 88, 4

### Requirement: Shield bars
A shield card SHALL show weight as that byte times 0.01 and impact as that byte times 0.05. It SHALL show encumbrance as that byte times 0.05, and SHALL omit it when the byte is 0. When the flags share a bit with 0x06000000, weight SHALL be 1 and encumbrance SHALL be 8.

#### Scenario: A zero burden stays hidden
- **WHEN** flags are 0 with weight 5, impact 20, and encumbrance 0, then encumbrance is 1, then flags are 0x02000000 with weight 5, impact 0, and encumbrance 0
- **THEN** the first weight is 0.05, impact is 1, and encumbrance is absent, the second encumbrance is 0.05, and the marked weight is 1, impact is 0, and encumbrance is 8

### Requirement: Rank and points
Entering arena SHALL turn rank and points on. Entering story SHALL turn them off. Entering practice SHALL turn them off unless the mode being left already has bit 2. The same mode, and any other mode, SHALL leave them. A shown rank SHALL use the skill byte's top three bits: zero is absent, then Aspirant, Novice, Adept, Expert, Master, a dash, and an empty seventh. Shown points SHALL be the low five bits times one half, and zero SHALL be absent.

#### Scenario: Arena opens the rows
- **WHEN** menu 0 enters arena 8, arena enters story 4, story enters practice 2, practice stays practice, mode 6 enters practice, and arena enters mode 1, then a shown skill of 0, 0x21, 0xc0, 0xe0, and 0x1f is read, and a closed gate reads 0xdf
- **THEN** arena opens both, story and that practice close both, staying and mode 6 and mode 1 leave them open, skill 0 is empty, 0x21 is Aspirant and 0.5, 0xc0 is a dash, 0xe0 has no name, 0x1f is 15.5, and the closed gate shows neither

### Requirement: Description phrase
A phrase whose low and high include the value SHALL be a candidate, in table order. No candidate SHALL leave the roll unchanged and add nothing. Otherwise the roll SHALL become that roll times 0x08088405 plus 1, and the high half of that times the count SHALL choose one. A repeated word longer than three letters, ignoring the last character and letter case, SHALL add nothing. A comma phrase SHALL gain a dot. Any other phrase SHALL gain a leading space and a dot.

#### Scenario: A repeated word stays out
- **WHEN** value 11 misses a row of 0 to 10 from roll 7, value 5 takes "fine" from roll 0, roll 16 chooses between "alpha" and "beta", "fine x" and "Fine x" meet "fine work", "fin x" and "fine" meet "fine work", a comma phrase is chosen, and "the x" meets "the item here"
- **THEN** the miss keeps roll 7 and adds nothing, "fine" becomes " fine." at roll 1, roll 16 becomes 0x80884051 and " beta.", both "fine" texts add nothing, "fin x" and "fine" become " fine work.", the comma phrase becomes ", but torn.", and "the x" becomes " the item here."

### Requirement: Weapon sentence
A name shorter than three letters SHALL stay unchanged. A longer name SHALL drop a trailing dot, and a bracket SHALL lose four letters. A `c` before it SHALL add "It is" and a condition phrase. A dash SHALL use the high and great tables. Otherwise the name SHALL use made, then flawless, then perfect. A mark SHALL be the byte divided by 2.55, dropping the fraction. The same side of 50 SHALL join with ", but", else " and". Unbracketed text SHALL take "An " for a, e, i, o, or u, else "A ".

#### Scenario: A sword, a bracket, and a dash
- **WHEN** "ab" is wrapped, "sword" uses "well" and "sharp", "edge." uses "edge", "Sword [x]" uses quality 0 and then 131, "Blade [c]" is wrapped, "-blade" is wrapped, and flawless misses so perfect supplies "ok"
- **THEN** "ab" stays "ab", "sword" is "A well sword sharp." at roll 0x08088406, "edge." is "An edge edge sharp.", the low bracket is "SwordIt is well, but sharp.", the high bracket is "SwordIt is well and sharp.", the marked blade is "BladeIt is sharp." at roll 1, the dash is "A prime blade keen.", and the miss is "A well sword ok."

### Requirement: Fixed name
When the id word is -1, a name shorter than three letters SHALL stay unchanged and the roll SHALL not step. A longer name SHALL delete a 0x10 byte and the byte after it, shortening by two. It SHALL add "An " or "A " only when the first byte is at least Z.

#### Scenario: A mark comes out and an article goes on
- **WHEN** "ab" is fixed, "Sword", "axe", "Zed", and "zed" are fixed, and the names "ab" plus a mark plus "xy", "ab" plus a mark plus "c", a mark plus "cde", and "Za" plus a mark plus "bc" are fixed
- **THEN** "ab" stays "ab" at the same roll, "Sword" stays "Sword", "axe" is "An axe", "Zed" is "A Zed", "zed" is "A zed", the middle mark becomes "An aby", the end mark becomes "An ab", the leading mark becomes "A de", and "Za" becomes "A Zac"

### Requirement: Apparel sentence
Id -1 SHALL keep the roll and use the fixed-name edit. A ] SHALL drop 4 letters. p and not c SHALL use They are. c SHALL add It is and skip quality. Else a bracket SHALL use It is. Same side of 50 SHALL join ", but", else " and". Leading "A " SHALL be stripped. A non-final dot tail SHALL follow the phrase. Only 14 made rows SHALL be read, then flawless, then perfect. Material 1 to 8 SHALL uppercase the side and replace a 0x10 pair with the name and a space. Any other 0x10 SHALL be deleted.

#### Scenario: A coat, a bracket, and a material
- **WHEN** "ab" is wrapped, "axe" is fixed, "coat" uses "well" and "sharp", "edge" is the made phrase, "Coat[p!]" uses quality 0, then 131, then 131 and 131, "Coat[x!]" is wrapped, "ax" plus a mark plus "e[pc]" is crafted, "a coat" and "coat." are wrapped, "A coat. lined" is wrapped, "ab" plus a mark plus "xy" has material 9 and then material 1 named "wool", a mark before "cde" uses "oak", and 14 missed rows hide a 15th "yes"
- **THEN** "ab" stays at the same roll, "axe" is "An axe" at the same roll, "coat" is "A well coat sharp." at roll 0x08088406, material 1 on "coat" keeps that text and the side becomes "Wool line", "edge" is "An edge coat sharp.", quality 0 is "CoatThey are well, but sharp.", quality 131 is "CoatThey are well and sharp.", both high marks are "CoatThey are well, but sharp.", the plain bracket is "CoatIt is well, but sharp.", the crafted mark stays in "ax" plus a mark plus "eIt is sharp." at roll 1 and the side stays, "a coat" is "A well a coat sharp.", "coat." is "A well coat sharp.", the dotted coat is "A well coat sharp. lined", material 9 becomes "A well aby sharp." and keeps the side, "wool" becomes "A well abwool y sharp." with side "Wool line", "oak" becomes "A well oak de sharp.", and the hidden row leaves "A  coat ok." at roll 1

### Requirement: Item release
The release SHALL clear bit 0 of the 0x14 flag word of every counted entry and run the base once.

#### Scenario: The release sweep
- **WHEN** the flag words are 3, 2 and 1
- **THEN** they become 2, 2 and 0
- **AND** one base call runs

### Requirement: Item scale
The scale SHALL divide each entry's x and y by the argument and multiply by the unit, rounding to nearest even, and run the base once.

#### Scenario: The scale
- **WHEN** the positions are 100, 200 and 50, 25 with divisor 4 and unit 10
- **THEN** they become 250, 500 and 125, 62
- **AND** one base call runs

### Requirement: Container destroy
The destroy SHALL make one release and one final call on a set free flag, call one per-entry release for each counted entry through 0x107a90 when detached and 0x1079e0 otherwise, destroy the array once and run the base once.

#### Scenario: The freeing path
- **WHEN** the free flag is set, the container is detached and 3 entries are counted
- **THEN** one release, three entry calls, one array teardown, one base and one final call run
- **WHEN** the free flag is clear and no entry is counted
- **THEN** only the array teardown and the base run

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
A name shorter than three letters SHALL stay unchanged. A longer name SHALL drop a trailing dot, and a bracket SHALL lose four letters. A `c` before it SHALL add "It is" and a condition phrase. A dash SHALL use the high and great tables. Otherwise the name SHALL use made, then flawless, then perfect. A mark SHALL be the byte divided by 2.55, rounded half to even. The same side of 50 SHALL join with ", but", else " and". Unbracketed text SHALL take "An " for a, e, i, o, or u, else "A ".

#### Scenario: A sword, a bracket, and a dash
- **WHEN** "ab" is wrapped, "sword" uses "well" and "sharp", "edge." uses "edge", "Sword [x]" uses quality 0 and then 131, "Blade [c]" is wrapped, "-blade" is wrapped, and flawless misses so perfect supplies "ok"
- **THEN** "ab" stays "ab", "sword" is "A well sword sharp." at roll 0x08088406, "edge." is "An edge edge sharp.", the low bracket is "SwordIt is well, but sharp.", the high bracket is "SwordIt is well and sharp.", the marked blade is "BladeIt is sharp." at roll 1, the dash is "A prime blade keen.", and the miss is "A well sword ok."

#### Scenario: A mark just past the half
- **WHEN** "Sword [x]" uses quality 129 and condition 0
- **THEN** the quality mark is 51, past the half, and the sentence is "SwordIt is well and sharp."

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

### Requirement: Inventory window
The inventory window SHALL be 244 by 644 units (628 compact, 590 inside the arsenal) with an 8-unit border, its doll filling all but 260 units of the height above a 228 by 180 pack of capacity 400 and items up to 50. The pack's used capacity SHALL count every listed item, held ones too, and everything worn. A drag out of the pack SHALL be refused while the player swaps its weapon sets. A right-button release SHALL close the window.

#### Scenario: The normal window
- **WHEN** the window is built in the normal layout
- **THEN** it is 244 by 644 units and its doll 228 by 384

### Requirement: Arsenal
The arsenal SHALL show as many 236-unit columns as fit after a 12-unit margin and 61 units for a shown inventory, 60 units apart and at most five, with the scroll clamped to the columns left. An item dragged out of the columns SHALL stay in its column marked 0x200; one dropped back from elsewhere SHALL clear its copy's mark when its set is the current one, and otherwise go back to its owning set.

#### Scenario: Four columns
- **WHEN** a 1280-pixel arsenal at unit 4 shows its inventory and asks for a scroll of 3
- **THEN** four columns show, the scroll is 1, the left arrow is on and the right arrow off

### Requirement: Icon cache
An icon SHALL be cached per item and variant; a lit model SHALL keep its framing bit and condition byte and gain the lit bit, any other only its framing bit. A match SHALL add a reference; the last release SHALL free the slot for reuse; a release past it SHALL report "Item reference underflow.".

#### Scenario: Shared and freed
- **WHEN** an item's icon is asked for twice and released twice, then another item is asked for
- **THEN** the second request renders nothing new, the second release frees the slot, and the new item takes it

### Requirement: Item GUI controls
Rating dots SHALL be whole up to the value less a quarter, with a half dot for a remainder above a quarter. An icon's halo SHALL make each inner pixel's alpha the clamped sum of the five-pixel cross around it. A popup SHALL open centred on the cursor and kept on the screen, its bottom 8 units below the cursor.

#### Scenario: Three and a half dots
- **WHEN** a rating of 3.5 is drawn at unit 4
- **THEN** three dots go 12 pixels apart and a half dot follows

### Requirement: Socket rows
A crystal dropped into a socket SHALL replace the one there, which goes back with its charge; a drag from the same socket SHALL be refused, and a finished drag SHALL clear the socket's kind and keep its charge byte. Committing a socket row SHALL write it into the item's weapon section, else its apparel section, and an AR crystal SHALL fire the game's fourth function.

#### Scenario: A socket row's commit
- **WHEN** a row holding the AR crystal is committed into an item with a weapon and an apparel section, then one with only apparel, then one with neither
- **THEN** the first goes to the weapon section and fires the function, the second to the apparel section, and the third nowhere

### Requirement: Panel entries
A panel entry SHALL not be drawn while dragged, held or without an icon; it SHALL be tinted green by the green mask, yellow at half opacity by the yellow mask alone, green at half opacity by both, else black.

#### Scenario: The tints
- **WHEN** entries flagged with the green mask, both masks, the yellow mask and neither are drawn, and a held one
- **THEN** they are green, half-opaque green, half-opaque yellow and black, and the held one is not drawn

### Requirement: Looted worn panel
A looted body's worn panel SHALL leave out every worn item or held shield the icon cache cannot draw, without drawing random numbers for it, scatter the rest over the middle half of the panel (a quarter plus a draw below half of each side) and push them apart over four rounds, measuring both axes with the icons' horizontal half extents and keeping every centre inside.

#### Scenario: An item without an icon
- **WHEN** a looted body wears two items and the cache cannot draw the first
- **THEN** the panel holds only the second, placed where it would be had the body worn it alone

### Requirement: Weapon handling
A weapon card SHALL show weight w·0.01, balance b·0.05 and handling 5·(a + f) with a = (g − 127)·0.01 and f = ((1 − b·0.01)·0.6 + w·0.004 + 0.1)/1.04 for a balance byte up to 100 or (w·0.004 + 0.1)/1.04 above, each product in double precision narrowed to single as the original computes it; a weapon flagged 0x6000000 SHALL show weight 1, handling 5 and balance 0. The striking side's shown rows SHALL fill the card from the first in slash, crush, pierce order at byte · 0.05.

#### Scenario: A balanced weapon
- **WHEN** a weapon of weight 100, balance 50 and grip 127 is shown
- **THEN** its weight is 1, its balance 2.5 and its handling about 3.846

### Requirement: Card condition blocks
A card's description SHALL first take its condition block: blocks start at an at sign with a two-digit limit five and six bytes after it (00 meaning 100) and text from eight bytes after it to three bytes before the next at sign, and the first block whose limit reaches the condition byte's percentage SHALL replace the text from the first block on.

#### Scenario: A worn item's text
- **WHEN** a text with a 50 block and a 00 block is shown for condition bytes 128 and 255
- **THEN** the first shows the 50 block's text and the second the 00 block's

### Requirement: Card pronouns
A card's pronoun markers SHALL be replaced by the name in their last field: HeShe, HimHer, HisHer or HisHers give he or she, him or her, his or her, his or hers by the owner's sex; an unknown name SHALL give nothing; the word SHALL be capitalised at the start of the text or two bytes after a dot.

#### Scenario: A sentence start
- **WHEN** a marker naming HeShe follows ". " for a female owner
- **THEN** it reads "She"

### Requirement: Card gates
A weapon card's material swatch SHALL add its height only when more than two striking rows (thrust included) show. A weapon's points row SHALL show while the skill's low five bits are set and SHALL read those bits, less one but not below 0 when the item flags' `0x30` bits are `0x10`, times one half. The value row SHALL follow its own gate. A shield with the fixed flags SHALL show no rank, points or value row.

#### Scenario: Swatch, points and a fixed shield
- **WHEN** a weapon shows slash and thrust only with a swatch, flags 0x10 and skill 0x65, and a fixed shield is shown with both gates open
- **THEN** the swatch adds no height, the points are 2, and the shield has no rank, points or value

### Requirement: Card title tint
Only the weapon and apparel cards SHALL tint the title, and only while the rank gate is off. The weight SHALL be a double-precision product narrowed to single, and each blended channel SHALL round half to even.

#### Scenario: Half way
- **WHEN** a channel blends from 0 to 255 by 0.5 with the rank gate off
- **THEN** it is 128

### Requirement: Card refresh slots
Refreshing an open weapon card SHALL write each non-zero slash, crush and pierce byte times 0.05 into the next striking bar from 0xd03, in that order.

#### Scenario: A zero slash
- **WHEN** slash is 0, crush 3 and pierce 40
- **THEN** 0xd03 holds 0.15 and 0xd04 holds 2

### Requirement: Panel mouse order
The panel's mouse event SHALL do nothing for the base form's bit or without the item bit, then SHALL start a drag for the 0x4000 bit, then SHALL treat 0x8000 as the double click and 0x2000 as the card. A double click on a use-class-1 item SHALL close the card. The card SHALL be delayed for a use section of kind 0, 1 or 2 (kind 2 running the use first) and built at once for a higher kind or no section.

#### Scenario: Drag wins
- **WHEN** a press carries 0x4000, 0x8000 and 0x2000 over an item that allows dragging
- **THEN** the drag starts

### Requirement: Icon render plan
The icon render SHALL project through the packed transform as `(x·a + y·b) + (z·c + w)`, SHALL take the extents' span in single precision and widen it by 0.0001 in double, and SHALL frame the scene's depth along the matrix's second row starting from the first kept node's stored depth. While the alpha global is set the screen-quad pass SHALL replace the downsample and the halo.

#### Scenario: Paired sums
- **WHEN** the point (1e8, 1, −1e8) goes through a row of ones with translation 1
- **THEN** the projected value is 0

### Requirement: Body loot panels
When a body's loot window opens, its pack SHALL hold the body's carried items at their saved cells (sixteenths of a GUI unit scaled by the unit) and its equipment panel SHALL hold the worn items in wearing order followed by each item of the set in the hands that has a shield section; held weapons and empty slots SHALL be left out.

#### Scenario: A record's panels
- **WHEN** a body carries 0x30 at (40, 4) and 0x31 at (8, 16), wears 0xa0 and 0xa1, and holds 0x11 and the shield 0x12, at GUI unit 4
- **THEN** the pack holds 0x30 at (10, 1) and 0x31 at (2, 4), and the equipment panel lists 0xa0, 0xa1 and 0x12

#### Scenario: A real corpse
- **WHEN** each dead body of exanima01 with carried items has its loot window bound
- **THEN** its pack lists the record's carried items in order with their flags

### Requirement: Body item transfer
A drag from a body's panel dropped on another pack SHALL move the item from the body's possessions into the destination's carried list at the drop's cell, removing a worn item, a hand item with its held carried copy, or the last carried copy; a drop on the same panel SHALL only reposition the entry, and a drop on an equipment panel or outside every panel SHALL be refused.

#### Scenario: Taking off a worn item
- **WHEN** the worn item 0xa0 is moved to the player's pack at cell (40, 60) with unit 4
- **THEN** the body wears only 0xa1 and the player carries 0xa0 saved at (160, 240)

### Requirement: Loot window flags
The loot window's show rule SHALL read the body's record profession word and refuse a body whose race byte is 3, and the deferred close SHALL close a window whose body's condition is no longer 0xa00.

#### Scenario: Race three
- **WHEN** a loot window is requested for a dead body of race 3
- **THEN** no window opens

#### Scenario: Revived body
- **WHEN** a body's loot window is open and its condition changes from 0xa00 to 0
- **THEN** the window closes

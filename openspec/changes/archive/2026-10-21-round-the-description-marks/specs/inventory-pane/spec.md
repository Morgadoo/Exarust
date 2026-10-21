## MODIFIED Requirements

### Requirement: Weapon sentence
A name shorter than three letters SHALL stay unchanged. A longer name SHALL drop a trailing dot, and a bracket SHALL lose four letters. A `c` before it SHALL add "It is" and a condition phrase. A dash SHALL use the high and great tables. Otherwise the name SHALL use made, then flawless, then perfect. A mark SHALL be the byte divided by 2.55, rounded half to even. The same side of 50 SHALL join with ", but", else " and". Unbracketed text SHALL take "An " for a, e, i, o, or u, else "A ".

#### Scenario: A sword, a bracket, and a dash
- **WHEN** "ab" is wrapped, "sword" uses "well" and "sharp", "edge." uses "edge", "Sword [x]" uses quality 0 and then 131, "Blade [c]" is wrapped, "-blade" is wrapped, and flawless misses so perfect supplies "ok"
- **THEN** "ab" stays "ab", "sword" is "A well sword sharp." at roll 0x08088406, "edge." is "An edge edge sharp.", the low bracket is "SwordIt is well, but sharp.", the high bracket is "SwordIt is well and sharp.", the marked blade is "BladeIt is sharp." at roll 1, the dash is "A prime blade keen.", and the miss is "A well sword ok."

#### Scenario: A mark just past the half
- **WHEN** "Sword [x]" uses quality 129 and condition 0
- **THEN** the quality mark is 51, past the half, and the sentence is "SwordIt is well and sharp."

## MODIFIED Requirements

### Requirement: Power tree
A power tree SHALL refuse a file whose first word is not the power magic. The node count SHALL be the word at byte 52. Each node SHALL carry a power id, the index of its name text, the index of its description text, an x, a y, and 16-byte links. A link count below 1 SHALL store no links. Every node after the first SHALL subtract the first node's x and y. A file that ends before the count SHALL be an error.

#### Scenario: Two nodes and a bad file
- **WHEN** a tree with a node at 10, 20 and a node at 15, 25 with one link is read, a link count below 1 is read, a file with a zero first word is read, and a file that ends after the magic is read
- **THEN** the first node stays at 10, 20, the second is at 5, 5 with that link, the low link count stores none, and the other two reads are errors

#### Scenario: Text indices
- **WHEN** a node whose two words after the power are 3 and 4 is read
- **THEN** its name text index is 3 and its description text index is 4

### Requirement: Tree text
A text chunk after the nodes SHALL give a count that is its last string's index, so it holds count + 1 offsets and strings. Each string SHALL be a length word and the bytes that follow it, at that string's offset in the text. A length below 1 SHALL be empty. A string whose offset or bytes lie outside the text SHALL be empty. A tree with no text chunk SHALL have no strings. A text chunk that ends before its table or text SHALL be an error.

#### Scenario: Blast, an empty string, and a short chunk
- **WHEN** a tree whose text chunk has the count 1, the offsets 0 and 9, and holds Blast and a length of 0 is read, the same nodes with no text chunk are read, and a text chunk that ends after its first word is read
- **THEN** the first text is Blast, the second is empty, there are two texts, the bare tree has no texts, and the short chunk is an error

#### Scenario: An offset past the text
- **WHEN** a text chunk has an offset that lies past its text
- **THEN** that string is empty and the others are read

## ADDED Requirements

### Requirement: Power grant target
A grant SHALL need the seventh insight technique. The target SHALL be the last node whose power, with bits 20 to 23 cleared, equals the granted power, and it SHALL not be the first node. Its first link SHALL name a prerequisite node, one-based; zero names none, and a name past the nodes reads as unlearnt.

#### Scenario: Refusals
- **WHEN** the first node is granted, a node with no link is granted, a power no node holds is granted, a node whose link names a node past the list is granted, and a linked node is granted without the technique
- **THEN** every grant is refused and the known list is unchanged

### Requirement: Power grant write
A node's flags SHALL be the flags of the last known pair whose id, with bits 20 to 23 cleared, equals the node's word, or zero. The prerequisite SHALL have bit 0x1000. The power SHALL then overwrite the first known pair that matches it with the flags 0x9fff, or be appended.

#### Scenario: A prerequisite and a granted chain
- **WHEN** power 0x102, linked to the learnt node 0x101, is granted with the technique, then 0x103 linked to the granted 0x102
- **THEN** the first grant appends 0x102 with 0x9fff, the second succeeds because the granted flags carry the learnt bit, and the list holds three pairs

### Requirement: Tree opening
A tree opened by a character other than the last one SHALL open the mind domain; the same character SHALL reopen the last domain. A record that knows no powers SHALL be seeded with two pairs: 0x101 with 0x1000, and 0x01020101 with 0x2800. A record that knows powers SHALL not be seeded.

#### Scenario: First and repeated opening
- **WHEN** the domain is chosen for a new character after the displacement domain, for the same character after it, and an empty record and a starter record are seeded
- **THEN** the domains are mind and displacement, the empty record gets the two pairs, and the starter record keeps its two

### Requirement: Running power limit
Running powers SHALL be counted in list order, skipping any whose id has bit 0x100000. Each power counted beyond the allowed slots SHALL be set to state 2, and the check SHALL report failure when it set one.

#### Scenario: One slot
- **WHEN** powers 0x20207, 0x20208 and 0x102020b run in state 1 and one slot is allowed, and then power 0x20207 alone runs with one slot
- **THEN** the first stays in state 1, the other two are set to state 2 and the check fails, and the lone power passes

### Requirement: Power bar slots
The bar SHALL lie ten slots across, 68 pixels apart, the first centred 194 pixels from the left at scale 4. A press within the slot's half-width SHALL name the slot, and a press left of the first slot SHALL name none. The enlarged row SHALL take the nearest slot within 48 pixels.

#### Scenario: First, last and outside
- **WHEN** the bar is pressed at 194, at 194 plus nine pitches, 29 pixels left of the first slot, and, enlarged, 10 pixels short of the second slot
- **THEN** the slots are 0, 9, none and 1

### Requirement: Effect scroll
A centred texture scroll SHALL give each layer `((layer + frame) + 0.5) * 0.002 / divisor + base - 0.001`, taken in double precision and narrowed once, and any other `((layer + frame) + 0.5) * step / divisor + base - step * 0.5` in single precision. The divisor SHALL be the effect shader's global, which holds 1. Zero layers SHALL give no offsets.

#### Scenario: Three layers
- **WHEN** a centred scroll of 3 layers at frame 10 and base 0.5 and a plain scroll of 2 layers at frame 4, base 0.25 and step 0.75 are taken
- **THEN** the centred offsets are 10.5, 11.5 and 12.5 times 0.002, plus 0.5, less 0.001, and the plain first offset is 4.5 times 0.75, plus 0.25, less 0.375

### Requirement: Force creator looks
Each Force creator SHALL look up its sound ids and store them without playing them. The blast family SHALL wear the force material, the domes and the barricade the barrier material and the shell the aura material. The barricade SHALL carry its sound ids, then a pitched play of `thmfrc_ss` at 0.8 + 0.4 of a draw, and the shell and the strikes a pitched play of `thmfrc_ss` at 0.6 + 0.4 of a draw, at volume 0.3. The materials SHALL bind the parameters 0.25, 0.25, 0.25 and 1.

#### Scenario: Materials and sounds
- **WHEN** the blast, bolt, barrier and shell creators are read, the shell start pitch is taken for a draw of 0.5, and the bubble's fourth sound is read
- **THEN** the blast sounds are `thmfrc_cu01` and `thmfrc_bb`, the bolt's second sound is `thmfrc_blt`, the barrier wears the barrier material with a 0xb8-byte block, the shell wears the aura material and its active power is 0x20208, the pitch is 0.8, and the bubble's fourth sound is `thmfrc_brf`

### Requirement: Constant displacement veil
A constantly displaced body SHALL start its hum at the product of 2, 0.8 and a draw, taken in double precision and narrowed. The displaced bit SHALL be set on the body. The first hit SHALL pass when the streak is zero and clear bits 0xC000 of the hit node and its children. The restore after half a second SHALL give the phased nodes back and end the shell.

#### Scenario: A hit and its restore
- **WHEN** a veil over a body with one child starts, is hit at 1000 ms on its first child, and is restored at 1500 and at 1501 ms
- **THEN** the hum start is the double-precision product, the hit passes and records its wisp, the 1500 ms restore gives nothing, and the 1501 ms restore gives that child back with the shell event ended

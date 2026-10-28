## ADDED Requirements

### Requirement: Prop collision sounds
Each body tick, the game SHALL ring a prop's pair impacts in pair order, then its level impacts. An impact SHALL sound when it is above 0.5 and its part's clock has passed or it is above the part's threshold. The part SHALL play its material collision set as a one-shot at its mesh centre in the world. Held items and character bodies SHALL be left to their own sound paths.

#### Scenario: A crate dropped on the floor
- **WHEN** a wooden prop part of contact mobility 0.3 takes a level impact of 50 at game time 1000 ms with the generator at 1
- **THEN** its `wdlcol` set plays at volume 0.75 and pitch about 0.906, its threshold becomes 100 and its clock 1138 ms

#### Scenario: A second impact in the same tick
- **WHEN** the same part takes another impact of 60 in that tick
- **THEN** nothing more plays, since the clock holds and 60 is not above 100

### Requirement: Prop collision clock
A sounding part SHALL store twice the strength as its threshold and set its clock to `now + 220 − round(170·v)` plus a draw below 100 ms taken after the play. A part whose set has no shipped variant SHALL stay silent and park its clock at the all-ones word.

#### Scenario: A part without a shipped set
- **WHEN** a part whose collision set has no variant takes an impact of 50 past its clock
- **THEN** nothing plays, no draw is taken and its clock becomes the all-ones word

### Requirement: Scrape loops
Each scrape source of a body SHALL drive one looping voice: over the speed floor a voice that is not playing SHALL start on a variant of the source's set at volume and pitch 1, and the voice SHALL then take the source's volume, pitch and point; under the floor a playing voice SHALL be asked to stop. A body that leaves the world SHALL free its voices. A source whose sound has no shipped variant SHALL never bind a voice.

#### Scenario: A flail chain dragged
- **WHEN** a source's speed rises over the floor with volume 0.06 and pitch 1.0836
- **THEN** a looping voice starts and plays at gain 0.06 and pitch 1.0836 at the source's point, and gets the stop bit once the speed falls under the floor

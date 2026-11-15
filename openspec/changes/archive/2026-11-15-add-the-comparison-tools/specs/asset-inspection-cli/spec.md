## ADDED Requirements

### Requirement: Picture comparison
`exa-tool picdiff <a> <b>` SHALL compare two PNGs of one size (or the second resampled with `--fit`) without an installation: a pixel changed when its largest RGB channel difference is above the threshold (default 8). It SHALL report the changed count and share, the largest, mean, 99th and 99.9th percentile difference and the PSNR, leaving out `--ignore x,y,w,h` rectangles.

#### Scenario: A changed block
- **WHEN** a 32 by 16 block of one picture is recoloured
- **THEN** 512 pixels are changed and one region has the block's bounds

### Requirement: Picture regions
The changed pixels SHALL be grouped into regions: tiles (16 pixels by default) holding at least the minimum count (4) are marked, and touching marked tiles (eight neighbours) form one region, reported with its pixel bounds, the most changed first.

#### Scenario: Two blocks apart
- **WHEN** two separate blocks change
- **THEN** two regions are reported, the larger first

### Requirement: Noise floor
With `--noise <c> <d>` (two runs of one build) the pair's changed share SHALL be judged against the noise pair's: at the floor when no more than the factor (1.5) times it plus the slack (0.0001). Above the floor the command SHALL exit 1.

#### Scenario: A speck over a speck
- **WHEN** the noise pair differs in 10 pixels and the pair in 12
- **THEN** the pair is at the noise floor; at 100 it is above

### Requirement: Comparison pictures
With `--out <prefix>` the command SHALL write `<prefix>.side.png` (both pictures side by side, regions boxed) and `<prefix>.diff.png` (the first dimmed to grey, changed pixels red by size, regions boxed), and with `--json <file>` the report.

#### Scenario: The pictures written
- **WHEN** two 20 by 10 pictures are compared with `--out`
- **THEN** a 48 by 10 side-by-side picture and a diff picture are written

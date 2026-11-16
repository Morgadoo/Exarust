## ADDED Requirements

### Requirement: Inline text pictures
A manual page's `[rimg=NAME]` picture SHALL load once from `NAME.rfi`, keyed by the name's first 16 bytes, take its size over six in GUI units in the page's layout, and draw centred in the text's width in white under the premultiplied blend, cut at the page's client.

#### Scenario: The Interaction page
- **WHEN** the manual shows a page naming a 280 x 170 picture at unit 6
- **THEN** a 276 x 168 picture draws above the text, which starts below it

### Requirement: Tooltip background
The default tooltip's background SHALL be the skin's rounded fill from one pixel into the box, one pixel short of its size, with the corner radius word 2, at the fade's alpha of 153.

#### Scenario: A shown tooltip
- **WHEN** a tooltip has faded in
- **THEN** its background is the rounded program's quad at alpha 153

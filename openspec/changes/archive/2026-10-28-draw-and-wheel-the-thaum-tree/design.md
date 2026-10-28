# Design

- `TreeScreen::shown_zoom` is the original's displayed zoom; the springs
  keep their own value and previous value. Placement, hit tests, the fixed
  zoom and the link width read the shown zoom.
- The scale centre is captured at the start of the frame's presentation,
  before the nodes are placed again, so it is the previous frame's first
  node, as the draw head copies it before stepping.
- `DomainWheel` wraps `effects::wheel::Wheel` rather than a second wheel
  model: one hit test, one confirm rule, one open ease. The tree only
  places it, writes the share into `fade` and the child alpha into
  `shown`, and turns a confirmed choice into `choose_domain`. The running
  screen routes a button over a wheel icon to the wheel (the click on
  release) before the tree sees it, unless a power drag is held.
- The wheel's known domains are the tree's `known_domains` (the record's),
  not a second array.

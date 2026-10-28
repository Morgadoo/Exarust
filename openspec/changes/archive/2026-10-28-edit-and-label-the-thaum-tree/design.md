# Design

- The link-turn drag reads `TreeScreen::focus`, the same id the hover and
  press read: a second copy of it would go stale. The pressed node is
  `node_under`, which the press already stores from the focus's low word.
- The flash's end is checked at the top of `TreeScreen::view`, so the
  frame that finds the flash over draws neither the flash nor the refusal.
  The glows also go back to their dimmed colour on that frame, as the
  original's reset of the flashed node does.
- The glowing caption reuses `exa_ui::skin::glow_text` for its four
  places and inks, with the line skin's palette (the palette the game's
  screens paint with). The tree's batches have no depth test, so the copies
  are drawn before the text and the text stays in front, as the
  original's depth test keeps it.
- The wheel texture's side and size word are functions of the GUI unit,
  because the load multiplies the unit. The running screen still samples
  the shipped 384-pixel sheet.

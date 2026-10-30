# Proposal

## Why

docs/PLAN.md, Phase 6 (the power tree screen, thaumtree lane). The
thaumtree8 pass found three places where the tree screen did not follow
the original: the quick bar's slots faded with the domain wheel child's
show value, the bar and creation-prompt hit tests assumed an interface
unit of four, and the form's two opening paths (the character creator's
and the game's) were not told apart.

## What Changes

- The tree's `1 − r²` opening ease is the domain wheel child's show value
  (`TreeScreen::wheel_shown`, times the window while closing); the bar
  draw no longer multiplies its slots by it and keeps its own fades.
- `TreeScreen::hover` and `PowerBar::drop` take the interface unit; the
  running screen passes the live `GuiScale`, so the bar's slots (17-unit
  pitch, 8-unit top, 12- or 7-unit radius) and the creation prompt's
  buttons (5 to 37 units in from each side, 2 to 9 units up) are hit where
  they are drawn at every interface level.
- The form records its opening words: the creation screen (opened with no
  calling form) has task word `0x10`, hints at `0xff` and no child panel;
  the game's opening has task word `0x1010` and a child panel whose paint
  alpha the draw writes as its shown value less twice the window's.
- docs/THAUMATURGY.md describes the three rules.

## Evidence

REA and the objdump listing of Exanima.exe 0.9.5: the bar's slot hit test
(the interface unit read for pitch, top and both radii, the arithmetic
half-shift of the row start), the tree's hover (the prompt's button
bounds in units), the bar draw (only its own flags, stamps and the form
width), the tree draw head (the wheel child's shown value from the ease,
the child panel's paint alpha from the window and the constant 2.0) and
the form opening (the two task words, the `0xff` hint byte against the
constructor's `0x80`).

## Out of scope

The calling form's child panel object (its shown value is a labelled
guess of one), the character creator page that would open the tree in
creation mode, the skin's panel texture, the Mind helper behind the
controlling bit.

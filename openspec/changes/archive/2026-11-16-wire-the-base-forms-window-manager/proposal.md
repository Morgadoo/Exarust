## Why

Roadmap tasks B1 and B4 (wiring lane `guiform`): the base form
(`TGUIForm`) had about 58 open ledger rows. Its tag search, active test,
layer insertion, close cascade, freed-form sweep, focus memory and the
application's deactivation were ported as rules with test callers only, or
not at all, while the live window manager (`exa_ui::gui_core::GuiTask`)
and the in-game screens did simpler things: the inventory and journal keys
toggled on the hover focus rather than the window on top, a closed window's
id could stay in the focus, the pointer's target or the tooltip request, the
journal's text box was closed by hand, open screens kept the GUI unit they
opened at, and a held record's release carried the landing bit only when
the pointer was over it.

## Evidence

REA (Exanima.exe), checked against the assembly where constants or order
matter:

- The journal opener: the tag search over the task's children (visible
  only), the active test, the close slot on an active window and the focus
  routine on another; the inventory opener's active test.
- The active test (an exact upper-layer word is active as the first child,
  any other as the last child outside the upper layer).
- The focus routine (nil: the remembered pair when it is visible and not the
  focus, else the root with control -1; a change remembers the visible form
  it leaves with its control read before the deselect) and the deactivation
  (the focus deselects, is remembered when visible, the root takes control
  -1).
- The activation (walk to the task's child, raise it in its parent by the
  child rule) and the press dispatch that calls it.
- The freed-form sweep's slots and follow-ups, and the button-up's record
  release (message 0x50 with the release word or 0x10000 and the point).
- The level switch's walk over the root's children and the item container's
  rescale slot (each entry `round(unit · place / old)`).

## What changes

- `exa_ui::gui_core::GuiTask`: `add` inserts by layer; `children_of`,
  `find_by_tag`, `is_active`, `activate`, `raise_child` (the press's
  activation), `focus_form`, `deactivate`, `remembered`, `close_form`,
  `sweep_references`; `draw_order` walks the tree. `FormHost::tag`,
  `BaseForm::tag`.
- `exa_ui::base_form::BaseForm::release`: the landing bit always.
- `exa_game::gui_screens`: tags on the screens; the inventory and journal
  toggles read the active test and activate a window not on top; the
  journal is found by its tag; closes go through `close_form`; the show
  rule's place; `deactivate_on_focus_loss`; `GameScreens::rescale_open` and
  `rescale_open_screens`.
- `exa_game::hub_windows`: the window closes through `close_form`.

Not in: maximise and restore (the line skin answers no maximise mark and no
live form maximises), the grips and the horizontal bar, the client and
control-span refresh, the snap slots, the task list, owned popups (forms
carry no owner), the pad's saved focus on a push, the container popups'
rescale.

## Why

Phase 7 (UI) in docs/PLAN.md, `RF_GUI_Controls_Ex (unit routines)`. Round 147's routine table left three reachable
gaps in the extended controls: the combo boxes' key path in the hub's forms (merchant options, loadout), the colour
swatch's drag in the emblem form, and the text field's own editing in the emblem name and number fields (the hub's
forms appended bytes and popped them, with the focus set by a click, and applied `klean` on any edit).

## What the original does

REA (Exanima.exe) and the assembly of the text field's and combo box's message handlers, their caret, origin and
span helpers, the colour swatch's handler and its drag draw, the emblem form's name and swatch drop callbacks and the
static records of the emblem, merchant and loadout forms:

- **Focus.** The idle pointer hit tests and focuses its target; a text field or combo box gaining the focus gets the
  focus message, which selects its whole text. Keys go to the focused record.
- **Text field keys.** Enter with a callback sets state bit 0x100000 for the call and, with style 0x2000000,
  releases the focus. An editable field (style 0xd) drops a typed non-digit under style 0x4000000, else passes the
  key to the text edit unit with its limit and span, then calls the owner after every key under style 0x1000000. A
  read-only field passes the key to the edit unit's moves.
- **Combo box keys.** Enter sets 0x300000 and runs the callback; any other key goes to the edit unit whatever the
  editable bits, the callback under 0x200000 with the notify style.
- **Caret.** A left press on a hovered field (state 0x10000) places the caret and anchor under the pointer from the
  field's own label origin; a drag places the caret only.
- **Callbacks.** The emblem name's callback acts only on 0x100000: Enter applies `klean`. The merchant's callbacks
  read only the selection word. The loadout list's steps to its word less one when that is not the shown row.
- **Colour drag.** A swatch's drag press starts a drag (code 0xcd0c: colour, press offset, inner size) drawn at
  alpha 0xa0 under the pointer; a swatch takes only that code, stores the colour and runs its drop callback. The
  emblem swatches' drop callback stores the whole word in the form's colour slot and the preview's copy.

## What Changes

- `exa_ui::field_edit`: the two key handlers, the caret press and drag, the field origin, the focus selection, the
  swatch drop and the emblem drop callback.
- `exa_ui::record_form::RecordForm::{hovered, cancel_hold}`.
- `exa_game::option_forms`: the focus follows the pointer; keys go through the handlers (with the system clipboard);
  the field callbacks; the swatch drag with the pointer's threshold and its draw; the text field's front draw through
  the edit unit with its selection and caret. `klean` now waits for Enter.
- `exa_game::hub_forms`: the loadout list's Enter callback; the combo front draw takes the box's edit bytes and focus.
- `exarust --press` names Left, Right, Home, End, Backspace and Delete.

## Impact

Hub forms only (merchant options, emblem form, loadout). The duel and exanima01 lockstep traces equal the base's.

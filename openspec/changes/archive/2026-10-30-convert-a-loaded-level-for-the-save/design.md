# Design

## Hooks instead of virtual calls

The original calls three of the scene objects' own methods while it
exports. Exarust's scene is plain data, so the exporter takes an
`ExportHooks` trait object and calls it at the same points and in the same
order; `export` passes a no-op so its bytes are unchanged.

## Conversion as a separate module

`rfc_level` sits beside the reader and the writer and depends on both: it
reads the reader's metre-based structures and returns the writer's
centimetre scene. The few records the reader drops are scanned from the
chunk tree once (`LevelRaw`) rather than widening the reader.

Round-trip rules that follow from the format: the saved bit is masked out
of stored node flags, so converted nodes set it again; names go back to
their stored bytes (the readers decode one character per byte); a
`_Default` part reads back as no material; a material list's count is the
number of listed names, so the scan takes every record present; the props
and placed objects are numbered through each owner's actual scene index,
because part nodes sit between owners.

## Condition words

`CVTSS2SI` with round-to-nearest gives ties to even; a value it cannot
represent gives the 64-bit integer indefinite value, whose low sixteen
bits are zero, which is what the original stores.

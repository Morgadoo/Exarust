# Design

## Two entry walks

The original reads a character entry in two places. The prelude's
whole-entry builder, after the record, reads a tag and a length word and
then hands the body to the loader through a copy of the position, so its
own position only moves eight bytes per step: every aligned word pair
after the record is a candidate header, nested ones included, and each
items header makes a new database (a library read earlier goes with the
old one). It walks while consumed plus six is under the length. The item
import's reader moves past each body, walks while consumed plus eight is
under the length and loads strings and items into one scratch tier.
Exarust keeps both as `read_entry` and `read_entry_items` rather than one
reader that is right for neither.

## Migration before the import

The import runs the whole-tier migration on the scratch tier before any
copy. A template whose material class changed sets the item's two string
ids to inherit, so the copy then allocates nothing; migrating after the
copy would leave allocated slots behind. `migrate_store` runs the existing
per-record migration on a store outside the database, with the templates
resolved in the live database, and `import_items` calls it first.

## Deviations kept

Out-of-range reads in the original (a section shorter than its rank word,
a string id past the library, a last step with seven bytes left) read as
zero or stop in Exarust; each is named in the code. The original's scratch
tier keeps the previous entry's items when an entry has none; each Exarust
read starts empty.

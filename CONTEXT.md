# `context/` — working memory outside git

Everything an agent needs that is *not* the product lives under `context/`,
which is ignored in full: the journal of how a change was made, research
dumps, Ghidra projects, RenderDoc captures, third-party clones. This file is
the contract for that folder. A fresh clone has these rules and an empty
`context/`.

```
context/
  artifacts/     one folder per task: YYYY-MM-DD-<slug>/
  externals/     third-party clones, for reference only, never a source of truth
  re/            REA/Ghidra snapshots and evidence, hex notes, captures of the original game
```

## Artifacts

A task folder holds a `README.md` (header + one line + pointer to the
current slice) and a column of iterations named `<N>-<NAME>-<SUFFIX>`:

| suffix | meaning | whose move |
|---|---|---|
| `-FINAL` | the slice reached its goal | nobody's |
| `-PART` | code work remains | the next agent |
| `-READY` | done on the code side; next step is a human (playtest, decision) | the human |

```markdown
---
status: active        # planned | active | blocked | landed | closed
updated: 2026-10-13
next: what comes next
---

# Title

One line about what this is.

Current slice: [3-RFC-MESH-PART](3-RFC-MESH-PART.md).
```

Closed iterations are a journal. Do not rewrite them; corrections go in a
new iteration. Living documents (`docs/PLAN.md`, `docs/formats/*`) describe
the present and are updated in place.

A report that hands work over states what it is based on, **what was not
done**, and which probes were removed from the tree before committing.

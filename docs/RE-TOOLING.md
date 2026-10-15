# Reverse-engineering tooling

Format and behaviour work on `Exanima.exe` (Free Pascal 3.2.2, Win64 PE) goes
through **[REA](https://rea.tools/)** ("Reverse Engineer Anything",
[source](https://github.com/morluto/rea), MIT). REA is an MCP server and CLI
that gives agents read-only access to a disassembler. Here it uses the
**Ghidra** provider.

## Requirements

| component | version | why |
|---|---|---|
| Node.js | ≥ 22.19, 24.x, or 26+ | REA runtime |
| rea-agents | 6.4.0 | MCP server and CLI |
| Ghidra | 12.1.x (verified 12.1.4) with the native decompiler | analysis engine |
| JDK | 21 (64-bit, full JDK) | Ghidra's runtime |

## Setup (Linux, user space, no root)

```bash
# 1. Runtimes into ~/.local/opt (verify the published SHA-256 of each download)
#    node-v24.21.0-linux-x64.tar.xz         nodejs.org/dist/v24.21.0
#    OpenJDK21U-jdk_x64_linux_hotspot_*.tar.gz  api.adoptium.net
#    ghidra_12.1.4_PUBLIC_20260921.zip       github.com/NationalSecurityAgency/ghidra/releases

# 2. REA into its own prefix, run with Node 24
PATH=~/.local/opt/node-v24.21.0-linux-x64/bin:$PATH \
  npm install -g --prefix ~/.local/opt/rea rea-agents@6.4.0

# 3. A `rea` wrapper on PATH that pins Node 24 and exports
#    GHIDRA_INSTALL_DIR and JAVA_HOME (see ~/.local/bin/rea on the
#    maintainer's machine), then:
rea setup --client claude_code     # registers MCP in ~/.claude.json, installs the REA skill
rea doctor --client claude_code    # registration + skill "aligned", Ghidra checks ok
```

If the system `node` is older than 22.19, make sure the MCP entry starts REA
with an absolute Node 24 path: `"command": "<node24>/bin/node"`,
`"args": ["<prefix>/lib/node_modules/rea-agents/scripts/rea.mjs", "mcp"]`.
REA's script uses `#!/usr/bin/env node`, which would otherwise pick up the old
runtime. Restart Claude Code after setup. Tools then appear as `mcp__rea__*`,
and the `reverse-engineer-anything` skill is available.

Other agents: `rea setup --client codex` (or `cursor`, `gemini`, …).

### Sandboxed hosts: JVM cgroup warnings

Inside some sandboxes (seen in a distrobox under the Claude Desktop app's
systemd scope), the JVM prints `Cgroup … seems to have moved` warnings on
**stdout**. Ghidra's `LaunchSupport` output then fails to parse, and REA
reports `GhidraLaunchError: Ghidra headless launch failed` about a second
after starting. REA clears `JAVA_TOOL_OPTIONS` / `JDK_JAVA_OPTIONS`, and it
checks that the launched process is exactly `$JAVA_HOME/bin/java`, so neither
an environment variable nor a wrapper script can fix this. Build a JDK image
with the option embedded instead, and point `JAVA_HOME` at it, both in the
`rea` wrapper and in the MCP entry's `env`:

```bash
J=~/.local/opt/jdk-21.0.12.1+1
$J/bin/jlink --module-path $J/jmods --add-modules ALL-MODULE-PATH \
  --add-options="-XX:-UseContainerSupport" --output ~/.local/opt/jdk-21-rea
```

## Use in this project

- Targets are always the user's own `Exanima.exe`. REA imports a private
  copy into a temporary Ghidra project and never modifies the file.
- Save snapshots and evidence under `context/re/` (git-ignored), e.g.
  `rea analyze "$EXANIMA_DIR/Exanima.exe" --provider ghidra --snapshot "$PWD/context/re/exanima.json" --json`.
- Useful entry points: the class names in
  [`research/engine-classes.md`](research/engine-classes.md) (FPC RTTI names
  virtual method tables), chunk tags and magic numbers from
  [`formats/`](formats/README.md), and string references.
- What lands in the tracked tree: the rule or layout, described in Exarust's
  terms, in `docs/formats/` or a spec under `openspec/specs/`. Addresses,
  decompiler output and snapshots stay in `context/`.

CLI equivalents of the MCP tools, for scripts:

```bash
rea search    "$EXE" "TRFCReader"  --provider ghidra --json
rea function  "$EXE" <name|addr>   --provider ghidra --json
rea decompile "$EXE" <addr>        --provider ghidra --json
rea xrefs     "$EXE" <addr>        --provider ghidra --json
```

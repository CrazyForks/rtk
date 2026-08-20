---
title: Configuration
description: Customize RTK behavior via config.toml, environment variables, and per-project filters
sidebar:
  order: 4
---

# Configuration

## Config file location

| Platform | Path |
|----------|------|
| Linux | `~/.config/rtk/config.toml` |
| macOS | `~/Library/Application Support/rtk/config.toml` |

```bash
rtk config            # show current configuration
rtk config --create   # create config file with defaults
```

## Full config structure

```toml
[tracking]
enabled = true              # enable/disable token tracking
history_days = 90           # retention in days (auto-cleanup)
database_path = "/custom/path/history.db"   # optional override

[display]
colors = true               # colored output
emoji = true                # use emojis in output
max_width = 120             # maximum output width

[filters]
# These apply to file-reading commands (ls, find, grep, cat/rtk read).
# Paths matching these patterns are excluded from output, keeping noise low.
ignore_dirs = [".git", "node_modules", "target", "__pycache__", ".venv", "vendor"]
ignore_files = ["*.lock", "*.min.js", "*.min.css"]

[tee]
enabled = true              # save raw output on failure
mode = "failures"           # "failures" (default), "always", "never"
max_files = 20              # rotation: keep last N files
# directory = "/custom/tee/path"  # optional override

[telemetry]
enabled = true              # anonymous daily ping — see Telemetry & Privacy for full details

[hooks]
exclude_commands = []       # commands to never auto-rewrite

[awareness]
level = "default"           # "default", "high", "full" — see Awareness level
```

For full details on what is collected, opt-out options, and GDPR rights, see [Telemetry & Privacy](../resources/telemetry.md).

## Awareness level

`rtk init` writes a short instructions file that your agent loads at the start of every session
(`~/.claude/RTK.md`, `~/.gemini/GEMINI.md`, `~/.vibe/prompts/rtk.md`, …). `awareness.level`
controls how much that file tells the agent about RTK.

### Why the file is nearly silent by default

RTK is a transparent proxy. With a command hook installed, the agent runs `git status`, the hook
rewrites it to `rtk git status`, and the agent sees condensed output without knowing RTK exists.
That is the whole point: no prompt-level instructions to follow, no `rtk` prefixes to remember,
nothing to get wrong.

Earlier versions shipped a `RTK.md` that listed commands, savings percentages, and
"always prefix with `rtk`" rules. That text cost tokens on every turn and taught the agent to
talk about RTK, retry with and without the prefix, or skip the prefix "to see more". The
`default` level replaces it with a few lines that only explain how to read condensed output.

### Levels

| Level | The agent is told | Pick it when |
|-------|-------------------|--------------|
| `default` | Output is condensed; treat it as complete; batch commands; truncated output states its own recovery path; `rtk proxy <cmd>` only if the result is unusable. Nothing about RTK itself. | You have a hook-based agent and want RTK invisible. This is the default. |
| `high` | `default` + what RTK is, that a hook does the rewriting, and the meta commands: `rtk gain`, `rtk proxy <cmd>`, `RTK_DISABLED=1 <cmd>`, `rtk discover`. | You want to be able to ask the agent "how many tokens did RTK save?" or "run that without RTK". |
| `full` | `high` + "prefix every shell command with `rtk`", including inside `&&` chains. | Your agent has no hook, or you deliberately want the agent to drive RTK itself. |

Each level contains the previous one verbatim, so the output-reading rules are identical at every
level; only how much the agent knows about RTK changes.

```toml
[awareness]
level = "high"
```

Then re-run the init command for your agent (`rtk init -g`, `rtk init -g --gemini`,
`rtk init -g --agent vibe`, …). The file is rewritten only when its content changes, and the
summary line shows which level was written (`RTK.md: ~/.claude/RTK.md (awareness: high)`).

### Agents without a hook always get `full`

Codex CLI, Cline, Windsurf, Kilo Code, Antigravity and Kimi have no command hook: RTK only runs
if the agent types `rtk` itself, and only the `full` text tells it to. `rtk init` therefore
writes `full` for those agents regardless of `awareness.level`, and prints a note:

```
Awareness: full — Kilo Code has no command hook, so the agent is told to prefix rtk itself (config awareness.level = "default" does not apply here)
```

One `config.toml` can serve a hook-based agent at `default` and a rules-file agent at `full`
on the same machine; nothing to switch between them.

### Upgrading from an older `RTK.md`

The next `rtk init -g` replaces the old command-list `RTK.md` with the `default` file. If you
relied on the agent knowing RTK (for example asking it to run `rtk gain`), set `level = "high"`
before re-running. `rtk init --claude-md` still writes the legacy full command table into
`CLAUDE.md` for anyone who wants it.

## Environment variables

| Variable | Description |
|----------|-------------|
| `RTK_DISABLED=1` | Disable RTK for a single command (`RTK_DISABLED=1 git status`) |
| `RTK_TEE_DIR` | Override the tee directory |
| `RTK_TELEMETRY_DISABLED=1` | Disable telemetry |
| `RTK_HOOK_AUDIT=1` | Enable hook audit logging |
| `SKIP_ENV_VALIDATION=1` | Skip env validation (useful with Next.js) |

## Tee system

When a command fails, RTK saves the full raw output to a local file and prints the path:

```
FAILED: 2/15 tests
[full output: ~/.local/share/rtk/tee/1707753600_cargo_test.log]
```

Your AI assistant can then read the file if it needs more detail, without re-running the command.

| Setting | Default | Description |
|---------|---------|-------------|
| `tee.enabled` | `true` | Enable/disable |
| `tee.mode` | `"failures"` | `"failures"`, `"always"`, `"never"` |
| `tee.max_files` | `20` | Rotation: keep last N files |
| Min size | 500 bytes | Outputs shorter than this are not saved |
| Max file size | 1 MB | Truncated above this |

## Excluding commands from auto-rewrite

Prevent specific commands from being rewritten by the hook:

```toml
[hooks]
exclude_commands = ["git rebase", "git cherry-pick", "docker exec"]
```

Patterns match against the full command after stripping env prefixes (`sudo`, `VAR=val`), so `"psql"` excludes both `psql -h localhost` and `PGPASSWORD=x psql -h localhost`.

Subcommand patterns work too: `"git push"` excludes `git push origin main` but not `git status`.

Patterns starting with `^` are treated as regex:

```toml
[hooks]
exclude_commands = ["^curl", "^wget", "git rebase"]
```

Invalid regex patterns fall back to prefix matching.

Or for a single invocation:

```bash
RTK_DISABLED=1 git rebase main
```

## Telemetry

RTK sends one anonymous ping per day (23h interval). No personal data, no file paths, no command content.

Data sent: device hash, version, OS, architecture, command count/24h, top commands, savings %.

To opt out:

```bash
# Via environment variable
export RTK_TELEMETRY_DISABLED=1

# Via config.toml
[telemetry]
enabled = false
```

## Custom filters

Add your own filters (or override built-ins) in either location:

- **Project-local** — `.rtk/filters.toml` in your project root (committed with the repo)
- **User-global** — `~/.config/rtk/filters.toml` (applies to every project)

See [`src/filters/README.md`](https://github.com/rtk-ai/rtk/blob/master/src/filters/README.md) for the full TOML DSL reference.

### Trusting custom filters

Because a filter can rewrite what your AI assistant sees, custom filter files are **not applied until you trust them**. An untrusted (or edited) filter file is skipped silently on the command path. You review and manage trust with explicit commands:

```bash
rtk trust      # shows each filter and asks to confirm (--yes to skip the prompt)
rtk untrust    # revokes trust
```

`rtk init` also detects existing filters and lets you enable them — interactively, or non-interactively with `--trust-filters` / `--no-trust-filters`. Trust is tied to the file's contents (SHA-256), so editing a trusted file requires re-running `rtk trust`.

> **Upgrading:** earlier versions applied `~/.config/rtk/filters.toml` without trust. After upgrading, the user-global file is gated like project filters — if you already relied on a global filter, run `rtk trust` once to re-enable it.

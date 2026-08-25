// RTK Pi extension — rewrites bash commands to use rtk for token savings.
// Shared with Oh My Pi (OMP) — OMP loads this same file via its legacy-pi-compat layer.
// Requires: rtk >= 0.23.0 in PATH.
//
// This is a thin delegating extension: all rewrite logic lives in `rtk rewrite`,
// which is the single source of truth (src/discover/registry.rs).
// To add or change rewrite rules, edit the Rust registry — not this file.
//
// Exit code contract for `rtk rewrite`:
//   0 + stdout  Rewrite found → mutate command
//   1           No RTK equivalent → pass through unchanged
//   3 + stdout  Rewrite (advisory) → mutate command

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent"
import { isToolCallEventType } from "@earendil-works/pi-coding-agent"

const REWRITE_TIMEOUT_MS = 2_000
const MIN_SUPPORTED_RTK_MINOR = 23

// Parse "X.Y.Z" semver, return [major, minor, patch] or null.
function parseSemver(raw: string): [number, number, number] | null {
  const m = raw.trim().match(/(\d+)\.(\d+)\.(\d+)/)
  if (!m) return null
  return [parseInt(m[1], 10), parseInt(m[2], 10), parseInt(m[3], 10)]
}

// Calls `rtk rewrite`; returns the rewritten command or null (pass through).
async function rewriteCommand(
  pi: ExtensionAPI,
  cmd: string,
  signal?: AbortSignal
): Promise<string | null> {
  const result = await pi.exec("rtk", ["rewrite", cmd], {
    timeout: REWRITE_TIMEOUT_MS,
    signal,
  })
  if (result.killed) return null
  if (result.code !== 0 && result.code !== 3) return null
  return result.stdout.trim() || null
}

// Persistent "RTK disabled" notice for runtimes that expose one. OMP's
// legacy-pi-compat provides ctx.ui.setStatus on session_start; on Pi the
// callback simply never fires (a strict on() implementation is caught below).
// pi.notify is intentionally not used — OMP wipes it on the initial render.
function notifyRtkUnavailable(pi: ExtensionAPI, reason: string) {
  try {
    pi.on("session_start", (_event: unknown, ctx: unknown) => {
      (ctx as { ui?: { setStatus?: (key: string, text: string) => void } })?.ui?.setStatus?.(
        "rtk",
        `RTK disabled: ${reason}`
      )
    })
  } catch {
    // Runtimes without a session_start event: nothing to report.
  }
}

export default async function (pi: ExtensionAPI) {
  // OMP surfaces extension labels in the session UI; Pi has no setLabel (no-op there).
  if (typeof (pi as { setLabel?: (label: string) => void }).setLabel === "function") {
    (pi as { setLabel?: (label: string) => void }).setLabel("RTK")
  }

  // Probe rtk version at load time; disables extension if missing or too old.
  const ver = await pi.exec("rtk", ["--version"], { timeout: REWRITE_TIMEOUT_MS })
  if (ver.code !== 0) {
    notifyRtkUnavailable(pi, "rtk binary not found in PATH")
    console.warn("[rtk] rtk binary not found in PATH — extension disabled")
    return
  }

  // Warn and bail if rtk predates 0.23.0 (when `rtk rewrite` was introduced).
  const parsed = parseSemver(ver.stdout.replace(/^rtk\s+/, ""))
  if (parsed) {
    const [major, minor] = parsed
    if (major === 0 && minor < MIN_SUPPORTED_RTK_MINOR) {
      notifyRtkUnavailable(pi, `rtk ${parsed.join(".")} is too old (need >= 0.23.0)`)
      console.warn(`[rtk] rtk ${ver.stdout.trim()} is too old (need >= 0.23.0) — extension disabled`)
      return
    }
  }

  pi.on("tool_call", async (event, ctx) => {
    try {
      if (!isToolCallEventType("bash", event)) return

      const cmd = event.input.command
      if (typeof cmd !== "string" || cmd.trim() === "") return

      if (cmd.startsWith("rtk ")) return
      if (process.env.RTK_DISABLED === "1") return

      // Delegate to RTK.
      const rewritten = await rewriteCommand(pi, cmd, ctx.signal)
      if (rewritten && rewritten !== cmd) {
        event.input.command = rewritten
      }
    } catch (err) {
      // Fail open: never block execution on an unexpected error.
      console.warn("[rtk] unexpected error in tool_call handler; passing through command", err)
      return
    }
  })
}

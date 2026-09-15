# Experimental Assistant

Open **Assistant** in the top bar. It connects automatically to an installed
Codex CLI and uses the signed-in Codex account. If connection or sign-in is
needed, the panel provides Retry and login controls. rexafs does not store a
separate model API key. Choose the model and reasoning effort in the panel;
availability comes from the connected server.

## Connection troubleshooting

An installed Codex launcher can still fail before connecting. For example,
macOS apps opened from Finder do not necessarily inherit Terminal's `PATH`.
An npm/Bun launcher using `#!/usr/bin/env node` then needs Node.js on the app's
search path, even if `codex --version` succeeds in Terminal. In rexafs 0.2.6,
the resulting startup error can appear only as **Codex disconnected**.

For 0.2.6, save your project and quit rexafs, then start it from a Terminal
where `codex --version` works:

```sh
/Applications/rexafs.app/Contents/MacOS/rexafs
```

This uses Terminal's environment for that launch; it does not change Finder's
environment. `REXAFS_CODEX` can select an explicit Codex executable when needed.
Connection initialization does not send the draft message or analysis context;
Send shares the context described below. Codex owns authentication through its
[app-server protocol](https://learn.chatgpt.com/docs/app-server).

**Source-checkout fix, not included in 0.2.6:** the
[launcher](../crates/rexafs-gui/src/codex_client.rs) preserves absolute inherited
search-path entries and adds common Bun, npm, Homebrew, Nix, Volta and mise
locations for the child process. It does not read shell startup files or change
the parent environment. On macOS, it also checks the Codex app's bundled CLI
when no standalone launcher is found. Explicit executable overrides retain
priority. Relative/empty path entries are excluded so the temporary assistant
workspace is not searched for executables.

Connection failures show the final nonempty stderr line, limited to 300
characters; only the last 4 KiB is retained in memory. This diagnostic is not
written into a saved project. The launcher change does not alter authentication,
Review/Edit analysis permissions or Extended access.

## Compact composer (unreleased)

The source checkout places the message above a compact footer with **Model**,
**Reasoning**, and **Access** menus. The footer wraps in a narrow docked panel;
the same composer is used in a separate Assistant window. The rounded arrow
sends the message and becomes Stop while a response is running.

The model button shows the resolved model name. Its menu retains **Automatic**;
hover the button to see whether the selection is automatic. The reasoning button
shows the effective level, while its menu distinguishes **Model default** from an
explicit choice. Model changes retain the existing supported-level fallback.
Use arrow keys to move through a menu, Enter or Space to choose, and Escape to
close it. Long model and reasoning lists scroll.

**Access** contains **Review** and **Edit analysis** with descriptions of their
scope. **Workspace commands** is the existing Extended access switch: it starts
off and does not change the selected analysis mode. A small amber dot on the
Access button indicates that workspace commands are enabled. Command approvals
and the assistant sandbox still apply. **Plot images** and **Web search** are
in **Assistant settings** (the gear button).

Opening **Parameters** or **Groups**, by its toolbar button or keyboard
shortcut, keeps that panel visible beside the Assistant. When space is tight,
the older panel closes first and the Assistant narrows temporarily. Its preferred
width returns when space is available. On small windows, the plot can be narrower
than the usual 360 px target so the requested panel remains usable.

These presentation changes are not included in the 0.2.6 download. The
[composer implementation](../crates/rexafs-gui/src/app/shell/assistant_composer.rs)
uses the existing preference persistence and permission checks.

## Workspace and conversations

The Assistant opens at the right of the analysis. Drag its left border to resize
between 320 and 640 px. **Pop out** moves the same conversation to a separate
window; **Dock**, or closing that window, brings it back. Closing the docked panel
keeps its connection and transcript. **Hide side panels / Restore side panels**
controls Groups and the inspector, which can also collapse automatically to
leave room for plots. Width and preferred host are computer settings.

- **Review** permits inspection and navigation. **Edit analysis** enables the
  supported analysis edits for the current turn. App-authored receipts describe
  what changed and provide View/Undo when those operations are still valid.
- **Enter** sends; **Shift+Enter** inserts a newline. **Stop** interrupts the
  assistant turn. A calculation already running in the analysis engine can
  finish independently.
- Thinking is folded initially. Tool activity, processing progress, permission
  decisions and completed answers remain in the transcript. **Copy conversation**
  includes the transcript text.
- **Web search** can be toggled. Structure retrieval validates destinations and
  the returned structure before importing it. **Extended access** starts off;
  turning it on requires session consent, and command approvals remain explicit.
  It does not silently enable arbitrary analysis edits.

**Conversations** lists the project's saved conversations, their relative update
time and turn count. **New** starts a fresh conversation on the next Send. Choose
a saved entry to read it, then **Resume** to continue. A connected client first
tries the stored server thread. An unavailable thread or unsupported resume
method falls back to a new thread with the last ten entries as labelled previous
context. The transcript states which route was used. Resuming applies the current
session's access policy.

The fallback keeps at most 2000 characters from each of those ten entries, so a
long previous calculation may need its relevant settings stated again. This is
historical conversation context, not a replay of its commands or permissions.
The saved transcript remains available to read in the project.

Project Save keeps the newest five completed conversations by default. Set
**Conversations kept per project** in the picker to another count, or zero to
disable saving. An asterisk in the project label indicates unsaved conversation
changes. A turn still running is not serialized; the latest completed snapshot
is retained. Restored thinking starts folded, and saved receipts contain no live
approval or undo tokens. Older projects start with no conversation history.

Sending shares the current analysis context and enabled plots through the
configured Codex account. Context includes spectrum names, source paths and
bounded source comments, requested processing settings, model inputs, fit
history, additional analyses and the action journal. Imported comments and
previous-conversation content are labelled as data. Verify the chosen phase,
paths, ranges and scientific interpretation before relying on a fit.

The Assistant follows the app's Data → Normalize → Background → Transform →
Structure → Calculate → Paths → Model → Results workflow. Each assigned spectrum's
current processing must be inspected before an Assistant fit can run. Plot access
lets the model assess the result; it does not certify scientific quality.

Assistant processing edits target the current spectrum and are validated by
running the proposed settings before applying them. Column mappings are excluded
from that processing-edit operation; review import interpretation in the Data
tools. Pending access requests expire after five minutes, and stopping the turn
invalidates unapproved requests. A saved conversation does not retain a usable
approval or undo token. See the [processing tool
contract](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/codex_client.rs#L547)
and [access request lifetime](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app/shell/assistant.rs#L2127).

Protocol: [Codex App Server lifecycle and thread resume](https://learn.chatgpt.com/docs/app-server).
`codex_client.rs` owns transport and protocol parsing; `app/shell/assistant.rs`
and its history module own conversation behavior; `assistant_actions.rs` applies
semantic actions; `assistant_workflow.md` contains the workflow instructions.

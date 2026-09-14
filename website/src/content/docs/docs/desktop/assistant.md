---
title: "Optional analysis assistant"
description: "Use the assistant with explicit access controls and project context."
audience: user
---

Open **Assistant** in the top bar to connect to an installed Codex CLI using
your signed-in account. Use the panel's Retry or login controls if needed.
rexafs stores no separate model API key. Choose a model and reasoning effort
from those supplied by the connected server.

Drag the panel's left border to resize it between 320 and 640 px. **Pop out**
moves the conversation to a separate window; **Dock**, or closing that window,
returns it. Closing the docked panel keeps the connection and transcript.
**Hide side panels / Restore side panels** controls Groups and the inspector;
they can also collapse automatically to leave room for plots. Panel width and
window preference are computer settings.

## If Codex is installed but disconnected

On macOS, rexafs 0.2.6 opened from Finder can find an npm/Bun Codex launcher
but fail to find the Node.js runtime it needs. To work around this, save your
project and quit rexafs, then open it from a Terminal where `codex --version`
works:

```sh
/Applications/rexafs.app/Contents/MacOS/rexafs
```

This launch inherits Terminal's executable search path. It does not reinstall
Codex or change your account. The development source adds runtime discovery and
more useful startup errors; that fix is not part of the 0.2.6 download.

## Controls and access

- **Review** permits inspection and navigation. **Edit analysis** enables the
  supported analysis edits for the current turn. App-authored receipts describe
  what changed and provide View/Undo when those operations are still valid.
- **Enter** sends; **Shift+Enter** inserts a newline. **Stop** interrupts the
  assistant turn. A calculation already running in the analysis engine can
  finish independently.
- Thinking starts folded. Tool activity, processing progress, permission decisions
  and answers remain in the transcript. **Copy conversation** copies its text.
- **Web search** can be toggled. Structure retrieval validates destinations and
  the returned structure before importing it. **Extended access** starts off;
  turning it on requires session consent, and command approvals remain explicit.
  It does not silently enable arbitrary analysis edits.

## Saved conversations

**Conversations** lists saved conversations with update times and turn counts.
**New** starts a fresh conversation on the next Send. Select a saved entry to
read it, then **Resume** to continue under the current session's access policy.
The client first tries the stored server thread. If unavailable or unsupported,
it opens a new thread with the last ten entries as labelled previous context.
The transcript states which route was used.

Fallback context keeps at most 2000 characters per entry; restate relevant
settings from longer calculations. It does not replay commands or permissions.
The saved transcript remains readable in the project.

Project Save keeps the newest five completed conversations by default. Change
**Conversations kept per project** in the picker, or set zero to disable saving.
An asterisk in the project label marks unsaved conversation changes. Running
turns are not serialized; the latest completed snapshot is retained. Restored
thinking starts folded, and saved receipts have no live approval or undo tokens.
Older projects start with no conversation history.

## Shared context and analysis edits

Sending shares analysis context and enabled plots through your Codex account:
spectrum names, source paths, bounded source comments, requested processing
settings, model inputs, fit history, additional analyses and the action journal.
Imported comments and previous conversations are labelled as data. Verify the
phase, paths, ranges and scientific interpretation before relying on a fit.

The Assistant follows Data → Normalize → Background → Transform → Structure →
Calculate → Paths → Model → Results. It must inspect each assigned spectrum's
current processing before fitting. Plot access supports review but cannot
certify scientific quality.

Processing edits target the current spectrum. Proposed settings are run and
validated before applying them. Column mappings are excluded; review import
interpretation in Data. Access requests expire after five minutes; stopping a
turn invalidates unapproved requests. See the [processing tool
contract](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/codex_client.rs#L547)
and [access request lifetime](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app/shell/assistant.rs#L2127).

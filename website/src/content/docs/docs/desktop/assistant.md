---
title: "Optional analysis assistant"
description: "Choose a model, review spectra and edit analysis with explicit access controls."
audience: user
---

Open **Assistant** in the top bar to connect to your installed Codex CLI and
signed-in account. Use **Retry** or the login controls if needed. rexafs stores
no separate model API key. The Assistant remains experimental; review scientific
choices and results before relying on them.

[![rexafs 0.2.7 showing the Assistant composer beside the Cu spectrum and Parameters panel](/screenshots/0.2.7/assistant-layout.jpg)](/screenshots/0.2.7/assistant-layout.jpg)

Captured from the signed 0.2.7 Mac app with the bundled Cu example. The available
models and reasoning levels depend on the connected Codex installation.
Select an image to view it at full size.

## Write and send

Write your request in the message field. The footer has three menus:

- **Model** shows the effective model. Its menu retains **Automatic**, which
  follows Codex's default; hover the button to check whether it is automatic.
- **Reasoning** shows the effective level. Choose **Model default** to follow
  the selected model, or select one of its supported levels.
- **Access** selects what the Assistant may do in the analysis.

[![Model menu in the rexafs 0.2.7 Assistant](/screenshots/0.2.7/assistant-models.jpg)](/screenshots/0.2.7/assistant-models.jpg)

Use arrow keys to browse a menu, Enter or Space to choose, and Escape to close.
**Enter** in the message field sends; **Shift+Enter** inserts a newline. The arrow
button becomes **Stop** during a response. Stopping interrupts the assistant
turn; an analysis calculation already running can finish independently.

## Choose access

[![Access menu in the rexafs 0.2.7 Assistant](/screenshots/0.2.7/assistant-access.jpg)](/screenshots/0.2.7/assistant-access.jpg)

**Review** permits inspection and navigation. **Edit analysis** also permits
supported parameter changes and calculations. App-authored receipts describe
changes and provide View/Undo while those operations remain available.

**Workspace commands** is a separate switch, previously labelled Extended access.
It starts off and requires session consent. Command approvals and the assistant
sandbox still apply. An amber dot on Access shows when the switch is enabled;
it does not change the selected analysis mode.

Open **Assistant settings** with the gear button to choose **Plot images** and
**Web search**, or inspect **Shared context**. Sending shares spectrum names,
source paths, bounded source comments, processing settings, model inputs,
analysis results, journal entries and enabled plots through your Codex account.
Imported comments and previous conversations are labelled as data.

## Keep the workspace visible

Open **Parameters** or **Groups** with the toolbar or their keyboard shortcuts.
The newly opened panel stays visible beside the Assistant. When space is tight,
the older panel closes first, then the Assistant narrows temporarily. Its
preferred width returns when more space is available.

Drag the Assistant's left border to resize it between 320 and 640 px. **Pop out**
moves it to a separate window; **Dock**, or closing that window, returns it.
Closing the docked panel keeps the connection and transcript. Panel width and
window preference are saved on this computer.

## Conversations and results

Thinking starts folded. Tool activity, processing progress, permission decisions
and answers remain in the transcript. **Copy conversation** copies its text.
**Conversations** lists saved conversations; select one to read it, then choose
**Resume**. **New** starts a fresh conversation on the next Send.

Resume first tries the stored Codex thread. If unavailable or unsupported, it
starts a new thread using the last ten entries, each limited to 2000 characters,
as labelled previous context. The transcript identifies which route was used.
Restate important settings from longer calculations; commands and permissions
are not replayed.

Project Save retains the newest five completed conversations by default. Change
**Conversations kept per project**, or set zero to disable saving. Running turns
are not serialized. Restored receipts have no live approval or undo tokens, and
older projects start with no conversation history.

The Assistant inspects each assigned spectrum's processing before fitting.
Processing edits target the current spectrum and are calculated and validated
before being applied. Review column mappings in Data; the Assistant does not
edit import interpretation. Access requests expire after five minutes, and Stop
invalidates unapproved requests. See the [processing tool contract](https://github.com/Ameyanagi/rexafs/blob/v0.2.7/crates/rexafs-gui/src/codex_client.rs)
and [request handling](https://github.com/Ameyanagi/rexafs/blob/v0.2.7/crates/rexafs-gui/src/app/shell/assistant.rs).

## If Codex does not connect

Version 0.2.7 discovers common runtime installations needed by npm/Bun Codex
launchers when macOS opens rexafs from Finder. A failed connection shows a short
startup diagnostic. Check that the installed Codex CLI works, then choose **Retry**.

<details>
<summary>Workaround for the older 0.2.6 Mac download</summary>

Save your project and quit rexafs, then launch it from a Terminal where
`codex --version` works:

```sh
/Applications/rexafs.app/Contents/MacOS/rexafs
```

This launch inherits Terminal's executable search path. It does not change your
Codex account. Version 0.2.7 includes the runtime-discovery correction.

</details>

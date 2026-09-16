---
title: "Optional analysis assistant"
description: "Choose a model, review spectra and edit analysis with explicit access controls."
audience: user
---

Open **Assistant** in the top bar to connect to your installed Codex CLI and
signed-in account. Use **Retry** or the login controls if needed. rexafs stores
no separate model API key. The Assistant remains experimental; review scientific
choices and results before relying on them.

[![rexafs 0.2.9 showing the Assistant composer beside the Cu spectrum and Parameters panel](/screenshots/0.2.9/assistant-layout.jpg)](/screenshots/0.2.9/assistant-layout.jpg)

Captured through computer use from the signed 0.2.9 Mac app with the prepared
Cu foil reference. See [screenshot provenance](/licenses/#documentation-screenshots).
No Assistant message was sent for these three captures. The available
models and reasoning levels depend on the connected Codex installation.
Select an image to view it at full size.

## Write and send

Write your request in the message field. The footer has three menus:

- **Model** shows the effective model. Its menu retains **Automatic**, which
  follows Codex's default; hover the button to check whether it is automatic.
- **Reasoning** shows the effective level. Choose **Model default** to follow
  the selected model, or select one of its supported levels.
- **Access** selects what the Assistant may do in the analysis.

[![Model menu in the rexafs 0.2.9 Assistant](/screenshots/0.2.9/assistant-model.jpg)](/screenshots/0.2.9/assistant-model.jpg)

Use arrow keys to browse a menu, Enter or Space to choose, and Escape to close.
**Enter** in the message field sends; **Shift+Enter** inserts a newline. The arrow
button becomes **Stop** during a response. Stopping interrupts the assistant
turn; an analysis calculation already running can finish independently.

## Choose access

[![Access menu in the rexafs 0.2.9 Assistant](/screenshots/0.2.9/assistant-access.jpg)](/screenshots/0.2.9/assistant-access.jpg)

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

<a id="unreleased-context-retrieval-and-analysis"></a>

## Context retrieval and analysis in 0.2.9

Version **0.2.9** adds context retrieval.
New messages send a short overview of the current
group and up to 50 available groups. The Assistant retrieves relevant source
headers, import mappings, processing settings, fit models or historical results
when needed. Headers and long group lists are paginated; plots are requested on
demand when Plot images is enabled. Original measurement bytes, full raw tables
and unrelated saved conversations stay out of the prompt. This prevents large
import archives from exceeding the Assistant's input limit. Project saves and
publication exports still retain their original data. These changes were introduced after 0.2.8.

The Assistant can select imported groups by
identity, run independent EXAFS fits in sequence, and configure a joint fit with
explicit shared and per-spectrum variables. Each independent fit creates a
separate History entry. Joint fitting produces one result for the assigned
spectra. Existing paths, ranges, expressions and local values are retained.

It can also run **Linear combination fit** or **Principal components** in Data.
Specify the target, references and energy interval. The interval is relative to
the target's edge energy for LCF and the first training spectrum's edge energy
for PCA; the GUI defaults to −20 to +30 eV. Use compatible absorption edges and
a measured interval shared by every spectrum. LCF defaults to nonnegative
weights summing to one and no fitted energy shifts. PCA uses the GUI's
uncentered convention and requires a retained component count: its percentages
describe squared signal, not chemical concentrations. Results and residual
plots appear in the main workspace. The Assistant checks current processing
for every requested operand before running these operations. See the
[analysis guide](/docs/science/analysis/) for assumptions and interpretation.

[![Unreleased rexafs Assistant completing LCF of a synthetic mixture, with its weights and residual visible](/screenshots/next/assistant-lcf.jpg)](/screenshots/next/assistant-lcf.jpg)

Example request: “Fit Synthetic mixture 1 using components A, B and C in
normalized μ, from −20 to +80 eV relative to E₀. Keep the processing settings,
constrain weights to sum to one, and inspect the fit and residual.” This
controlled example recovers the known 20%, 30% and 50% weights.

[![Unreleased rexafs Assistant comparing two- and three-component PCA reconstructions of synthetic spectra](/screenshots/next/assistant-pca.jpg)](/screenshots/next/assistant-pca.jpg)

Example request: “Train PCA on mixtures 2–6 and reconstruct mixture 1 over the
same interval. Compare two and three retained components, then show the
three-component residual.” Here three components reproduce the known synthetic
signal. This example does not establish a component count for experimental data.

Both captures show an unreleased macOS source build tested through computer use
on 15 September 2026. The inputs are generated mathematical signals, not measured
Cu oxidation-state standards. See [screenshot provenance](/licenses/).

Changing an input marks its earlier analysis result as stale in the Assistant's
state; stale results are not supplied as current analysis plots. Use
**Publish → Analysis folder → Export** to retain computed arrays and the report.
Version 0.2.9 also saves completed LCF, batch LCF, PCA and MCR results in `.rxs`
projects alongside the input groups and retained conversations.

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
invalidates unapproved requests. See the [processing tool contract](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/crates/rexafs-gui/src/codex_client.rs)
and [request handling](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/crates/rexafs-gui/src/app/shell/assistant.rs).

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

# Acceptance standards research

Research date: 2026-08-27

This document records standards and host expectations relevant to Ley's public landing page, editor, Codex/Claude Code bundles, and local MCP server. It is an acceptance checklist, not a conformance claim. The external sources below are first-party or official sources and were checked on the research date.

The labels in this document have deliberately different meanings:

- **Normative / MUST:** a requirement stated by a specification or Recommendation. A selected subset of WCAG 2.2 is included because it maps directly to the observed landing-page and editor risks; this is not a complete WCAG audit.
- **Host expectation:** a current packaging or runtime contract stated by the Codex or Claude Code documentation. These contracts are version-sensitive and are not WCAG or MCP normative requirements.
- **Recommendation / SHOULD:** guidance stated as a recommendation by the cited source. It becomes a Ley acceptance requirement only when adopted by the product decision.
- **Verify:** evidence to collect during acceptance. A successful automated check does not replace manual checks of keyboard behavior, focus, visual presentation, or assistive-technology output.

## Repository acceptance anchors

- Public landing page: [`src/website/LandingPage.tsx`](../../src/website/LandingPage.tsx), including the page header/navigation, one primary content region, headings, links, decorative icons, and the informative graph SVG.
- Editor: [`src/features/editor/NoteWorkspace.tsx`](../../src/features/editor/NoteWorkspace.tsx), [`src/features/editor/CodeMirrorEditor.tsx`](../../src/features/editor/CodeMirrorEditor.tsx), and [`src/features/editor/lib/mount.ts`](../../src/features/editor/lib/mount.ts), including the title/file controls, mode controls, formatting toolbar, CodeMirror surface, completion popup, alerts, and status messages.
- Codex package: `integrations/codex/plugins/ley-memory/`, with `.codex-plugin/plugin.json`, `skills/`, `.mcp.json`, and `hooks/hooks.json`; its repo marketplace is `integrations/codex/.agents/plugins/marketplace.json`.
- Claude Code package: `integrations/claude-code/ley-memory/`, with `.claude-plugin/plugin.json`, `skills/`, `.mcp.json`, and `hooks/hooks.json`.
- MCP smoke-test context: [`docs/agent-memory/mcp.md`](../agent-memory/mcp.md) already documents an official Inspector CLI path for the local `ley mcp` server.

## 1. PWA manifest, installability, and icons

### Normative/current Chromium-facing baseline

| Requirement | Acceptance evidence | Source |
| --- | --- | --- |
| **MUST** link the web app manifest from the app HTML. If an app has more than one page, every page must reference it. | Inspect the built landing and app entry documents and confirm a `<link rel="manifest" href="…">` points to the deployed manifest. | [MDN: Making PWAs installable](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable) — modified 2025-11-30; checked 2026-08-27. [Chromium: Web Apps concepts](https://chromium.googlesource.com/chromium/src/+/main/docs/webapps/concepts.md) — current `main`; checked 2026-08-27. |
| For Chromium promotion, the manifest **MUST** include `name` or `short_name`; `icons` containing 192px and 512px icons; `start_url`; `display` and/or `display_override`; and `prefer_related_applications` set to `false` or omitted. | Fetch the deployed manifest, parse it, assert each field, and verify the declared 192px/512px icon files are reachable and have those dimensions. | [MDN: Making PWAs installable](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable) — modified 2025-11-30; checked 2026-08-27. |
| A promoted PWA **MUST** be served over `https`, or locally from `localhost`/`127.0.0.1` (with or without a port). | Test the public origin over HTTPS and use only localhost/loopback for local acceptance. A `file://` launch is not evidence of installability. | [MDN: Making PWAs installable](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable) — modified 2025-11-30; checked 2026-08-27. |
| A service worker is **not a current installability requirement**. | Do not fail manifest installability solely because a service worker is absent; test offline behavior separately if the product promises it. | [MDN: Making PWAs installable](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable) — modified 2025-11-30; checked 2026-08-27. |

Chromium distinguishes “installable” from “promotable”: its current concepts documentation describes installation eligibility separately from whether Chrome proactively promotes installation. Chrome 124 also added universal install for pages that do not meet PWA criteria. Therefore, a manual “Install this site as an app” result is **not sufficient evidence** that Ley's manifest meets the PWA promotion baseline. Test both manifest fields and the browser's promotion/install flow. Sources: [Chromium: Web Apps concepts](https://chromium.googlesource.com/chromium/src/+/main/docs/webapps/concepts.md) — checked 2026-08-27; [Chrome 124 release notes](https://developer.chrome.com/release-notes/124) — stable 2024-04-16; checked 2026-08-27.

### Manifest and icon quality recommendations

- **SHOULD** serve a `.webmanifest` with `Content-Type: application/manifest+json`; a `.json` manifest commonly uses `application/json`. Check the response headers as well as the JSON body. Source: [MDN: Web application manifest](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest) — modified 2025-08-08; checked 2026-08-27.
- Each icon object **MUST** have `src`; `src` relative URLs resolve against the manifest URL. `sizes`, `type`, and `purpose` are optional manifest members, but **SHOULD** be declared accurately: exact `WxH` sizes for raster icons, `any` for scalable SVG, and the matching `image/<subtype>` MIME type. Source: [MDN: `icons` manifest member](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/icons) — modified 2025-06-23; checked 2026-08-27.
- **SHOULD** provide purpose-specific icons only when the artwork supports that purpose. `maskable` means the artwork accounts for the operating-system mask and safe zone; `monochrome` and `any` have different meanings. Source: [MDN: `icons` manifest member](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/icons) — modified 2025-06-23; checked 2026-08-27.
- For a maskable icon, **SHOULD** keep important artwork inside the safe zone (a circle 80% of the icon's minimum dimension) and use an opaque background. For general app icons, MDN recommends at least 1024×1024 pixels or scalable artwork and multiple versions for different contexts. These are design recommendations, not Chromium installability gates. Source: [MDN: Define your app icons](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/How_to/Define_app_icons) — modified 2025-06-30; checked 2026-08-27.

### PWA acceptance run

1. Build the production site and inspect every relevant HTML entry for the manifest link.
2. Fetch the manifest over the actual acceptance origin. Check JSON validity, response type, required Chromium members, `start_url`, and icon URLs/dimensions. Confirm CSP does not prevent icon fetches; MDN documents that icon fetching is governed by the document's `img-src` policy. Source: [MDN: `icons` manifest member](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/icons) — checked 2026-08-27.
3. In a supported Chromium browser, verify the browser's PWA promotion and the installed app's name, icon, launch URL, and display mode. Record browser/platform because installation-promotion support varies by browser and platform.
4. If offline behavior is part of the product promise, run a separate service-worker/offline test; do not conflate it with the manifest installability gate.

## 2. WCAG 2.2 and axe semantics

### Normative WCAG baseline for the landing page and editor

WCAG 2.2 is a W3C Recommendation published 2024-12-12. The criteria below are the most direct fit for the observed semantic, editor, popup, focus, and visual issues.

| WCAG 2.2 criterion | Normative acceptance requirement | Ley-specific evidence |
| --- | --- | --- |
| **1.1.1 Non-text Content (A)** | Informative non-text content needs an equivalent text alternative; purely decorative content must be implemented so assistive technology can ignore it. | The landing graph SVG needs an accessible name if informative; decorative SVGs/icons must not create redundant announcements. Check editor icons and any image-like controls. |
| **1.3.1 Info and Relationships (A)** | Information, structure, and relationships conveyed through presentation must be programmatically determinable or available in text. | Check native header/nav/main/footer/section/heading structure, form-label associations, toolbar grouping, mode state, completion ownership, and alert/status relationships. |
| **2.1.1 Keyboard (A)** | All content functionality must be operable through a keyboard interface, without timing-dependent keystrokes except the path-dependent-function exception. | Tab through the landing page and every editor action; operate title/file controls, modes, formatting, search, completion, alerts, and dialogs without a pointer. |
| **2.1.2 No Keyboard Trap (A)** | Focus that enters a component must be movable away with the keyboard; any nonstandard exit method must be communicated. | Check CodeMirror, completion popup, in-note search, modal/dialog flows, and any focus-managed editor overlays. |
| **2.4.3 Focus Order (A)** | Sequential focus order must preserve meaning and operability where sequence affects operation. | Check the public navigation order and editor order, including toolbars, title/file actions, editor, popup, and dismissal controls. |
| **2.4.4 Link Purpose (In Context) (A)** | The purpose of each link must be determinable from link text alone or its programmatic context. | Landing-page CTA and navigation links should remain meaningful when read without surrounding visual styling. |
| **2.4.6 Headings and Labels (AA)** | Headings and labels must describe topic or purpose. | Check the landing page's `h1`/section headings, title input, file input, icon-only buttons, mode controls, and editor overlays. |
| **2.4.7 Focus Visible (AA)** | Keyboard operation must have a mode in which the focus indicator is visible. | Verify focus rings/indicators on landing links, editor buttons, title/file controls, popup options, modal controls, and CodeMirror. |
| **2.4.11 Focus Not Obscured (Minimum) (AA)** | A focused component must not be entirely hidden by author-created content. | At desktop, 320px-wide, and short-height viewports, ensure sticky chrome, popups, menus, and dialogs do not fully cover the focused control. |
| **2.5.3 Label in Name (A)** | Where a control has visible label text, its accessible name must contain that visible text. | Check buttons that show a text label at larger breakpoints but become icon-only at small widths, and ensure the accessible name remains consistent with the visible label. |
| **4.1.2 Name, Role, Value (A)** | Every UI component, including scripted components, must expose a programmatically determinable name and role; user-settable states/properties/values must be programmatically settable and changes available to user agents. | Inspect all buttons/inputs/links, `aria-pressed` or equivalent mode/bookmark state, CodeMirror/editor semantics, listbox/option selection, and dialog state. |
| **4.1.3 Status Messages (AA)** | Status messages must be programmatically determinable through role or properties without moving focus to the message. | Check save/sync/attachment/status announcements and ensure normal status updates do not steal focus. Distinguish status from an error/alert requiring attention. |
| **1.4.1 Use of Color (A)** | Color must not be the only visual means of conveying information, action, response, or distinction. | Error, selected, bookmarked, missing-file, and link states need text, shape, state, or another non-color cue. |
| **1.4.3 Contrast (Minimum) (AA)** | Normal text is at least 4.5:1; large text is at least 3:1, subject to the criterion's exceptions. | Measure landing/editor text, muted labels, warnings, links, code text, and focus/selected states in the actual themes. |
| **1.4.11 Non-text Contrast (AA)** | Visual information needed to identify controls and states, and essential graphical objects, has at least 3:1 contrast against adjacent colors. | Measure icon-only controls, borders needed to identify inputs, focus indicators, selected mode/bookmark states, and meaningful graph lines. |

Sources: [WCAG 2.2 Recommendation](https://www.w3.org/TR/WCAG22/) — published 2024-12-12; checked 2026-08-27. Direct criteria: [1.1.1](https://www.w3.org/TR/WCAG22/#non-text-content), [1.3.1](https://www.w3.org/TR/WCAG22/#info-and-relationships), [2.1.1](https://www.w3.org/TR/WCAG22/#keyboard), [2.1.2](https://www.w3.org/TR/WCAG22/#no-keyboard-trap), [2.4.3](https://www.w3.org/TR/WCAG22/#focus-order), [2.4.4](https://www.w3.org/TR/WCAG22/#link-purpose-in-context), [2.4.6](https://www.w3.org/TR/WCAG22/#headings-and-labels), [2.4.7](https://www.w3.org/TR/WCAG22/#focus-visible), [2.4.11](https://www.w3.org/TR/WCAG22/#focus-not-obscured-minimum), [2.5.3](https://www.w3.org/TR/WCAG22/#label-in-name), [4.1.2](https://www.w3.org/TR/WCAG22/#name-role-value), [4.1.3](https://www.w3.org/TR/WCAG22/#status-messages), [1.4.1](https://www.w3.org/TR/WCAG22/#use-of-color), [1.4.3](https://www.w3.org/TR/WCAG22/#contrast-minimum), and [1.4.11](https://www.w3.org/TR/WCAG22/#non-text-contrast). WCAG's conformance section explains that the main success-criteria content is normative and that conformance requires the applicable criteria for the selected level, not just the checks listed here.

### axe-core: automated signals, not a conformance substitute

axe-core's official rule catalogue separates WCAG A/AA/AAA rules from best-practice rules. Its official README says axe finds about 57% of WCAG issues automatically and returns `incomplete` results where manual review is needed. Therefore, “zero axe violations” is a useful gate for rendered states, but cannot prove keyboard behavior, focus visibility, popup interaction, or full WCAG conformance.

| axe signal | Use in Ley acceptance | Classification |
| --- | --- | --- |
| [`button-name`](https://dequeuniversity.com/rules/axe/4.12/button-name) | Every icon-only or visually collapsed button must have a discernible accessible name that describes its action. | axe rule mapped to WCAG 2.2 A; automated check. |
| [`label`](https://dequeuniversity.com/rules/axe/4.12/label) | Title, file, search, property, and other form controls must have programmatic labels. | axe rule mapped to WCAG; automated check. |
| `heading-order` | Use as a signal for heading hierarchy and likely skipped-level mistakes on the landing page and editor shell. | axe best practice, not a standalone WCAG success criterion. Source: [axe-core rule catalogue](https://github.com/dequelabs/axe-core/blob/develop/doc/rule-descriptions.md) — checked 2026-08-27. |
| [`landmark-one-main`](https://dequeuniversity.com/rules/axe/4.12/landmark-one-main) | Confirm one primary `main` landmark and that page content is meaningfully contained in landmarks. | axe best practice; verify the actual page structure manually. |
| [`landmark-unique`](https://dequeuniversity.com/rules/axe/4.12/landmark-unique) | Give repeated landmarks distinguishable accessible names where needed. | axe best practice; duplicate landmarks are not automatically a WCAG failure in every context. |
| `aria-*` validity rules and duplicate-ID rules | Catch invalid ARIA attributes, missing required ARIA properties, and ID references that break accessible relationships. | Automated signal; interpret against the actual widget semantics and WCAG 4.1.2. Source: [axe-core rule catalogue](https://github.com/dequelabs/axe-core/blob/develop/doc/rule-descriptions.md) — checked 2026-08-27. |

Run axe after each important state is rendered, not only on initial load. This matters for the editor's completion popup, search panel, warnings, dialogs, and responsive controls. Source: [axe-core API guidance](https://github.com/dequelabs/axe-core/blob/develop/doc/API.md) — checked 2026-08-27.

### WAI-ARIA Authoring Practices recommendations for custom interactions

The APG patterns are implementation guidance, not additional WCAG success criteria. Use native HTML controls where they express the interaction; use the matching APG pattern when a custom widget is necessary.

- **Toolbar:** If the formatting actions are exposed as a composite toolbar, give the toolbar an accessible label and use the APG keyboard model for moving through its controls. Source: [WAI-ARIA APG toolbar pattern](https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/) — checked 2026-08-27.
- **Completion popup:** If the editor exposes a combobox/listbox interaction, preserve the text-editing behavior, popup ownership, selected option state, and expected Down/Escape/Enter/Tab behavior. Source: [WAI-ARIA APG combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/) — checked 2026-08-27.
- **Modal dialogs:** A modal needs `role="dialog"`, an accessible label, `aria-modal="true"` when it is modal, a contained Tab/Shift+Tab sequence, and a reliable close action. Verify focus placement on open and restoration to the invoking control on close. Source: [WAI-ARIA APG dialog (modal) pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) — checked 2026-08-27.

### Repository-specific semantic checks

Landing page:

- Verify one page-level `main` landmark, one page-level `h1`, headings that describe their sections, and meaningful link text/context. Treat a clean `heading-order` result as a best-practice signal, not as proof of WCAG conformance.
- Verify native landmark structure and distinguish repeated landmarks if there are multiple instances. The graph SVG is informative and should expose its name; purely decorative icons and background graphics should be ignored by assistive technology.
- Run axe on the fully rendered page, then manually tab through every CTA/navigation link and inspect focus visibility, 320px reflow, text contrast, non-text contrast, and color-independent state cues.

Editor:

- Verify the note-title input, hidden/file input flow, search controls, property inputs, and every icon-only button have an accessible name. The accessible name of a responsive control must not become empty when its visible text is hidden.
- Verify the formatting controls are either a clearly labelled toolbar or ordinary labelled buttons with a sensible focus order. Mode controls expose their current state; use tab semantics only if they actually implement a tabbed interface.
- Verify CodeMirror's editable surface has an understandable name/role and that all editor functions remain keyboard-operable. Exercise completion with typing, arrows, Enter, Tab, and Escape; inspect the live accessibility tree for listbox/option ownership and selected state.
- Verify missing-file and external-change messages are exposed as alerts when immediate attention is required, while ordinary sync/attachment/status updates use a status mechanism without forced focus. Verify the message is not conveyed by color alone.
- Run axe both before and after opening search, completion, warning, and modal states, then perform manual focus/keyboard checks. Automated output must include and resolve `incomplete` items rather than ignoring them.

## 3. Codex and Claude Code plugin/hook packaging

The host scope here is intentionally limited to Codex and Claude Code.

### Codex

Current official Codex plugin documentation states the following host expectations:

- A plugin has a `.codex-plugin/plugin.json` manifest. It may also contain `skills/`, `.mcp.json`, lifecycle hooks, and assets. Use a stable kebab-case plugin `name`; skills use `skills/<skill-name>/SKILL.md`. Source: [OpenAI: Package your plugin](https://developers.openai.com/plugins/build/plugins) — checked 2026-08-27.
- The default plugin hook file is `hooks/hooks.json`. A manifest `hooks` entry can override it with a `./`-relative path, array, or inline hook object. Paths must remain within the plugin root. Source: [OpenAI: Package your plugin — plugin hooks](https://developers.openai.com/plugins/build/plugins) — checked 2026-08-27.
- A repo-scoped marketplace is `.agents/plugins/marketplace.json`; each entry uses a `./`-prefixed `source.path` relative to the marketplace root. The catalog can provide `interface`, `policy`, and `category` metadata. Source: [OpenAI: Package your plugin — local marketplace](https://developers.openai.com/plugins/build/plugins) — checked 2026-08-27.
- Codex hooks use an event → matcher-group → handler structure. Command handlers use `type: "command"`, receive one JSON object on stdin, and may return supported output on stdout; `timeout` is in seconds. Relevant lifecycle events include `SessionStart`, `UserPromptSubmit`, and `Stop`. Source: [OpenAI: Codex hooks](https://developers.openai.com/codex/hooks) (redirects to the official ChatGPT Learn hooks page) — checked 2026-08-27.
- Non-managed hooks, including plugin-bundled hooks, require review/trust of the exact current definition before Codex runs them; changed definitions are skipped until trusted. Acceptance must include the host trust step or explicitly record that hooks were not executed because they remain untrusted. Source: [OpenAI: Codex hooks — review and trust](https://learn.chatgpt.com/docs/hooks) — checked 2026-08-27.

Repository comparison: the Codex bundle follows the documented default layout: `.codex-plugin/plugin.json` declares `skills` and `.mcp.json`, `hooks/hooks.json` exists, and `integrations/codex/.agents/plugins/marketplace.json` points to `./plugins/ley-memory`. That file-layout comparison does not prove that a particular Codex build loaded the plugin; verify with the host's plugin view, `/hooks`, and a real session. The hook commands also assume the `ley` executable is available on `PATH`.

Codex acceptance evidence:

1. Parse the manifest, marketplace, MCP config, and hooks JSON; assert relative paths resolve inside the intended package.
2. Load the package in the target Codex host, inspect the plugin, review/trust the hook definitions, and exercise `SessionStart`, `UserPromptSubmit`, and `Stop` with a disposable project.
3. Verify hook stdin is valid JSON, the executable returns within its configured timeout, and intended context/status behavior appears in the host. Keep hook output bounded; Codex documents spill behavior for oversized output.

### Claude Code

Current official Claude Code documentation gives these expectations and one important nuance:

- A plugin's components live at the plugin root: `.claude-plugin/plugin.json` is the manifest location, while `skills/`, `agents/`, `hooks/`, and `.mcp.json` are siblings at the root. Do not place those component directories inside `.claude-plugin/`. Source: [Claude Code: Create plugins](https://code.claude.com/docs/en/plugins) — checked 2026-08-27.
- The reference says the manifest is optional at runtime when default component locations are used; if present, it provides identity/metadata and custom paths. For a named, distributed Ley bundle, keeping the manifest is the safer acceptance choice because it makes identity and metadata explicit. Source: [Claude Code: Plugins reference — manifest](https://code.claude.com/docs/en/plugins-reference) — checked 2026-08-27. This is a documentation nuance, not a contradiction to the package layout.
- Default hooks are `hooks/hooks.json`, or hooks may be inline in `plugin.json`; default MCP configuration is `.mcp.json`, or it may be inline. Custom paths are relative to the plugin root. Plugin-internal paths should use `${CLAUDE_PLUGIN_ROOT}`; persistent plugin data should use `${CLAUDE_PLUGIN_DATA}`. Source: [Claude Code: Plugins reference](https://code.claude.com/docs/en/plugins-reference) — checked 2026-08-27.
- Validate with `claude plugin validate ./my-plugin --strict`; the documented validator checks the manifest, `hooks/hooks.json`, and default component frontmatter. Use `claude --debug` when load, hook registration, or MCP initialization needs runtime evidence. Source: [Claude Code: Plugins reference — validation](https://code.claude.com/docs/en/plugins-reference) — checked 2026-08-27.
- If distributed through a Claude marketplace, `.claude-plugin/marketplace.json` lists plugins with `name` and `source`; a relative source such as `./plugins/my-plugin` is resolved from the marketplace root and must not contain `..`. Source: [Claude Code: Plugin marketplaces](https://code.claude.com/docs/en/plugin-marketplaces) — checked 2026-08-27.
- Command hooks receive event JSON on stdin and communicate through exit codes, stdout, and stderr. Exit code 0 means success; for most blocking-capable events, exit code 2 signals blocking. `SessionStart` itself is not a blocking event. Source: [Claude Code: Hooks reference](https://code.claude.com/docs/en/hooks) — checked 2026-08-27.

Repository comparison: the Claude Code bundle has the expected root-level `skills/`, `hooks/hooks.json`, `.mcp.json`, and `.claude-plugin/plugin.json`. Its hook and MCP commands use the host-provided `${CLAUDE_PROJECT_DIR}` and the `ley` executable; acceptance must verify those variables resolve and that `ley` is available in the host process environment. The checked tree contains no Claude marketplace catalog under `integrations/claude-code`, so marketplace distribution is not evidenced by this bundle alone.

Claude Code acceptance evidence:

1. Run the strict validator against the package root and resolve every warning/error, including schema warnings introduced by copied metadata.
2. Load the package from its actual test/cache location, not only from the repository checkout; verify root-relative component paths still resolve after installation/caching.
3. Trigger `SessionStart`, `UserPromptSubmit`, and `Stop` in a disposable project. Confirm stdin JSON, exit behavior, stderr diagnostics, timeout, project-directory resolution, and intended context injection.
4. Start the configured MCP server from `.mcp.json`, then perform the MCP checks in the next section.

## 4. MCP Inspector and stdout protocol verification

### MCP stdio requirements

The current versioned MCP transport page checked on 2026-08-27 is the 2025-11-25 specification page. It defines these normative stdio invariants:

- MCP messages **MUST** be UTF-8 JSON-RPC messages.
- The client launches the server as a subprocess; the server reads JSON-RPC from stdin and writes JSON-RPC to stdout.
- Each message is newline-delimited and **MUST NOT** contain embedded newlines.
- The server **MUST NOT** write anything to stdout that is not a valid MCP message. Logging may go to stderr.
- The client **MUST NOT** write anything to the server's stdin that is not a valid MCP message.

Source: [MCP specification: Transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) — checked 2026-08-27.

The lifecycle handshake is also normative: initialization is the first interaction; the client sends `initialize` with a supported protocol version, capabilities, and client information; the server responds; the client then sends `notifications/initialized`; normal operations follow. Version negotiation and capability negotiation must be honored. Source: [MCP specification: Lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle) — checked 2026-08-27.

### Inspector verification

The official Inspector documentation describes Inspector as a developer tool for testing/debugging MCP servers. It recommends launching a local server, verifying basic connectivity and capability negotiation, iterating after rebuilds, and exercising invalid inputs, missing arguments, concurrent operations, and error responses. Source: [MCP Inspector guide](https://github.com/modelcontextprotocol/docs/blob/main/docs/tools/inspector.mdx) — official `modelcontextprotocol/docs` repository; checked 2026-08-27.

The current Inspector repository ships web, CLI, and TUI entry points from `@modelcontextprotocol/inspector`; the documented CLI entry point is `npx @modelcontextprotocol/inspector --cli`. Source: [MCP Inspector repository](https://github.com/modelcontextprotocol/inspector) — official `modelcontextprotocol` repository; checked 2026-08-27.

For Ley, use the existing repository command shape with an absolute binary and project path:

```bash
npx @modelcontextprotocol/inspector --cli \
  /absolute/path/to/ley mcp /absolute/path/to/project \
  --method tools/list
```

The Inspector check should establish connectivity, complete initialization/capability negotiation, list tools, and call at least one safe read-only tool. Also exercise a representative invalid or missing-argument request and record the structured error. The exact tool name should come from the fixture's advertised `tools/list`; do not make a write-capable call part of a smoke test unless the fixture and consent are explicit.

Inspector is not, by itself, proof of the raw stdout invariant: it is a client that can parse and display server traffic. Add a direct subprocess check that:

1. Launches `ley mcp <absolute-project-path>` with stdout and stderr captured separately.
2. Sends a valid `initialize` request, waits for the matching response, sends `notifications/initialized`, then requests `tools/list`.
3. Parses every non-empty stdout line as JSON and asserts it is a valid JSON-RPC request, notification, or response. Any human-readable log, banner, panic, or unrelated line on stdout is a failure.
4. Allows informational/debug logging only on stderr and records it separately.
5. Repeats against an ordinary/uninitialized disposable project and checks the server's expected zero-tool or unavailable capability response remains protocol-valid rather than emitting explanatory text on stdout.

This split is intentional: Inspector covers client-visible connectivity and tool behavior, while the captured subprocess check covers the transport-level stdout requirement. Both are traceable to the [MCP stdio transport rules](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) and the [official Inspector development workflow](https://github.com/modelcontextprotocol/docs/blob/main/docs/tools/inspector.mdx), checked 2026-08-27.

### Host-configured MCP checks

- Parse both `integrations/codex/plugins/ley-memory/.mcp.json` and `integrations/claude-code/ley-memory/.mcp.json`. Confirm the server key, command, argument order, project-directory argument, and consent flags match the intended host adapter.
- Run the same Inspector and direct-stdout checks through each host package's configured command, not only through a manually typed command. This catches missing `PATH`, unresolved `${CLAUDE_PROJECT_DIR}`, package-cache path, and working-directory assumptions.
- Keep Inspector and MCP version details in the acceptance record. The Inspector CLI and MCP specification are actively versioned; the cited Inspector repository is the current v2 line as checked on 2026-08-27, while the cited protocol page is explicitly versioned `2025-11-25`.

## Uncertainty and source caveats

- The MDN installability guide is current as checked on 2026-08-27 but was last modified 2025-11-30. Chromium's concepts page separates installable from promotable and includes an older promotable section; use the current MDN Chromium field list for this acceptance baseline and do not revive stale service-worker requirements.
- Claude Code's creation guide presents `.claude-plugin/plugin.json` in its starter package, while the current reference says the manifest is optional when default locations are used. This document keeps a manifest as a distribution/identity acceptance choice and does not claim it is required for every runtime-loaded Claude plugin.
- axe-core explicitly reports `incomplete` results and partial automated coverage. No axe run or browser/assistive-technology run was performed in this research-only task; the document defines the evidence still required for parent acceptance.
- MCP Inspector and host hook behavior can change with releases. The sources were available and readable on 2026-08-27; no network-side installation, host login, plugin publication, or external state change was performed.

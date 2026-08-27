# PRD: skhd.zig Support

## 1. Introduction / Overview

Keybinder currently integrates with the original [`koekeishiya/skhd`](https://github.com/koekeishiya/skhd) only — it parses `~/.config/skhd/skhdrc`, manages the `com.koekeishiya.skhd` LaunchAgent via `launchctl`, uses `brew services` for restart, and calls `skhd --reload` for hot-reloads.

[`jackielii/skhd.zig`](https://github.com/jackielii/skhd.zig) is a backwards-compatible Zig port endorsed by the upstream skhd README as the recommended path for new development. It is wire-compatible at the CLI level (`skhd -r` / `--reload`, same `SIGUSR1`, same binary name `skhd`) but differs in:

- **launchd label:** `com.jackielii.skhd` (vs `com.koekeishiya.skhd`)
- **Service registration:** uses `SMAppService` (plist bundled inside `skhd.app`) instead of `~/Library/LaunchAgents/`. `brew services` does **not** work.
- **Service commands:** `skhd --install-service` / `--start-service` / `--stop-service` / `--restart-service` / `--status` / `--uninstall-service`
- **Homebrew tap:** `brew install jackielii/tap/skhd-zig` (formula `skhd-zig`, binary still `skhd`)
- **Config lookup adds XDG support:** `$XDG_CONFIG_HOME/skhd/skhdrc` is checked before `~/.config/skhd/skhdrc`
- **Logs:** `~/Library/Logs/skhd.log` (vs `/tmp/skhd_$USER.log`)
- **Syntax extensions:** `.alias`, `.define` (process groups + command templates), `.path`, `.device`, `.remap` (1:1 and tap-hold blocks), `mouse1`–`mouse5`, `backtick` keyword, `@group` proc-map references, mode activation with inline command (`; mode : cmd`)

This feature adds first-class support for skhd.zig as a selectable variant alongside the original skhd, with auto-detection, settings UI, status indicator, install guidance, and parser updates for the new syntax.

## 2. Goals

- Allow Keybinder to operate correctly when only skhd.zig is installed (no original skhd).
- Allow Keybinder to operate correctly when both are installed, with the user picking the active variant.
- Add a settings preference: **Variant = `auto` | `skhd` | `skhd-zig`** (default `auto`).
- Auto-detect the installed/running variant by inspecting PATH, launchd labels, and Homebrew formulae.
- Show the active variant in the service status UI (badge/label).
- Use the correct service-management commands per variant (no `brew services` for skhd.zig).
- Parse the additional skhd.zig directives without errors so users can edit existing skhd.zig configs in Keybinder.
- Provide in-app install guidance (Homebrew tap copy-button + repo link) when the chosen variant is missing.
- Maintain backwards compatibility: existing original-skhd users see no behavior change unless they opt in.

## 3. User Stories

### US-001: Add `Variant` enum and detection service in Rust

**Description:** As a developer, I need a typed representation of the active skhd variant and a function that detects which one is installed/active so other commands can branch behavior.

**Acceptance Criteria:**

- [ ] Add `SkhdVariant { Original, Zig }` enum in `src-tauri/src/models/` (mirrored in `src/types.ts`).
- [ ] Add `services/variant_detector.rs` with `detect_variant() -> DetectedVariant { variant, binary_path, plist_label, source }` where `source` is one of `Running`, `Plist`, `Homebrew`, `Path`, `AppBundle`, `None`.
- [ ] Detection order: (1) running launchd job (`launchctl list` for `com.jackielii.skhd` then `com.koekeishiya.skhd`), (2) installed Homebrew formula (`brew list skhd-zig` / `brew list skhd`), (3) `which skhd` + binary fingerprint (run `skhd --version`; skhd.zig output contains `zig` or `skhd.zig`), (4) `.app` bundle fallback — check `/Applications/skhd.app/Contents/MacOS/skhd` for skhd.zig users who installed via drag-to-Applications, (5) None.
- [ ] Unit tests in `src-tauri/tests/variant_detector_test.rs` mock command outputs and filesystem checks; assert variant resolution including the `.app` bundle fallback path.
- [ ] `cargo test` passes; `bun run check` passes.

### US-002: Persist variant preference in app settings

**Description:** As a user, I want my variant choice to persist between launches.

**Acceptance Criteria:**

- [ ] Add `skhd_variant: "auto" | "original" | "zig"` field to settings storage (default `"auto"`).
- [ ] Tauri commands `get_skhd_variant_setting` and `set_skhd_variant_setting` exposed and typed in `src/services/`.
- [ ] When setting is `"auto"`, `effective_variant()` returns the result of US-001 detection; otherwise returns the user's choice.
- [ ] If the user-chosen variant is not installed, surface a non-blocking warning (returned from the command) and still attempt the action.
- [ ] Typecheck passes; `bun run check` passes.

### US-003: Variant-aware service manager (start / stop / restart / reload / status)

**Description:** As a user, I want service controls to use the correct commands for whichever variant is active so start/stop/restart actually work for skhd.zig.

**Acceptance Criteria:**

- [ ] Refactor `src-tauri/src/services/service_manager.rs` so each public function (`get_status`, `start`, `stop`, `restart`, `reload`) dispatches on `effective_variant()`.
- [ ] **Original skhd path** (unchanged behavior):
  - status: parse `launchctl list` for `com.koekeishiya.skhd`
  - start/stop: `launchctl bootstrap`/`bootout` against `~/Library/LaunchAgents/com.koekeishiya.skhd.plist` (existing logic)
  - restart: existing `brew services restart skhd` fallback
  - reload: `skhd --reload`
- [ ] **skhd.zig path** (new):
  - status: parse `launchctl list` for `com.jackielii.skhd`; optionally also call `skhd --status` and surface output
  - start: `skhd --start-service`
  - stop: `skhd --stop-service`
  - restart: `skhd --restart-service`
  - reload: `skhd --reload` (same as original)
  - install (new action surfaced for skhd.zig only): `skhd --install-service`
- [ ] Errors include the variant in the message (e.g. `"skhd.zig: failed to start service: ..."`).
- [ ] Existing service tests still pass; new tests cover skhd.zig command dispatch.
- [ ] `cargo test` passes.

### US-004: Variant-aware config path resolution

**Description:** As a user with an XDG-style setup, I want Keybinder to find my skhdrc the same way current skhd variants do.

**Acceptance Criteria:**

- [ ] `service_manager::get_config_path` (or equivalent) checks paths in this order for both variants: `$XDG_CONFIG_HOME/skhd/skhdrc`, `~/.config/skhd/skhdrc`, `~/.skhdrc`.
- [ ] Error messages list the paths that were searched.
- [ ] Unit test covering both branches; `cargo test` passes.

### US-005: Parser support for skhd.zig directives — `.alias`, `.define`, `.path`

**Description:** As a user editing a skhd.zig config in Keybinder, I want non-binding directives parsed without errors so the rest of my file is still recognized.

**Acceptance Criteria:**

- [ ] Update `src-tauri/src/parser/grammar.pest` to recognize:
  - `.alias $name <modifier_combo|keycode>`
  - `.define <name> [ "app1", "app2" ]` (process group)
  - `.define <name> : <command_template_with_{{1}}>` (command template)
  - `.path "<dir>"` (PATH prepend)
- [ ] Update `parser/ast.rs` to add corresponding AST node variants (kept in a `ZigDirective` sub-enum so original-skhd parser remains pure).
- [ ] Parser does not emit syntax errors for valid skhd.zig configs containing only these directives.
- [ ] These directives appear as read-only items in the UI for now (display only — editing them is out of scope for this iteration).
- [ ] Unit tests with sample skhd.zig snippets; `cargo test` passes.

### US-006: Parser support for skhd.zig directives — `.device`, `.remap`, `mouse1`–`mouse5`, `backtick`, `@group`, mode-with-command

**Description:** As a user, I want the remaining skhd.zig syntax extensions to parse so my full config opens cleanly.

**Acceptance Criteria:**

- [ ] Grammar accepts:
  - `.device <alias> { vendor: 0xVVVV, product: 0xPPPP }`
  - `.remap <key> [<device>] : <key>` (1:1 form)
  - `.remap <key> [<device>] { tap: ..., hold: ..., timeout: 200ms, permissive_hold: on, hold_on_other_key_press: off, retro_tap: off }` (block form)
  - `mouse1`–`mouse5` and `backtick` as key tokens
  - `@<group>` references on the LHS of proc-map list entries
  - mode activation with command: `<binding> ; <mode> : <command>`
- [ ] AST nodes added for each.
- [ ] Read-only display in UI for these advanced features (no edit UI in this iteration).
- [ ] Unit tests over `SYNTAX.md` examples; `cargo test` passes.

### US-007: Settings UI — variant picker

**Description:** As a user, I want a settings panel where I can choose `Auto`, `skhd (original)`, or `skhd.zig`.

**Acceptance Criteria:**

- [ ] New section in the existing settings UI (or new `/settings` route if none exists) with a radio group / segmented control for variant.
- [ ] Selecting a value persists immediately via `set_skhd_variant_setting`.
- [ ] Below the picker, show detected state per variant: `✓ Installed (binary path)` / `Not found` / `Running` badges, refreshed on mount.
- [ ] When the chosen variant is not installed, show a yellow warning row with install instructions (links to US-009).
- [ ] Typecheck passes; `bun run check` passes.
- [ ] Verify in browser using dev-browser skill.

### US-008: Status indicator showing active variant

**Description:** As a user, I want to see which skhd variant is active at a glance.

**Acceptance Criteria:**

- [ ] The existing service-status component displays a badge: `skhd` or `skhd.zig`.
- [ ] Badge tooltip shows binary path + launchd label.
- [ ] Updates automatically after variant change in settings (reactive `$state` / `$derived`).
- [ ] Typecheck passes; `bun run check` passes.
- [ ] Verify in browser using dev-browser skill.

### US-009: Install guidance modal

**Description:** As a user without skhd.zig installed who selects it as the variant, I want clear install instructions.

**Acceptance Criteria:**

- [ ] When `effective_variant() == Zig` and detection returns `None`, show a dismissible info card or modal with:
  - Headline: "skhd.zig is not installed"
  - Code block with `brew install jackielii/tap/skhd-zig` and a copy-to-clipboard button
  - Secondary code block with `skhd --install-service` and copy button
  - Link to https://github.com/jackielii/skhd.zig (opens in default browser via `tauri-plugin-opener` or shell.open)
  - Same pattern for original skhd: `brew install koekeishiya/formulae/skhd`
- [ ] Re-detect on demand via a "Check again" button.
- [ ] Typecheck passes; `bun run check` passes.
- [ ] Verify in browser using dev-browser skill.

### US-010: Logs path and tailer respect variant

**Description:** As a user on skhd.zig, I want the in-app log viewer to read `~/Library/Logs/skhd.log` instead of `/tmp/skhd_$USER.log`.

**Acceptance Criteria:**

- [ ] `services/log_tailer.rs` accepts (or computes) the log path based on variant.
- [ ] Original variant: `/tmp/skhd_$USER.log` (unchanged).
- [ ] skhd.zig variant: `~/Library/Logs/skhd.log`.
- [ ] Empty-state message lists the path that was searched.
- [ ] Typecheck passes; `cargo test` passes; `bun run check` passes.
- [ ] Verify in browser using dev-browser skill.

### US-011: "Install skhd.zig service" button in settings

**Description:** As a user who just installed the skhd.zig binary, I want a one-click "Install service" button so I don't have to drop to the terminal to run `skhd --install-service`.

**Acceptance Criteria:**

- [ ] When variant is skhd.zig, the settings panel shows an "Install Service" button if the launchd agent is not registered.
- [ ] Clicking the button runs `skhd --install-service` via a Tauri command and surfaces stdout/stderr.
- [ ] After success, status auto-refreshes and the button is replaced with an "Uninstall Service" button (calls `skhd --uninstall-service`).
- [ ] A confirmation dialog appears before uninstall.
- [ ] Errors are shown inline with the underlying command output.
- [ ] Typecheck passes; `cargo test` passes; `bun run check` passes.
- [ ] Verify in browser using dev-browser skill.

### US-012: Migration wizard — original skhd → skhd.zig

**Description:** As an existing original-skhd user who wants to switch to skhd.zig, I want a guided migration flow so I don't have to manually stop/uninstall/install/start.

**Acceptance Criteria:**

- [ ] New "Migrate to skhd.zig" entry point in settings (visible only when original skhd is detected).
- [ ] Multi-step wizard:
  1. **Pre-flight:** show current original-skhd status + a checklist of what will happen.
  2. **Stop original:** stop `com.koekeishiya.skhd` via existing service-manager logic.
  3. **Install skhd.zig:** show `brew install jackielii/tap/skhd-zig` with copy-button; user clicks "I've installed it" to proceed (we do not run brew automatically).
  4. **Register service:** runs `skhd --install-service`.
  5. **Start service:** runs `skhd --start-service`.
  6. **Verify:** runs `skhd --status` and shows the result; offer "Switch variant preference to skhd.zig" toggle which updates settings (US-002).
- [ ] Each step is skippable and shows clear pass/fail state.
- [ ] Wizard can be cancelled at any step; partial state is non-destructive (the original skhd plist is not deleted).
- [ ] Wizard does not touch the user's `skhdrc` config file.
- [ ] Typecheck passes; `cargo test` passes; `bun run check` passes.
- [ ] Verify in browser using dev-browser skill.

### US-013: Documentation updates

**Description:** As a developer / user reading the project docs, I want skhd.zig support to be discoverable.

**Acceptance Criteria:**

- [ ] `README.md`: add a "Supported skhd variants" subsection mentioning skhd.zig with the install command and a callout that skhd.zig is the actively-developed fork.
- [ ] `CLAUDE.md` / `AGENTS.md`: note the variant abstraction in the architecture section.
- [ ] `CHANGELOG.md`: entry under Unreleased.

## 4. Functional Requirements

- **FR-1:** App MUST expose a settings field `skhd_variant` with values `auto` (default), `original`, `zig`.
- **FR-2:** App MUST detect installed variants by checking, in order: running launchd job → installed Homebrew formula → `which skhd` + `skhd --version` fingerprint.
- **FR-3:** When `skhd_variant = auto`, app MUST resolve to a concrete variant using FR-2 and prefer a _running_ variant over a merely-installed one.
- **FR-4:** Service start/stop/restart MUST use variant-correct commands:
  - Original: `launchctl bootstrap/bootout` + `brew services restart skhd` (current behavior).
  - skhd.zig: `skhd --start-service` / `--stop-service` / `--restart-service`.
- **FR-5:** `reload` MUST call `skhd --reload` for both variants (wire-compatible).
- **FR-6:** Status detection MUST distinguish `com.jackielii.skhd` from `com.koekeishiya.skhd` and return the correct label/PID.
- **FR-7:** Config-path resolution for both current variants MUST include `$XDG_CONFIG_HOME/skhd/skhdrc` first.
- **FR-8:** Parser MUST accept all skhd.zig 0.1.x directives listed in US-005/US-006 without raising syntax errors on valid files.
- **FR-9:** Parser AST MUST tag skhd.zig-only nodes so the UI can show them as read-only.
- **FR-10:** UI MUST show the active variant as a visible badge in the service-status component.
- **FR-11:** Settings UI MUST show, per variant, an install-state indicator and an install-help affordance.
- **FR-12:** Install-help MUST surface `brew install jackielii/tap/skhd-zig`, `skhd --install-service`, and the GitHub repo URL for skhd.zig; equivalent for original skhd.
- **FR-12a:** When a config contains block-form `.remap` rules that may require privileged `skhd-grabber` installation, Keybinder MUST not invoke `skhd --install-service` directly and MUST direct the user to run it in a terminal to review the privileged flow.
- **FR-13:** Log tailer MUST read from `~/Library/Logs/skhd.log` for skhd.zig and `/tmp/skhd_$USER.log` for original skhd.
- **FR-14:** All error messages from variant-aware code paths MUST identify which variant was being acted upon.

## 5. Non-Goals (Out of Scope)

- Editing UI for new skhd.zig directives (`.alias`, `.define`, `.path`, `.device`, `.remap`). They render read-only in this iteration; full edit support is a follow-up.
- Tap-hold / grabber daemon management (`com.jackielii.skhd.grabber`). The app will not install or manage the root-level grabber `LaunchDaemon` — users do this manually with `skhd --install-service`.
- Migrating user _config files_ between variants (no syntax-stripping or up-converting). Configs are presented as-is. **Service migration** (stop original / install zig / start) IS in scope — see US-012.
- Windows / Linux support — macOS-only is unchanged.
- Replacing or deprecating original skhd support.
- Auto-installing skhd.zig via Homebrew from inside the app (we show the command, we don't run it). The migration wizard pauses for the user to run brew themselves.
- Custom UI affordances for `mouse1`–`mouse5` bindings beyond plain text (deferred to a later design pass).

## 6. Design Considerations

- Reuse existing settings, status, and modal components rather than introducing new visual primitives.
- The variant badge should be visually understated (small pill near the service status) — not a giant header.
- Install-help blocks reuse the existing copy-to-clipboard pattern (search codebase for any existing copy button before adding a new one).
- Settings layout follows the existing settings page pattern (if none exists yet, follow the `/logs` route layout for consistency).
- Use Svelte 5 runes (`$state`, `$derived`, `$props`, `$effect`) per `AGENTS.md`.

## 7. Technical Considerations

- **Plist label constants** belong in a single Rust module (`src-tauri/src/services/variant.rs`) to avoid drift.
- `skhd --version` may or may not exist on older builds — fall back to checking the binary's parent directory (Homebrew Cellar paths differ: `koekeishiya/formulae` vs `jackielii/tap`).
- `launchctl list` output parsing already exists for original skhd — generalize it to take a label parameter rather than duplicating the parser.
- Avoid blocking the UI thread during detection; detection should be one async Tauri command that returns a snapshot.
- Existing tests in `src-tauri/tests/` rely on mocked commands — extend the same mocking pattern; do not add a new test framework.
- The grammar in `grammar.pest` is shared between original and skhd.zig parsing. Adding zig-only rules is acceptable as long as they remain optional and don't change parsing of valid original-skhd files.
- File-watching of skhdrc should still work with skhd.zig (it has its own `Hotload`, but Keybinder's reload flow still calls `skhd --reload` explicitly after writes).

## 8. Success Metrics

- A user with **only skhd.zig** installed can open Keybinder, see the variant auto-detected, start/stop/restart the service, edit shortcuts, and reload — all without manual configuration.
- A user with **both** installed can switch variants in settings and see the status indicator update within one second.
- Zero regressions in original-skhd workflows (existing test suite green).
- Parser accepts the entire skhd.zig `SYNTAX.md` example file without errors.
- README mentions skhd.zig and links to the upstream repo.

## 9. Open Questions — Resolved

- ~~Migration wizard?~~ **YES** — included as US-012.
- ~~`--install-service` button in settings?~~ **YES** — included as US-011.
- ~~`.app` bundle detection fallback?~~ **YES** — added to US-001 detection order (step 4).
- ~~Mouse button UI presentation?~~ **Deferred** — plain text for now; revisit in a later design pass.

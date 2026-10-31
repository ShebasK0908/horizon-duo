PASS — no additional runtime-proven defect in mobile fix round 2.

Review range: `3277bb98164804f8cc00bf7468493df71d6cf455..d4317cd74b45e80798afafc733272178979427e6`.
Worktree: `/Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web`.
Initial/final HEAD `4ad4970aa436f2783b8f53ff2dd81645230b86ac`, branch `codex/horizon-web`, clean, upstream divergence `0/0`. Later changes contain documentation/audit evidence only. No later input/harness changes; tracked input/harness hashes unchanged during review. See `pin-start.json`, `pin-final.json`, `tracked-hashes-start.json`, and `source.diff`.

Read AGENTS.md, CONTRIBUTING.md, both requested audit reports and their JSON receipts, complete source/harness diff, BrowserInput and consuming client tick. Fixed-scope read-only independent review, no delegation, no source/tracked edits or intentional Git writes.

Fresh execution

- Original cancellation reproduction: 6/6 PASS. Both cancelled-Jump repetitions peak at **-58.7478** from **-60**; observer receives peak **-58.750732421875**. Keyboard-only and cancelled-Forward controls also jump. Long keyboard/mixed gaps do not enable flight.
- Expanded real CDP/keyboard matrix: **19/19 PASS**, no page errors. Keyboard taps completed before/during cancellation, two completed keyboard taps during a cancelled contact, touch-only cancellation, old completed contacts, overlapping contacts, lost capture, valid touch/keyboard/mixed doubles, >700 ms stalled gaps, and cancellation preserving earlier double-tap timing.
- Owner probes: **6/6 PASS**. Independent key/touch releases, sampled touch cancellation with held key, two pointers with one losing capture or releasing, sole sampled owner cancellation. Remaining owners ascend about **3.747–3.749 blocks/500 ms**; measured drift after final release is **0**. Observer positions recorded.
- Trusted CDP pen events reuse browser pointer ID **2** for completed Jump contacts and a later cancelled Jump. Earlier completed single/double contacts survive. Real trusted `lostpointercapture` receipts confirm cancellation.
- Full committed `tools/web_touch_check.cjs`: **PASS**, all three regression assertions, movement/release, simultaneous look, cancellation, lost capture, source ownership, flight, authoritative gold placement/removal, and four viewports. Inspected `baseline/placed.png`: rendered gold block and touch controls visible. No page/GPU errors.
- Prior stalled-tap regression: two presses **717.2 ms** apart while RAF paused leave flight false. Trusted blur/focus regression: fresh touch and fresh W each move approximately **1.0425 blocks**; two real trusted Chrome blur/focus pairs recorded.
- Nine input unit tests: **9/9 PASS**, compiled with matching rustc from a byte-identical temporary copy. This does not claim a fresh full Cargo/Wasm build.

Scope ledger

| Requested item | State / receipt |
| --- | --- |
| Keyboard completed before cancelled touch | Locally end-to-end verified; cancellation-confirm.json, expanded.json |
| Keyboard completed during cancelled touch | Locally end-to-end verified; expanded.json |
| Already sampled owners | Locally end-to-end verified; flight-owners.json |
| Two owners | Locally end-to-end verified; flight-owners.json, expanded.json |
| Old completed contacts / reused IDs | Locally end-to-end verified for Jump; expanded.json, trusted pointer ID 2 |
| Cancel / lost capture | Locally end-to-end verified; expanded.json, flight-owners.json, baseline/browser.json |
| Valid doubles / source mixing | Locally end-to-end verified; expanded.json, flight-owners.json |
| Long stalled gaps | Locally end-to-end verified; cancellation-confirm.json, expanded.json |
| Two prior P2 regressions | Locally end-to-end verified; baseline/browser.json |
| Gameplay / resize | Locally end-to-end verified; baseline/browser.json |
| Actual artifact / private retrieval | Byte identity verified; artifact-start.json, artifact-final.json |
| Cleanup / final integrity | Verified; pin-final.json, summary.json |

Evidence boundaries and unrun surfaces

Gameplay uses real desktop Chrome WebGPU, CDP trusted touch/pen input and actual keyboard input, resource ZIP, and a fresh native server **25591** / bridge **18791**. The observer receives positions from that server. RAF instrumentation gates callbacks only; no Rust memory, physics, or network state mutation.

Local HTTP and private Tailscale HTML/JS/wasm return 200, match disk, and match at start/end. Wasm SHA-256:
`2ab67e89cc87b1eef188fe1e9d57a5b311fb40f721366c3e6c9d92a7de1fb938`.
This is served-artifact identity, separate from local gameplay and source-build provenance. No fresh Tailscale gameplay session or fresh Wasm rebuild was performed.

**COULD NOT RUN:** supplementary reused-pen-ID Look-role capture-loss combination. The first helper produced no Look `lostpointercapture`, so its apparent pass is not cancellation proof. A corrected helper explicitly trying capture failed with Chrome `InvalidStateError: Failed to execute 'setPointerCapture' on 'Element': InvalidStateError`. Both raw attempts are retained and excluded from cancellation coverage; reused-Look cancellation has unit evidence only. This is a harness limitation, not a finding in the fix diff.

Physical phone/Safari/Duo and actual OS background suspension unrun; DEVICE_IN_USE respected. Cross-family review remains blocked by Codex-only configuration, no Claude retry. Native platform client/full parity, offline, X execution, public hosting/deployment, desktop multiplayer suite and tick suite were not run in this focused pass.

Cleanup

Owned native server **77028** stopped normally, exit **0**; owned bridge **77174** terminated, exit **143**. Both PIDs absent and ports **25591/18791** closed. All test Chrome contexts/browsers closed in finally; temporary preview tab closed; no owned test Node or Playwright Chrome process remains. Protected PIDs **55502 / 88366 / 88504 / 9771** retain original commands. No Tailscale configuration change, device action, external message, deployment, PR, merge, push, or agent spawn.

Exact reproduction commands: `commands.txt`. Raw receipts: `receipts.zip`; individual JSON/log/scripts retained beside this report. Fixture worlds/resources are not included in the receipt ZIP.

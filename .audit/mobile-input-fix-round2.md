# Mobile input fix round 2

The goal is reliable Horizon browser input while preserving authoritative
multiplayer. This round starts from clean pushed `3277bb98164804f8cc00bf7468493df71d6cf455`.
The fresh review passed both earlier corrections but proved one additional P2.

| Finding | Before | After | State |
| --- | --- | --- | --- |
| Cancelling Jump touch clears a completed queued keyboard jump | Keyboard-only and cancelled-Forward controls peak at -58.7478; both cancelled-Jump cases stay at -60 | Both cancelled-Jump cases peak at -58.7478, with server-delivered observer ascent; flight stays false | Implemented and parent browser-verified; fresh review pending |

The previous queue kept only aggregate boolean transitions. It could not remove
one source's pending press without discarding unrelated input. The correction
retains `JumpTransition` source, press/release state, and event timestamp until
sampling. Live owners and sampled owners represent the two ends of this queue.
Sampling derives aggregate edges while preserving each owner's lifetime.
Cancellation removes only the active pointer's latest unconsumed press and then
releases that source. Completed keyboard taps and earlier completed contacts,
including reused pointer IDs, remain queued. A press already sampled is released
normally. Completed gesture history is not globally reset.

Model the Domain changed the queue to retain ownership rather than merge it
early. Fix Root Causes removed the blanket queue clear. No renderer, network,
resource, native gameplay, or physics source changed.

## Reproduction

The parent copied the review's exact `cancellation-confirm.cjs` into a new
temporary directory, changing only its local bridge from 18789 to 18787.
The original review is preserved in `mobile-adversarial-round2.md`.

```sh
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-cancel-fix-round2.udmtwln2/before/cancellation-confirm.cjs
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-cancel-fix-round2.udmtwln2/after/cancellation-confirm.cjs
```

The before run reproduced both cancelled-Jump failures. The after run passed
all six cases, including repeated cancellation, keyboard-only and cancelled-
Forward controls, and long keyboard/mixed tap gaps without flight. Parent
assertions checked the failures, corrected peaks and matching observer ascent.
`mobile-cancel-repros.json` retains input events, client results and the relevant
server-delivered observer samples. The full raw samples and scripts remain in
the temporary directory above.

## Final artifact

The committed touch driver now stalls RAF, completes a Space tap, cancels a
Jump touch, resumes and asserts the jump survives without toggling flight. The
full driver passed this case, both prior P2 regressions, valid double-tap flight,
held/mixed controls, block placement/removal and four viewport sizes. The parent
inspected its fresh `placed.png` and saw the gold block and touch controls.
Owned-texture captures remain private in `.local/web-mobile-fix-round2/`.

```sh
export PATH=/Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH
export RUSTC=/Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc
cargo fmt -p horizon-web
cargo test -p horizon-web --lib
cargo clippy -p horizon-web --target wasm32-unknown-unknown -- -D warnings -A clippy::manual_range_contains
python3 tools/build_web.py
node --check tools/web_touch_check.cjs
python3 tools/check_public_tree.py
git diff --check
HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright \
HORIZON_WEB_URL='http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:18787' \
HORIZON_WEB_PROOF=.local/web-mobile-fix-round2 node tools/web_touch_check.cjs
```

Nine input tests pass, including keyboard taps before and during cancelled touch,
reused pointer IDs, pending versus sampled cancellation and mixed ownership.
Scoped Clippy passes with only the existing aquifer `manual_range_contains`
exception. No new comments or suppressions were added.

Gameplay used a disposable native world on 25587 and bridge on 18787, created
with the existing test-server tool and fresh game/launcher/XDG directories.
The original live services and Tailscale routes were untouched.
`mobile-cancel-touch.json` records the full driver. `mobile-cancel-served.json`
confirms the private Tailscale HTML, JavaScript and wasm match local bytes.
Final wasm SHA-256 is `2ab67e89cc87b1eef188fe1e9d57a5b311fb40f721366c3e6c9d92a7de1fb938`.
Byte identity is separate from a new Tailscale gameplay run.

Fresh runtime review of this correction remains pending. Desktop multiplayer
and tick receipts from fix round 1 remain prior evidence; those unrelated
checks were not repeated for this queue-only correction. Actual phone/Safari/
Duo, cross-family review, offline browser implementation, native parity and X
execution remain unfinished. The two offline design candidates are inspected,
not implemented. No device, public deployment, PR, merge or X post occurred.

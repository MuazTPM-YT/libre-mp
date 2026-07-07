# Foundation: Repo Slimming + Core Library Extraction — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Purge committed build artifacts from git history and restructure the two Rust crates into a Cargo workspace with a shared `libremp-core` library, deleting the dead protocol reimplementation — without changing runtime behavior.

**Architecture:** A root Cargo workspace ties together `core/` (all real logic, moved verbatim from `Rust/src`), a thin `cli/` binary, and the existing `frontend/src-tauri` GUI (which will depend on `core` instead of spawning a sibling binary). This plan stops at "everything builds and the proven code paths are unchanged"; discovery, protocol, and capture rewrites are separate follow-on plans.

**Tech Stack:** Rust 1.93 (edition 2021), Cargo workspaces, git-filter-repo, Tauri 2, turbojpeg/scrap/xcap.

## Global Constraints

- Rust edition: **2021**. Toolchain floor: **1.93**.
- The proven streaming path (`protocol.rs` handshake bytes, `template.rs` framing, keepalives) must remain **byte-for-byte unchanged** in this plan — this is a move/restructure, not a rewrite.
- `windows_perfect_stream.bin` (3.2 MB template) stays tracked; it is a required asset, not a build artifact.
- Never force-push until the backup tag AND bundle in Task 1 both exist.
- The app must build (`cargo build` in each crate) at the end of every task that touches Rust.
- No new runtime dependencies in this plan.

---

## File Structure (end state of this plan)

```
projector/
├── Cargo.toml                 # NEW: workspace root (members: core, cli, frontend/src-tauri)
├── .gitignore                 # REWRITTEN as UTF-8
├── windows_perfect_stream.bin # unchanged (tracked)
├── core/                      # NEW crate: libremp-core (lib)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # NEW: re-exports modules
│       ├── protocol.rs        # moved verbatim from Rust/src/protocol.rs
│       ├── capture.rs         # moved verbatim from Rust/src/capture.rs
│       ├── wifi.rs            # moved verbatim from Rust/src/wifi.rs
│       ├── template.rs        # moved verbatim from Rust/src/template.rs
│       └── hex.rs             # moved verbatim from Rust/src/hex.rs
├── cli/                       # NEW crate: thin CLI over core
│   ├── Cargo.toml
│   └── src/main.rs            # moved from Rust/src/main.rs, imports from libremp_core
└── frontend/
    └── src-tauri/
        ├── Cargo.toml         # add libremp-core dependency; drop unused deps later
        └── src/
            ├── lib.rs         # keep Tauri commands; protocol/ module deleted
            └── protocol/      # DELETED (dead code)
```

`Rust/` directory is removed once its contents are relocated.

---

## Task 1: Back up the repository before any history rewrite

**Files:**
- Create: `../projector-backup-2026-07-07.bundle` (outside the repo)

**Interfaces:**
- Consumes: nothing.
- Produces: a restorable backup + an annotated tag `pre-history-purge`.

- [ ] **Step 1: Create an annotated safety tag on current HEAD**

```bash
cd /home/Muaz/Documents/Software/projector
git tag -a pre-history-purge -m "State before target/ history purge (2026-07-07)"
```

- [ ] **Step 2: Create a full git bundle backup outside the repo**

```bash
git bundle create ../projector-backup-2026-07-07.bundle --all
```

- [ ] **Step 3: Verify the bundle is valid and complete**

Run:
```bash
git bundle verify ../projector-backup-2026-07-07.bundle
```
Expected: output ends with `The bundle records a complete history` and lists refs including `refs/heads/main` and `refs/tags/pre-history-purge`.

- [ ] **Step 4: Record the current bloat baseline for later comparison**

Run:
```bash
du -sh .git ; git ls-files 'Rust/target' | wc -l
```
Expected: roughly `935M .git` and `49156`. Note these numbers; Task 4 verifies they drop.

---

## Task 2: Rewrite `.gitignore` as UTF-8 and stop tracking build artifacts

**Files:**
- Modify: `.gitignore` (currently UTF-16, the root cause of the bloat)

**Interfaces:**
- Consumes: nothing.
- Produces: a correct UTF-8 `.gitignore`; `Rust/target` untracked in the index.

- [ ] **Step 1: Confirm the current `.gitignore` is mis-encoded (UTF-16)**

Run:
```bash
file .gitignore
```
Expected: reports `Unicode text, UTF-16` (or shows null bytes) — this is why its rules never matched.

- [ ] **Step 2: Overwrite `.gitignore` with correct UTF-8 content**

Write `.gitignore` (plain UTF-8) with exactly:

```gitignore
# Rust build artifacts
/Rust/target/
/core/target/
/cli/target/
/target/
/frontend/src-tauri/target/

# Node / frontend
/frontend/node_modules/
/frontend/dist/

# LibreMP saved config (contains PSKs)
projectors.json

# OS / editor cruft
.DS_Store
*.orig
*.rej
```

- [ ] **Step 3: Verify the new file is UTF-8**

Run:
```bash
file .gitignore
```
Expected: `ASCII text` or `UTF-8 Unicode text` (no UTF-16, no null bytes).

- [ ] **Step 4: Remove build artifacts from the git index (keep them on disk)**

Run:
```bash
git rm -r --cached --quiet Rust/target
git status --short | head
```
Expected: thousands of `D  Rust/target/...` deletions staged; working-tree files remain present on disk.

- [ ] **Step 5: Commit the untracking + gitignore fix**

```bash
git add .gitignore
git commit -q -m "chore: stop tracking Rust/target, fix UTF-16 .gitignore"
git ls-files 'Rust/target' | wc -l
```
Expected: commit succeeds; the count prints `0`.

---

## Task 3: Purge `Rust/target` from all git history

**Files:**
- None (history rewrite only).

**Interfaces:**
- Consumes: the backup + tag from Task 1, the untracking from Task 2.
- Produces: a rewritten history with no `Rust/target` blobs.

- [ ] **Step 1: Install git-filter-repo (not currently present)**

Run (Arch):
```bash
sudo pacman -S --needed git-filter-repo
command -v git-filter-repo
```
Expected: prints a path. If pacman is unavailable, fall back to `pipx install git-filter-repo` or `python -m pip install --user git-filter-repo`.

- [ ] **Step 2: Purge the path from every commit**

Run:
```bash
cd /home/Muaz/Documents/Software/projector
git filter-repo --path Rust/target --invert-paths --force
```
Expected: completes with a "Parsed N commits" / "New history written" summary. (filter-repo removes the `origin` remote by design; that is expected.)

- [ ] **Step 3: Expire reflogs and garbage-collect aggressively**

Run:
```bash
git reflog expire --expire=now --all
git gc --prune=now --aggressive
```
Expected: gc completes without error.

- [ ] **Step 4: Verify history no longer contains the artifacts**

Run:
```bash
git log --all --oneline -- Rust/target | head
```
Expected: **no output** (the path exists in no commit).

---

## Task 4: Verify repo slimming succeeded

**Files:** none.

**Interfaces:**
- Consumes: Task 3 result.
- Produces: confirmation `.git` shrank and the working app is intact.

- [ ] **Step 1: Confirm `.git` size dropped dramatically**

Run:
```bash
du -sh .git
```
Expected: single-digit to low double-digit MB (down from ~935 MB). If still hundreds of MB, gc did not run — repeat Task 3 Step 3.

- [ ] **Step 2: Confirm the spec + design docs survived the rewrite**

Run:
```bash
git log --oneline -- docs/superpowers/specs | head
ls docs/superpowers/specs docs/superpowers/plans
```
Expected: the design-spec commit is present; both doc files exist.

- [ ] **Step 3: Confirm the source tree is intact and still builds**

Run:
```bash
cd Rust && cargo build 2>&1 | tail -5 ; cd ..
```
Expected: `Finished` (a successful build using the on-disk `Rust/target` cache, which is now untracked but still present).

> **Note on remote:** `git filter-repo` intentionally drops the `origin` remote. Re-adding it and force-pushing (`git remote add origin <url>` then `git push --force --all`) is a manual step the user performs when ready; it is **not** part of automated execution because it is irreversible for collaborators.

---

## Task 5: Create the Cargo workspace and `libremp-core` library crate

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `core/Cargo.toml`, `core/src/lib.rs`
- Move: `Rust/src/{protocol,capture,wifi,template,hex}.rs` → `core/src/`

**Interfaces:**
- Consumes: nothing.
- Produces: `libremp_core` crate exposing `pub mod protocol; pub mod capture; pub mod wifi; pub mod template; pub mod hex;` and constants `STREAM_W: u32`, `STREAM_H: u32`, `JPEG_QUALITY: i32`.

- [ ] **Step 1: Create the `core/src` directory and move the modules verbatim**

Run:
```bash
cd /home/Muaz/Documents/Software/projector
mkdir -p core/src
git mv Rust/src/protocol.rs core/src/protocol.rs
git mv Rust/src/capture.rs  core/src/capture.rs
git mv Rust/src/wifi.rs     core/src/wifi.rs
git mv Rust/src/template.rs core/src/template.rs
git mv Rust/src/hex.rs      core/src/hex.rs
```

- [ ] **Step 2: Write `core/src/lib.rs`**

The shared constants currently live in `main.rs`; move them into the library so both front-ends use one definition.

Create `core/src/lib.rs`:
```rust
//! libremp-core: shared discovery, protocol, capture, and framing logic.

pub mod hex;
pub mod wifi;
pub mod template;
pub mod capture;
pub mod protocol;

/// Streaming frame width negotiated with Epson projectors.
pub const STREAM_W: u32 = 1024;
/// Streaming frame height negotiated with Epson projectors.
pub const STREAM_H: u32 = 768;
/// Baseline JPEG quality before per-tile adaptive downscaling.
pub const JPEG_QUALITY: i32 = 95;
```

- [ ] **Step 3: Repoint intra-crate references from `crate::` root constants**

`core/src/capture.rs` line 4 imports `use crate::{STREAM_W, STREAM_H, JPEG_QUALITY};` — this still resolves against the library root, so it is unchanged. Verify no module referenced `crate::main` or binary-only items:
```bash
grep -rn "crate::" core/src | grep -vE "crate::(STREAM_W|STREAM_H|JPEG_QUALITY|hex|wifi|template|capture|protocol)" || echo "clean"
```
Expected: `clean`.

- [ ] **Step 4: Write `core/Cargo.toml`**

Create `core/Cargo.toml`:
```toml
[package]
name = "libremp-core"
version = "0.1.0"
edition = "2021"

[dependencies]
turbojpeg = "1"
byteorder = "1"

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winsock2", "ws2def", "mstcpip", "minwindef", "winuser", "wingdi", "errhandlingapi", "winbase", "memoryapi", "libloaderapi", "windef"] }
```

- [ ] **Step 5: Write the workspace root `Cargo.toml`**

Create `Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["core", "cli"]
# frontend/src-tauri is its own workspace (Tauri convention); it depends on core by path.

[profile.release]
opt-level = 3
```

- [ ] **Step 6: Build the library alone**

Run:
```bash
cargo build -p libremp-core 2>&1 | tail -8
```
Expected: `Finished`. (turbojpeg needs system `libturbojpeg`/`cmake`; if it errors on that, it is a pre-existing environment need, not a plan defect — note and continue.)

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml core/
git commit -q -m "refactor: extract libremp-core library crate from Rust/src"
```

---

## Task 6: Create the thin `cli` crate over `libremp-core`

**Files:**
- Create: `cli/Cargo.toml`
- Move: `Rust/src/main.rs` → `cli/src/main.rs` (rewire imports)
- Delete: the now-empty `Rust/` directory

**Interfaces:**
- Consumes: `libremp_core::{protocol, capture, template, wifi, STREAM_W, STREAM_H, JPEG_QUALITY}`.
- Produces: binary `epson-streamer` with identical CLI flags (`--skip-wifi --ssid --password --os --projector-ip`).

- [ ] **Step 1: Move main.rs into the cli crate**

Run:
```bash
mkdir -p cli/src
git mv Rust/src/main.rs cli/src/main.rs
```

- [ ] **Step 2: Rewrite the module/import head of `cli/src/main.rs`**

Replace the top of the file (the `mod hex; mod wifi; ...` block and the `pub const` block, lines 6–18 of the original) so it consumes the library instead of declaring modules.

Change:
```rust
mod hex;
mod wifi;
mod template;
mod capture;
mod protocol;

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const STREAM_W: u32 = 1024;
pub const STREAM_H: u32 = 768;
pub const JPEG_QUALITY: i32 = 95;
const TARGET_FPS: u64 = 24;
```
to:
```rust
use libremp_core::{capture, protocol, template, wifi, STREAM_W, STREAM_H};

use std::collections::HashMap;
use std::time::{Duration, Instant};

const TARGET_FPS: u64 = 24;
```
(`JPEG_QUALITY` and `STREAM_W/H` are referenced inside `capture`, which now reads them from the library; `main.rs` itself only needs `STREAM_W`/`STREAM_H` for the `resize_bgra_to_rgb` calls. If the compiler reports an unused import, remove the specific unused name it names.)

- [ ] **Step 3: Fix the `scrap` capture path — it must move to core or cli**

`main.rs` uses `scrap::{Capturer, Display}` directly (lines 4, 115–120, 172–194). `scrap` is a capture concern. For this move-only plan, add `scrap` to the **cli** crate so behavior is unchanged; the capture-unification plan will later relocate it into `core::capture`.

Add to imports at top of `cli/src/main.rs`:
```rust
use scrap::{Capturer, Display};
```

- [ ] **Step 4: Write `cli/Cargo.toml`**

Create `cli/Cargo.toml`:
```toml
[package]
name = "epson-streamer"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "epson-streamer"
path = "src/main.rs"

[dependencies]
libremp-core = { path = "../core" }
ctrlc = "3"
scrap = "0.5.0"
```

- [ ] **Step 5: Fix the template path lookup for the new layout**

`main.rs` `find_template()` looks for `../windows_perfect_stream.bin` and `windows_perfect_stream.bin`. With the binary run from the workspace root or `cli/`, add the workspace-root-relative path. Change the array in `find_template()` to:
```rust
    for path in [
        "windows_perfect_stream.bin",
        "../windows_perfect_stream.bin",
        "../../windows_perfect_stream.bin",
    ] {
```

- [ ] **Step 6: Build the CLI**

Run:
```bash
cargo build -p epson-streamer 2>&1 | tail -12
```
Expected: `Finished`. Resolve any unused-import warnings by deleting exactly the names the compiler flags.

- [ ] **Step 7: Smoke-test the CLI help path (no projector needed)**

Run:
```bash
cargo run -p epson-streamer -- --skip-wifi --ssid TEST --password 001122334455 --os 3 2>&1 | head -5
```
Expected: prints the `=== Epson EasyMP Rust Streamer ===` banner and config lines, then attempts/‌fails to connect (fine — no projector present). It must NOT panic on missing modules or template.

- [ ] **Step 8: Remove the emptied `Rust/` directory and commit**

Run:
```bash
rmdir Rust/src 2>/dev/null; rm -f Rust/Cargo.toml Rust/Cargo.lock Rust/err.txt Rust/err2.txt Rust/projectors.txt
git add -A Rust cli
git rm -r --cached --quiet Rust 2>/dev/null || true
git commit -q -m "refactor: add thin cli crate over libremp-core, retire Rust/ dir"
```

---

## Task 7: Point the Tauri app at `libremp-core` and delete the dead protocol code

**Files:**
- Delete: `frontend/src-tauri/src/protocol/` (mod.rs, client.rs, streamer.rs, payloads.rs, config.rs)
- Modify: `frontend/src-tauri/src/lib.rs` (remove `pub mod protocol;`)
- Modify: `frontend/src-tauri/Cargo.toml` (add `libremp-core` path dep; remove deps only the dead code used)

**Interfaces:**
- Consumes: `libremp_core` (path dependency).
- Produces: a Tauri app that still spawns the `epson-streamer` binary (unchanged behavior), now with the dead reimplementation gone and core available for the follow-on plans.

- [ ] **Step 1: Confirm the dead module is truly unused by the GUI**

Run:
```bash
grep -rn "protocol::" frontend/src-tauri/src/lib.rs frontend/src-tauri/src/main.rs || echo "no references"
```
Expected: `no references` — `lib.rs` only declares `pub mod protocol;` but never calls into it (the GUI spawns the CLI binary instead).

- [ ] **Step 2: Delete the dead protocol module**

Run:
```bash
git rm -r --quiet frontend/src-tauri/src/protocol
```

- [ ] **Step 3: Remove its declaration from `lib.rs`**

In `frontend/src-tauri/src/lib.rs`, delete the line:
```rust
pub mod protocol;
```

- [ ] **Step 4: Add `libremp-core` as a path dependency**

In `frontend/src-tauri/Cargo.toml`, under `[dependencies]`, add:
```toml
libremp-core = { path = "../../core" }
```

- [ ] **Step 5: Drop dependencies only the dead code used**

Check whether `xcap`, `image`, and `tokio`'s streaming features are still referenced by remaining GUI code:
```bash
grep -rn -E "xcap|image::|DynamicImage" frontend/src-tauri/src || echo "unused"
```
If `unused`, remove `xcap` and `image` from `frontend/src-tauri/Cargo.toml`. Keep `tokio`, `regex`, `lazy_static`, `serde`, and Tauri deps (still used by `lib.rs`).

- [ ] **Step 6: Build the Tauri backend crate**

Run:
```bash
cd frontend/src-tauri && cargo build 2>&1 | tail -12 ; cd ../..
```
Expected: `Finished`. Fix any unused-import fallout from the deletions by removing exactly the flagged names.

- [ ] **Step 7: Commit**

```bash
git add -A frontend/src-tauri
git commit -q -m "refactor: delete dead src-tauri protocol reimpl, depend on libremp-core"
```

---

## Task 8: Add a regression snapshot test locking the proven handshake bytes

**Files:**
- Create: `core/tests/handshake_snapshot.rs`

**Interfaces:**
- Consumes: `libremp_core::protocol` internals. To test private payload builders, expose them via a `#[cfg(test)]`-friendly path: add `pub(crate)` test hooks or make the builders `pub` in a `pub mod` — minimal, see Step 1.

- [ ] **Step 1: Expose the payload builders for testing**

In `core/src/protocol.rs`, change the three builder signatures from private to `pub` so the integration test can call them (they take only IPs/strings, no sockets):
```rust
pub fn registration_payload(my_ip: Ipv4Addr) -> Vec<u8> {
pub fn auth_payload(my_ip: Ipv4Addr, proj_ip: Ipv4Addr, password: &str, ssid: &str) -> Vec<u8> {
```
(`response_0x0108` is already `pub`.)

- [ ] **Step 2: Write the failing snapshot test**

Create `core/tests/handshake_snapshot.rs`:
```rust
use std::net::Ipv4Addr;
use libremp_core::protocol::{registration_payload, auth_payload, response_0x0108};

#[test]
fn registration_payload_is_stable() {
    let p = registration_payload(Ipv4Addr::new(192, 168, 88, 2));
    // 8 magic + 4 ip + 20 fixed + 32 zero = 64 bytes
    assert_eq!(p.len(), 64);
    assert_eq!(&p[0..8], b"EEMP0100");
    assert_eq!(&p[8..12], &[192, 168, 88, 2]);
}

#[test]
fn auth_payload_encodes_mac_and_ssid_name() {
    // password is the projector's wired MAC (hex, no separators)
    let p = auth_payload(
        Ipv4Addr::new(192, 168, 88, 2),
        Ipv4Addr::new(192, 168, 88, 1),
        "b0f8ef531400",
        "RESEARCHLAB-abcdef123456",
    );
    assert_eq!(&p[0..8], b"EEMP0100");
    // The SSID prefix "RESEARCHLAB" (11 bytes) must appear as the padded name.
    let needle = b"RESEARCHLAB";
    assert!(p.windows(needle.len()).any(|w| w == needle), "ssid name not embedded");
}

#[test]
fn response_0x0108_rewrites_local_ip() {
    let r = response_0x0108(Ipv4Addr::new(10, 0, 0, 5));
    // The baked template IP 192.168.88.2 must be fully replaced by 10.0.0.5.
    assert!(!r.windows(4).any(|w| w == [192, 168, 88, 2]), "stale baked IP remains");
    assert!(r.windows(4).any(|w| w == [10, 0, 0, 5]), "new IP not written");
}
```

- [ ] **Step 3: Run the test to verify it fails to compile (builders still private)**

Run:
```bash
cargo test -p libremp-core --test handshake_snapshot 2>&1 | tail -12
```
Expected: compile error `function ... is private` — proving the test targets the right symbols. (After Step 1's visibility change it should compile; if you did Steps in order, this instead runs.)

- [ ] **Step 4: Run the test to verify it passes**

Run:
```bash
cargo test -p libremp-core --test handshake_snapshot 2>&1 | tail -12
```
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Commit**

```bash
git add core/src/protocol.rs core/tests/handshake_snapshot.rs
git commit -q -m "test: snapshot proven Epson handshake payloads as regression guard"
```

---

## Self-Review

**Spec coverage (this plan's slice):**
- Spec §2 architecture (workspace + core lib, delete dead code) → Tasks 5, 6, 7. ✓
- Spec §3.5 repo size (gitignore UTF-8, rm --cached, filter-repo purge, verify) → Tasks 1–4. ✓
- Spec §5 regression guard ("snapshot emitted handshake bytes") → Task 8. ✓
- Deferred to follow-on plans (explicitly out of this plan): §3.1 discovery-first, §3.2 protocol de-hardcoding, §3.3 universal capture, §3.4 config.rs, §3.6 UI polish. These require iterative hardware testing.

**Placeholder scan:** No TBD/TODO; every code step shows full content; commands have expected output. ✓

**Type consistency:** `libremp-core` exposes `STREAM_W/STREAM_H/JPEG_QUALITY` and modules `protocol/capture/wifi/template/hex`; `cli/main.rs` and `capture.rs` consume exactly those names; test calls `registration_payload`/`auth_payload`/`response_0x0108` matching the (now `pub`) signatures in Task 8 Step 1. ✓

**Known caveat surfaced, not hidden:** `turbojpeg` build needs system `cmake`/`libturbojpeg`; flagged in Task 5 Step 6 as environment, not a plan defect. The `git filter-repo` force-push to a remote is deliberately left as a manual user step (Task 4 note) because it is irreversible for any collaborator.

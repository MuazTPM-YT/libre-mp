# LibreMP Engineering Notes

The code keeps one short comment per function (see `CLAUDE.md`). The longer reasoning behind the code lives here.

## EasyMP protocol (`core/src/protocol.rs`)

Ground truth is a Windows iProjection session capture: `git show b1a85c2:test.pcap` (client `192.168.88.2`, projector `192.168.88.1`). `core/tests/handshake_snapshot.rs` locks the control-channel bytes to it. The real MAC is replaced with a fake one, because the MAC only passes through.

- **Ports.** Control is on TCP 3620 (EEMP). Video and audio are on TCP 3621 (EPRD).
- **Where the projector is.** In Simple AP / Quick Connect mode the projector is the DHCP server and gives itself out as the router. So its address is a default gateway. We try every default gateway, because a laptop also on Ethernet or a VPN has several. Order: an explicit IP (from the QR code), then the gateways, then `192.168.88.1`.
- **Bounded connects.** Each connect times out after 3 s. A wrong address must fail in seconds, not in the OS's ~2 minutes.
- **Registration (`0x0002`).** It is 68 bytes; older code sent 64. The projector answers with its name (`0x0003`, `payload[4..36]`) and MAC (`0x0015`, `payload[0..6]`; `0x0003` `payload[48..54]` as a fallback). The Windows client uses these for login, so no password or SSID parsing is needed.
- **Login (`0x0101`).** It is a TLV structure, byte-identical to Windows when there is no keyword. Byte 142 is TLV tag `0x0B`, not the name length. It only looked like `len("RESEARCHLAB") = 11`, and that hid a bug.
- **Projector Keyword.** It is 4 ASCII digits in the 16-byte field right after the MAC, zero-padded. Our capture had the keyword switched off, so this placement comes from Rhino Security Labs' working EasyMP PIN tool (`RhinoSecurityLabs/Security-Research`, `exploits/Epson/easymp-pintest.py`). That tool puts the PIN in the same position (MAC, then PIN, then zero padding, then projector IP) in an older version of the packet. It has not yet run against real keyword-protected hardware.
- **Login reply (`0x0102`).** Byte 51 (payload offset 30) is `0` on success. A non-zero value means refused: a wrong or missing keyword, or another device presenting. A refusal ends the cast at once. Retrying cannot fix it.
- **After login.** The projector sends status queries (`0x010E`). We must answer with `0x0108`, or the projector resets the session after about 50 s. `0x0110` means "ready to stream". `0x0016` arrives after the video channels open.
- **Heartbeat reads are non-blocking.** On Windows, a socket read that times out leaves the socket in an undefined state. So `drain_auth` switches to non-blocking for one read instead.
- **Aux channel.** The Windows client sends silent audio: 2646- and 1764-byte zero buffers, alternating every 50 ms. We send both every 100 ms. Each has a 5-byte header (`0xC9` + little-endian size).
- **Goodbye.** On stop, Windows sends `0x0104`, and the projector replies `0x0105` and frees the session at once. We do the same. The desktop app also does it when it quits.
  - The reply can take more than 1 s (we measured 0.9 s to 1.24 s). We wait up to 3 s for `0x0105` before we close the sockets. If we close first, the reply arrives at a closed socket and the kernel sends a reset. The projector then treats the session as a lost link and keeps the last picture on screen.

## Video frames (EPRD)

Checked against `windows_perfect_stream.bin`: the reassembled video channel of the Windows session, 106 blocks. `core/tests/frame_builder.rs` rebuilds **all 105 JPEG blocks byte for byte**.

- **Block header.** `EPRD0600` + sender IP + msg id (0) + size. The META block's size is little-endian. The JPEG block's size is big-endian.
- **META.** A 46-byte display config. Windows sends it once, as the first block. We send it with every whole frame. This was proven on hardware by the old template, and the projector accepts it.
  - Fields (big-endian `u16`): bytes 4–7 are the frame size (1024×768), bytes 24–27 are the PC screen (1600×900), and bytes 28–35 are the picture box (0, 96, 1024, 576).
  - We send Windows' bytes unchanged. On RESEARCHLAB, changing the PC size or the box had no visible effect (grid and circle tests), except that 1920×1080 looked worse.
- **Picture size limit.** The projector reports 1024×768 as its frame size (`0x0015`, tag 3), and Windows sends 1024×768. A 1280×800 frame (auth bytes and META changed to match) connected, but the projector showed nothing. So 1024×768 is the limit.
- **How RESEARCHLAB shows a frame** (grid photo and circle test, 2026-10-01):
  - It shows the whole 1024×768 frame, not just the META box. It cuts about 5.5% off every edge (overscan), so only about (56, 42, 912, 684) is visible.
  - It stretches the 4:3 frame to a 16:9 shape, so one frame pixel looks 4/3 wide. Of four pre-squeezed circles, the 1.33 one looked round.
  - So `capture::fit_rect` fits the screen into the visible part as it looks on the wall. A 1920×1080 desktop fills all 912×684 (stretched in the frame, undone by the projector). This is close to the very first version, which stretched over the whole frame and only lost the edges to overscan.
  - Windows' layout (16:9 in rows 96–672) looks squashed on this projector, and our first letterbox attempts did too.
  - Both numbers (`VISIBLE`, `PIXEL_ASPECT`) are measured on this projector only. Another projector may need different ones.
  - Detail: the projector scales the picture up, so 1-pixel lines blur and 2-pixel lines stay clear (line-chart test).
- **JPEG payload.** A big-endian `u32`, then per tile: a 16-byte descriptor (`x, y, w, h` as BE `u16`, flags `0x00000007`, a 4-byte length tag) and the JPEG bytes. The first `u32` is the **tile count**. Earlier notes called it a "frame type" (4 = key, 3/1 = delta). That was wrong: it always equals the number of tiles.
- **Windows sends mostly partial frames.** More than 90 of the 105 blocks cover only the changed areas. Tiles are multiples of 16 (4:2:0 JPEG), at most 624×416, and are cut from the origin of each changed area.
- **Length tag (was called "ts").** It is the JPEG's byte length: `0x90`, then the length as a 3-byte varint (7 bits per byte, low bits first, high bit set on the first two bytes). All 233 tiles in the capture match. The projector reads each JPEG by this length. A wrong value makes it send `0x0111` on the control channel and reset the video channel about 0.3 s after the first frame, so the cast loops "connected, dropped, reconnecting". The old code sent fixed values copied from the capture. That only worked while every tile was padded to the capture's exact JPEG sizes (the March template streamer).

### Sending only what changed (`core/src/session.rs`)

- Compare each new frame with the last one sent, on a 32 px grid.
- Changed cells in a row form runs. A run with the same columns as the run above it extends that area downward.
- More than 64 areas: send the whole frame (noise, a video playing). More than 8 areas: greedily merge the pair whose union wastes the least area, until 8 are left. More than 60 % of the screen: send the whole frame.
- Cut each area into tiles of at most 624×416. Every tile is JPEG quality 90 (4:2:0). There is no size budget. Old code squeezed each tile into Windows' byte sizes, which dropped busy tiles to very low quality. Large frames just send slower. The projector took 413 KB noise frames at about 12 fps.
- No change: send nothing, but keep the heartbeat and audio going. Every 1 s, send the whole frame anyway, so any lost part heals.
- Fallback: `LIBREMP_FULL_FRAMES=1` or `--full-frames` sends whole frames every time.

## Screen capture (`core/src/capture.rs`, `screencast.rs`, `x11_cursor.rs`)

- Every grabber shrinks the screen with an area average (`fit_4ch_to_rgb`), not nearest-neighbour. Nearest-neighbour dropped about half the pixel columns at 1920→1024, so thin text strokes vanished.

- The backend is picked from the OS and, on Linux, `XDG_SESSION_TYPE` / `WAYLAND_DISPLAY`. There is no manual picker.
- **Wayland** drives the xdg-desktop-portal ScreenCast + PipeWire session by hand, not through `xcap`'s recorder, for three reasons:
  - The portal hides the pointer unless the session asks for `cursor_mode = EMBEDDED`.
  - `xcap::Monitor::all()` needs XCB, so it fails without XWayland.
  - `persist_mode = 2` returns a restore token (saved next to the config), so the next run does not ask again.
- **Two silent traps on Wayland.** Both give a session that reports success but delivers no frames:
  - Connect PipeWire through the fd from `OpenPipeWireRemote`, not the default connection.
  - Keep the zbus `Connection` alive for the whole session.

  Also honour the buffer stride (it can be wider than `width*4`), and accept RGBx/BGRx/RGBA/BGRA/RGB/BGR.
- **PipeWire sends a frame only when the screen changes.** The grabber repeats the newest one.
- **Refusing screen sharing stops the cast.** Every fallback would project a black screen.
- **Fallback order on Wayland:** portal, then scrap under XWayland, then `xcap`. On KDE/GNOME, `xcap` can ask permission for every frame, so it comes last.
- **X11:** XShm frames never contain the pointer, so it is drawn in through XFixes (premultiplied ARGB, src-over).
- **Windows:** GDI BitBlt, with the cursor drawn with `DrawIconEx`.
- **Check capture without a projector:** `cargo run --release -p libremp-core --example portal_probe out.png`.

## Epson QR code (`core/src/qr.rs`)

- It is not a `WIFI:` code. It is a binary record, with every byte XORed with `0xE5`. Decode it with `quircs`; `rqrr` fails on this byte mode.
- **Layout.** A length byte, 2 header bytes, the IPv4 at offset 3, then the MAC. Wireless-only models add one byte before the MAC. Then come length-prefixed ASCII fields (the password, the SSID) and a `0x80` trailer.
- The header differs between models, so the parser **scans** for fields instead of using fixed offsets. A 12-hex-digit field is the Wi-Fi password (the MAC in uppercase hex; keep its case). A field containing `-` is the full SSID. The on-screen SSID is often cut short.
- Photos of a projector screen have glare and colour casts, which quircs' own binarization cannot handle. So decoding tries the raw image first, then adaptive local thresholding (integral image) at several window sizes.
- Tests with real projector payloads read them from `.env`. The fake-record tests cover both known layouts.

## Desktop app (`frontend/src-tauri`)

- The cast runs **in the app process**, on its own thread (`session::run`). Before, the app started a separate program, passed the Wi-Fi password on its command line (visible in `ps`), and had to guess the program's path.
- Events: `cast-event` (sharing, connecting, casting, reconnecting) and `cast-end` (with a typed error). Each is tagged with a UI-chosen id, so late events from an old cast are ignored.
- On quit, the app stops the cast (so the goodbye reaches the projector) and restores the previous Wi-Fi.
- **The camera runs in Rust (`nokhwa`), not through `getUserMedia`.** WebKitGTK's PipeWire camera path crashes the web process. `nokhwa` runs without default features, because its `mozjpeg` copy of libjpeg collided with libjpeg-turbo when linking with `rust-lld` (the default linker since Rust 1.90). MJPEG camera frames are decoded by core's libjpeg-turbo instead.
- **The Linux window** (`tauri.linux.conf.json`) has no GTK title bar. The app's toolbar drags the window and has its own close button; there is no double-click maximize.
  - The size is fixed with min = max = 920×760, not with `resizable: false`. On Wayland, `resizable: false` made GTK open the window 48 px bigger in each direction.
  - Hyprland floats windows whose size cannot change.
  - macOS and Windows keep their native title bar and `resizable: false`.
- **App id.** The Wayland window class is the program's file name, `libremp-app`. `scripts/install-linux.sh` keeps that name, so the desktop entry (`StartupWMClass`), the icon name and window-manager rules all match.
- `WEBKIT_DISABLE_DMABUF_RENDERER=1` is set on Linux. WebKitGTK's DMABUF renderer crashes on NVIDIA drivers.

## Wi-Fi (`core/src/wifi.rs`)

- Linux uses `nmcli`. A stale saved profile without a key gives "key-mgmt: property is missing". We delete that one profile and retry once. We never delete a profile on any other error.
- Windows uses `netsh`. The profile is written as a temporary XML file with `connectionMode=manual`, so Windows does not jump to the projector network on its own later. `netsh wlan connect` returns before the join finishes, so we poll `show interfaces`. Keys differ by locale (for example `Status`/`Profil`), so matching is loose.
- macOS uses `system_profiler` (the `airport` tool is gone since macOS 14) and `networksetup`. `networksetup` exits with 0 even when it fails, so we read its output.
- Restoring the old Wi-Fi does not block, so the app can quit while the OS reconnects.

## Saved projectors (`core/src/config.rs`)

- `<config dir>/libremp/projectors.json` (mode 0600) holds the name, SSID and last IP.
- Wi-Fi passwords go to the OS keychain (`keyring`, service `LibreMP`, keyed by SSID or name).
- Old files with plain `psk` fields are moved into the keychain when loaded. If the keychain is unavailable, the file is left as it is, so no password is lost.

## Linux distros (`scripts/install-linux.sh`, CI job `linux-distros`)

- **Packages.** The script reads `/etc/os-release` (`ID`, then `ID_LIKE`) and uses apt, dnf, pacman or zypper. On zypper it asks for `pkgconfig(...)` names, because openSUSE renamed `webkit2gtk3-devel` to `webkitgtk3-devel`. pacman runs without `-y`: a sync without a full upgrade can break Arch.
- **Toolchains.** The dependencies need Rust 1.89, and vite 7 needs Node 20.19. Ubuntu 24.04, Mint 22 and Debian 12 ship older ones. So the script adds rustup in `~/.cargo`, and puts Node 22 from nodejs.org (SHA-256 checked) in `~/.local/share/libremp/tools`, for this build only.
- **Old PipeWire.** `pipewire-rs` 0.9 does not compile against PipeWire older than 0.3.65 (Ubuntu 22.04, Mint 21, Pop 22.04 ship 0.3.48). That header has no `flags` in `spa_video_info_raw`, and its `modifier` is signed. `vendor/libspa` is libspa 0.9.2 with a `build.rs` check of `raw.h` that turns on `cfg(spa_old_video_raw)`. Both Cargo workspaces patch it in with `[patch.crates-io]`. Tested against the real jammy headers.
- **Wayland capture order.** The portal comes first, then xcap (wlroots screencopy, or one screenshot per frame). scrap under XWayland was removed from the Wayland chain, because it sees only X11 windows and always gave a black screen. If nothing works, the cast stops with a message that names the portal package to install.
- **Cursor mode.** We ask for `cursor_mode = EMBEDDED` only when `AvailableCursorModes` has it. The portal rejects modes the desktop lacks, and old portals have no cursor modes at all.
- **No NetworkManager.** Without `nmcli`, the app says so: join the projector's Wi-Fi by hand, then press Connect. Casting itself does not need NetworkManager.
- **Quitting.** SIGTERM, SIGINT and SIGHUP (kill, logout, Ctrl+C) end the app the normal way, so the exit handler says goodbye. A hard kill used to leave the projector frozen on the last picture.
- **Small screens.** If the screen is shorter than 840 logical pixels (1366×768 laptops, VMs), the window shrinks to fit and the list scrolls.
- **CI.** Each distro runs the install script (packages, build, install), then the core tests. Then it starts Xvfb at 1366×768, checks that the X11 capture reads a red xterm window, and checks that the app stays up for 20 s. `xsetroot -solid` does not paint the root window on Xvfb, so the test uses a window.


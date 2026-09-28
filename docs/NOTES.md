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

### The v9 dialect (older firmware)

The reference capture above is from a projector that reports EasyMP **version 11** (its `0x0015` blob has `0104 0b`). An Epson PowerLite 4650 reports **version 9** (`0104 09`) and does not accept the v11 `0x0101` login; it speaks an older, smaller dialect.

iProjection reports this projector as unsupported. Everything below comes from one capture of Epson's older Windows EasyMP client casting to it (client `10.240.62.181`, projector `10.240.61.255`, about 11 s of streaming). The capture is not in the repo; the login and `0x0015` bytes from it are in `core/tests/handshake_snapshot.rs` (`v9_*`). Byte meanings not listed here are unknown; those bytes are copied from the capture as-is.

- **Choosing the dialect.** We read the byte after the first `01 04` in the `0x0015` blob. The two projectors we have seen give `0x0b` and `0x09`. `protocol::is_v9` treats anything below 11 as v9. That cutoff is a guess from two data points, and so is using the first `01 04` match as the marker.
- **Login (`0x0004`, not `0x0101`).** A flat 95-byte payload: 48 bytes copied from the capture (including the netmask and gateway, below), the 6-byte MAC, a 16-byte keyword slot, the 4-byte projector IP, then 21 more bytes copied from the capture. We put the keyword in the slot after the MAC by analogy with v11; the capture had no keyword set. The reply is **`0x0005`, not `0x0102`**. We treat payload byte 30 `!= 0` as a refusal, as in v11. We have no v9 refusal capture, so that check is unconfirmed.
- **Heartbeats, and `0x0016`.** There is no `0x010E`/`0x0108`/`0x0110`. After login the projector sends a `0x000a` message (48-byte payload) about once a second. The client sends an empty `0x000a` about every 5 s. About 8.6 s after login the projector sends `0x0016` once. We guess it means "start streaming", because that is when the client starts sending video. `connect_v9` waits for `0x0016` before returning. `control_heartbeat` sends the right poll for each dialect.
- **Video.** The client opens TCP 3621 right after login and sends nothing on it until 60 ms after `0x0016`. There is **no `d0…` video-init packet**. The META display-config block is **24 bytes (`0xc8…`), not 46 (`0xcc…`)**, copied from the capture; `build_video_frame_v9` uses it. The EPRD tile layout looks the same as v11.
- **No silent audio.** The client opens a second 3621 socket but sends nothing on it. `session::stream` skips the silent-audio keepalive for v9.
- **Netmask and gateway in the login.** Payload bytes 13–20 are `ffff0000 0af00101`: 255.255.0.0 and 10.240.1.1. In the capture, 10.240.1.1 is the router (it sends ARP requests and answers DNS), and a /16 fits both hosts. The projector never sent these values, so the client most likely sends its own netmask and default gateway. We send the capture's values on every network (FIXME in `auth_payload_v9`). We don't know if the projector reads them; if a v9 projector on another network refuses the login, check this first.
- **Status.** Login and casting work on the PowerLite 4650 (tested 2026-09-27), after the tile length tag fix below.

## Video frames (EPRD)

Checked against `windows_perfect_stream.bin`: the reassembled video channel of the Windows session, 106 blocks. `core/tests/frame_builder.rs` rebuilds **all 105 JPEG blocks byte for byte**.

- **Block header.** `EPRD0600` + sender IP + msg id (0) + size. The META block's size is little-endian. The JPEG block's size is big-endian.
- **META.** A 46-byte display config. Windows sends it once, as the first block. We send it with every whole frame. This was proven on hardware by the old template, and the projector accepts it.
- **JPEG payload.** A big-endian `u32`, then per tile: a 16-byte descriptor (`x, y, w, h` as BE `u16`, flags `0x00000007`, a `u32` length tag) and the JPEG bytes. The first `u32` is the **tile count**. Earlier notes called it a "frame type" (4 = key, 3/1 = delta). That was wrong: it always equals the number of tiles.
- **Windows sends mostly partial frames.** More than 90 of the 105 blocks cover only the changed areas. Tiles are multiples of 16 (4:2:0 JPEG), at most 624×416, and are cut from the origin of each changed area.
- **The descriptor's last `u32` is the JPEG length, not a timestamp.** It is `0x90`, then the length in three 7-bit groups, low first, with the top bit set on the first two (`protocol::tile_len_tag`). It matches all 288 tiles in both captures (233 from v11, 55 from v9). The v11 projector ignores it, so our old fixed values worked there. The v9 projector checks it: a wrong value gets control message `0x0008` (payload `00000000 03000000 01000000…`), then the video socket is reset. Found by sending captured Windows JPEGs with our values (failed) and our JPEGs with Windows values (also failed).

### Sending only what changed (`core/src/session.rs`)

- Compare each new frame with the last one sent, on a 32 px grid.
- Changed cells in a row form runs. A run with the same columns as the run above it extends that area downward.
- More than 64 areas: send the whole frame (noise, a video playing). More than 8 areas: greedily merge the pair whose union wastes the least area, until 8 are left. More than 60 % of the screen: send the whole frame.
- Cut each area into tiles of at most 624×416. The JPEG size budget per tile is the same bytes-per-pixel as the whole-frame tiles.
- No change: send nothing, but keep the heartbeat and audio going. Every 1 s, send the whole frame anyway, so any lost part heals.
- Fallback: `LIBREMP_FULL_FRAMES=1` or `--full-frames` sends whole frames every time.

## Screen capture (`core/src/capture.rs`, `screencast.rs`, `x11_cursor.rs`)

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

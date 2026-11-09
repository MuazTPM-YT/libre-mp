# Epson iProjection 4.10 — Reverse-Engineering Evidence

Static analysis only. The installer and its programs were never run.
Tools: REA 6.8.0 (`rea-agents`) with Ghidra 12.1.2, 7-Zip, GNU strings.

Each finding is marked:

- **Observed** — read directly from the binary or the decompiled code.
- **Inferred** — a conclusion from observed facts. Not proven.
- **Unknown** — not answered yet.

## 1. What the installer holds

| Item | Value |
|---|---|
| Installer | `iProV410Win_Web.exe`, SHA-256 `0fc884cbdeb16e63704bcc60393496ff6401a24a277bcce6d7178e185ce47244` |
| Signer / version | SEIKO EPSON CORPORATION, "Epson iProjection Ver.4.10", 4.1.0.0 |
| Layout | PE stub → CAB resource (`.rsrc/1041/5428/131`) → `InstDataX64/Epson iProjection Ver.4.10.msi` (also ARM64, x86, and an old InstallShield copy) |

Programs that matter (from the x64 MSI, names from each file's version resource):

| File | Role (observed from class names and strings) |
|---|---|
| `EMP_PJCON.dll` (SHA-256 `4c70d248…4581`) | **The protocol.** All EEMP and EPRD code, connect, keep-alive, capabilities, picture encoders, AES, multi-PC, moderator, WebRTC glue. |
| `EMP_NMANG.dll` (SHA-256 `c18694f8…29ab`) | Network manager: projector search (`CEconSearchTcpThread`), "show keyword on projector" (`CEconDisplayKeywordTcpThread`), Wi-Fi join (`CWirelessLANVista`), capability logging. |
| `EMP_ENGN.dll`, `EMP_MPPCore.dll`, `EMP_MPP.exe` | Engine, app core, user interface. |
| `WebRTCManager.dll` | WebRTC transport (newer projectors). |
| `EMP_VHID*.sys`, `EMP_IPAU.sys`, `EMP_NSAU.sys` | Virtual HID (remote mouse/pen) and virtual audio drivers. |

`EMP_PJCON.dll` decompiled in full: 11 695 functions written, 746 external or bodiless
(REA journal, target SHA-256 `4c70d248…4581`, analysis profile `bf9186f7…a7`).

## 2. Findings that confirm LibreMP

1. **EEMP header** — Observed. `FUN_18021c410` accepts a message only when bytes 0–3 are `EEMP`
   and bytes 4–5 are `01`. Builders write `EEMP` `0100`, the sender IPv4 (only when the socket
   family is IPv4), the command (u32 LE), then the payload length (u32 LE). Same as
   `core/src/protocol.rs::eemp_header`.
2. **Capability `0x0015` is the source of LibreMP's magic numbers** — Observed in the RESEARCHLAB
   capture, decoded with the type/length layout seen in the code:
   - Type 3 = `00 04 00 03` → 1024×768 (u16 LE). This is LibreMP's `STREAM_W`/`STREAM_H`.
   - Type 5 = list of picture encodings, 22 bytes each: encoding, sub-encoding, align, rect unit,
     rect min, **rect max**, throughput. Encoding `07/0b`: unit 8×8, min 16×16,
     **max 624×416**. This is LibreMP's `MAX_TILE_W`/`MAX_TILE_H`. Encodings `07/09`, `07/0d`
     and `00/00` allow up to 656×416.
3. **Login reply refusal byte, goodbye `0x0104`/`0x0105`, registration `0x0002`** — consistent
   with the code paths named `SendEconDisconnect`, `SendEconConnectCommand`. No contradiction found.

## 3. Findings that are new

1. **The connect (login) body carries the stream size and pixel format** — Observed.
   `CEasyCommand::MakeReqConnect` (`FUN_18021c210`) fills, at payload offsets:

   | Payload offset | Size | Meaning | RESEARCHLAB value |
   |---|---|---|---|
   | 4 | 1 | connect type (0, 2 or 3 by flags) | `00` |
   | 5 | 8 | 8 bytes from the caller | `38 0f 00 00 00 00 00 00` |
   | 22 | 1 | a timeout in ms ÷ 1000 | `0f` (15 s) |
   | 24 | 2+2 | **client resolution**, u16 LE | 1024, 768 |
   | 28 | 13 | **pixel format**, RFB style: bpp 32, depth 32, big-endian 0, true colour 1, max 255/255/255, shifts | `20 20 00 01 ff 00 ff 00 ff 00 00 08 10` |
   | 44 | 2 | two flag bytes | `01 0e` |

   A 16-bit format is also built (`bpp 16`, max 31/63/31, shifts 11/5/0).
   LibreMP sends these bytes as a fixed copy. **Inferred:** a projector that reports a larger
   panel can be asked for a larger stream here. The old 1280×800 test failed. Unknown: whether
   it failed because the capability did not allow it, or because another field disagreed.
2. **Projector function flags (capability type 8)** — Observed bit layout from the logger
   `FUN_180202d00`. RESEARCHLAB sends `9f 03 f0 d5 00 00 00 00`:

   | Byte.bit | Flag | RESEARCHLAB |
   |---|---|---|
   | 0.0 | InterruptConnection | 1 |
   | 0.2 | PauseAndResume | 1 |
   | 0.3 | RetryConnection | 1 |
   | 0.5 | RectWithoutHeader | 0 |
   | 0.7 | UniDirectionalTCPConnect | 1 |
   | 1.0 | DES | 1 |
   | 1.1 | AES | 1 |
   | 1.2 | AES-EP-CTR | 0 |
   | 2.0 | StretchJPEG | 0 |
   | 2.3 | WhiteboardSharing | 1 |
   | 2.4 | WebRemote | 1 |
   | 2.6 | DisplayKeyword | 1 |
   | 2.7 | ConnectionIdentifier | 1 |
   | 8.0 | BiDirectionalTCPKeepAlive | not sent (record is 8 bytes) |
   | 8.6 | QRConnect | not sent |

3. **Encryption is optional** — Observed. `CProjector::GetEncryptType` returns 0 (none) unless the
   user turned encryption on. When on, it picks AES-EP-CTR (3), AES (1) or DES (2) from the flags
   above. The RESEARCHLAB capture is plain text, so encryption was off there. Classes:
   `CEncoderAES_Win`, `CEncoderAESEPCTR_Win`, BCrypt key-pair + symmetric APIs, and an embedded
   `-----BEGIN PUBLIC KEY-----`. Unknown: the key exchange.
4. **EEMP keep-alive, command `0x000A`** — Observed. `CKeepAlive::Exec` (`FUN_1802456e0`) sends a
   bare 20-byte `EEMP0100` header with command 10 and length 0, and times out the session when the
   projector's keep-alive stops. **Inferred:** used with `BiDirectionalTCPKeepAlive`, which
   RESEARCHLAB does not report. LibreMP does not send it.
5. **New capability list format** — Observed in `FUN_18021bc80`: u32 total length, then records of
   u32 LE type + u32 LE size + data, ended by type `0xFFFFFFFF`. Known types: 4–8, 0x0E–0x13,
   0x1B, 0x1E, 0x21, 0x25, 0x26, 0x28, 0x29. RESEARCHLAB uses the older form: u8 type + u8 length.
   **Inferred:** newer projectors (the `NSC10` classes) use the u32 form. LibreMP parses neither.
6. **Picture encodings** — Observed classes: JPEG, LZO, ZLIB, RAW16, RAW32, RAWEX16, RAWEX, EIC.
   LibreMP sends JPEG only, which RESEARCHLAB accepts.
7. **Features LibreMP lacks** — Observed method names in `EMP_PJCON.dll`: multi-PC split screen
   and layout (`NextMalutiLayoutCmd`), moderator (`RequestModeration`), screen delivery to other
   PCs, user lists, PC-to-PC messages (`U2UCommand`), audio (`SendPconAudioData`, real audio, not
   silence), movie mode (`SendReqMovie`), pen/mouse forwarding (`SendStartFwdPenData`, HID
   drivers), ESC/VP.net projector control (`SendESCVPCommand`), projector logs, WebRTC
   (`SendWebRTCOfferSDP`), dynamic resolution change (`SendUpdateClientResolution`), TCP search
   and "display keyword" request (`EMP_NMANG.dll`).

## 4. Still unknown

- **Projector Keyword placement.** Not yet found in the code. LibreMP's placement (16 bytes after
  the MAC) still comes only from Rhino Security Labs' tool.
- The AES key exchange and which bytes are encrypted.
- The meaning of the 8 bytes at connect payload offset 5, and of the flag bytes at offset 44.
- Whether RESEARCHLAB accepts a different client resolution when the capability allows it.

## 5. How to repeat

```bash
7z x iProV410Win_Web.exe 'InstDataX64/*'                     # nested CAB
7z x "InstDataX64/Epson iProjection Ver.4.10.msi"              # files named by MSI key
npx -y rea-agents@6.8.0 decompile-all EMP_PJCON.dll --output pjcon_c --provider ghidra
npx -y rea-agents@6.8.0 inspect-pe-imports EMP_PJCON.dll --format json
```

Function files: `pjcon_c/functions/<address>.c`. Addresses cited above are Ghidra addresses
(image base `0x180000000`).

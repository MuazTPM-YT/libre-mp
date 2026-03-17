import socket
import time
import struct
from . import config
from . import payloads

class ProtocolError(Exception):
    pass

class EpsonEasyMPClient:
    def __init__(self, projector_ip=None, my_ip=None):
        self.projector_ip = projector_ip or config.PROJECTOR_IP
        self.my_ip = my_ip or config.get_local_ip()
        
        # Sockets
        self.s_auth = None        # Port 3620
        self.s_video = None       # Port 3621 - FIRST connection (carries video data)
        self.s_video_aux = None   # Port 3621 - SECOND connection (carries zero buffers)

        # State
        self.first_frame = True
        self.warmup_sent = False
    
    def connect_and_negotiate(self):
        """
        Full connection sequence matching the Windows PCAP exactly:
        
        1. Register + Authenticate on port 3620
        2. Post-auth handshake (0x010E/0x0108/0x0109/0x0110)
        3. Open two video channels on port 3621
        4. WAIT for projector's 0x0016 on port 3620 (~1.5s delay)
        5. Send warmup zero buffers on aux channel
        6. Start streaming
        """
        print(f"[*] Starting deterministic negotiation sequence with {self.projector_ip}...")
        try:
            # 1. Authentication (TCP 3620)
            self._authenticate_session()
            # 2. Complete post-auth handshake on 3620
            self._complete_auth_handshake()
            # 3. Open dual video channels (TCP 3621)
            self._open_video_channels()
            # 4. Wait for projector's 0x0016 streaming confirmation on 3620
            self._wait_for_streaming_signal()
            
            # NOTE: Warmup buffers are now sent AFTER the first video frame 
            # in send_video_frame() to match Windows PCAP timing.

            print("\n[+] BINGO! Connection Fully Established and Ready for Video Stream!")
            return True

        except Exception as e:
            print(f"[-] Negotiation failed: {e}")
            import traceback
            traceback.print_exc()
            self.disconnect()
            return False

    def _authenticate_session(self):
        print(f"[*] 1. Opening Authentication Channel (Port 3620)...")
        self.s_auth = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_auth.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.s_auth.settimeout(5)
        self.s_auth.connect((self.projector_ip, config.PORT_CONTROL))

        # --- REGISTRATION PHASE ---
        print("[*]    Sending 68-byte Registration Packet...")
        reg_payload = payloads.get_registration_payload(self.my_ip)
        self.s_auth.sendall(reg_payload)
        
        try:
            resp1 = self.s_auth.recv(1024)
            print(f"[+]    Registration Resp 1: {len(resp1)} bytes -> {resp1.hex()[:32]}...")
            try:
                self.s_auth.settimeout(1)
                resp2 = self.s_auth.recv(1024)
                print(f"[+]    Registration Resp 2: {len(resp2)} bytes -> {resp2.hex()[:32]}...")
            except socket.timeout:
                pass
        except Exception as e:
            print(f"[-]    Registration failed: {e}")
            raise

        # Windows client explicitly closes the registration connection 
        # and opens a NEW connection for the actual authentication!
        print("[*]    Closing Registration channel, opening Auth channel...")
        self.s_auth.close()
        time.sleep(0.1)

        self.s_auth = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_auth.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.s_auth.settimeout(5)
        self.s_auth.connect((self.projector_ip, config.PORT_CONTROL))

        # --- AUTHENTICATION SEQUENCE ---
        print("[*]    Sending 264-Byte Full Auth Request...")
        auth_payload = payloads.get_auth_payload_full(self.my_ip, self.projector_ip)
        
        print(f"[*]    Payload length: {len(auth_payload)} bytes")
        self.s_auth.sendall(auth_payload)
        
        print("[*]    Waiting for authentication response...")
        try:
            auth_resp = self.s_auth.recv(1024)
            print(f"[+]    Auth Resp: {len(auth_resp)} bytes -> {auth_resp.hex()[:32]}...")
            
            if len(auth_resp) >= 50:
                status_byte = auth_resp[50]
                print(f"[+]    Auth Status Byte: 0x{status_byte:02x}")
                if len(auth_resp) == 296:
                    print("[+]    Perfect 296-byte Auth Response received! We are IN.")
        except socket.timeout:
            print("[-]    Auth response timed out")
        except Exception as e:
            print(f"[-]    Auth error: {e}")

        print("[+]    Authentication phase complete.")

    def _complete_auth_handshake(self):
        """
        After the 296-byte auth response, the projector sends additional 
        handshake packets. The client MUST respond to ONLY the FIRST 0x010E 
        command with a 0x0108 response (348 bytes), then wait for 0x0110.
        
        PCAP Windows sequence:
          Frame 123: Projector -> 360 bytes (cmd 0x010E)  <-- respond
          Frame 124: Client    -> 348 bytes (cmd 0x0108)
          Frame 125: Projector -> 348 bytes (cmd 0x0109)
          Frame 129: Projector -> 360 bytes (cmd 0x010E)  <-- DO NOT respond!
          Frame 134: Projector -> 20 bytes  (cmd 0x0110, "ready")
        """
        print("[*]    Completing post-auth handshake on Port 3620...")
        self.s_auth.settimeout(3)
        
        ready_received = False
        already_responded = False  # Only respond to FIRST 0x010E
        
        try:
            for attempt in range(10):
                try:
                    data = self.s_auth.recv(4096)
                    if not data:
                        break
                    
                    # The projector may send two 180-byte 0x010E packets concatenated 
                    # as one 360-byte recv, or as separate 180-byte packets.
                    # Parse all EEMP messages in the received data.
                    offset = 0
                    while offset + 20 <= len(data):
                        if data[offset:offset+8] != b'EEMP0100':
                            break
                        
                        cmd = struct.unpack('<I', data[offset+12:offset+16])[0]
                        payload_len = struct.unpack('<I', data[offset+16:offset+20])[0]
                        msg_len = 20 + payload_len
                        
                        print(f"[+]    Post-auth recv: cmd=0x{cmd:04x}, {msg_len} bytes")
                        
                        if cmd == 0x010E and not already_responded:
                            response = self._build_0x0108_response()
                            self.s_auth.sendall(response)
                            already_responded = True
                            print(f"[+]    Sent 0x0108 response: {len(response)} bytes")
                        
                        elif cmd == 0x010E and already_responded:
                            print(f"[*]    Ignoring subsequent 0x010E (no response needed)")
                        
                        elif cmd == 0x0110:
                            print("[+]    Received 0x0110 'Ready to Stream' signal!")
                            ready_received = True
                            break
                        
                        offset += msg_len
                    
                    if ready_received:
                        break
                    
                except socket.timeout:
                    break
                except Exception as e:
                    print(f"[*]    Post-auth error: {e}")
                    break
        except Exception as e:
            print(f"[*]    Post-auth handshake note: {e}")
        
        self.s_auth.settimeout(5)
        
        if ready_received:
            print("[+]    Post-auth handshake complete. Projector is ready!")
        else:
            print("[*]    Post-auth handshake complete (no explicit ready signal, continuing).")


    def _build_0x0108_response(self):
        """
        Build the exact 348-byte post-auth client response (pcap Frame 124).
        Uses the exact bytes from the pcap, only replacing client IP at known offsets.
        """
        # Exact 348-byte payload from PCAP Frame 124, with 192.168.88.2 (c0a85802) as IP
        pcap_hex = (
            "45454d5030313030c0a858020801000048010000"
            "0001000000000000000000000000000000000000"
            "00000000000000000000000000000000000000000000000000000000"
            "1401000005000000380000000200000004000000"
            "c0a858020c00000004000000010000000100000004000000"
            "500043000b00000004000000000000001c00000000000000"
            "07000000440000000100000005000000380000000200000004000000"
            "c0a858020c00000004000000010000000100000004000000"
            "500043000b00000004000000000000001c00000000000000"
            "08000000800000000400000005000000380000000200000004000000"
            "c0a858020c00000004000000010000000100000004000000"
            "500043000b00000004000000010100001c00000000000000"
            "000000000c000000020000000400000002000000"
            "000000000c000000020000000400000003000000"
            "000000000c000000020000000400000004000000"
        )
        raw = bytearray.fromhex(pcap_hex)
        
        # Replace all occurrences of the pcap's client IP with our actual IP
        ip_bytes = socket.inet_aton(self.my_ip)
        pcap_ip = socket.inet_aton('192.168.88.2')
        
        # Known IP offsets from pcap: 8, 56, 112, 168, 224
        for i in range(len(raw) - 3):
            if raw[i:i+4] == pcap_ip:
                raw[i:i+4] = ip_bytes
        
        return bytes(raw)

    def _open_video_channels(self):
        """
        Open the primary TCP connection to port 3621.
        The AUX channel is opened later (after first frame headers) to match Windows.
        """
        print(f"[*] 2. Opening Video Channel (Port 3621)...")
        time.sleep(0.3)
        
        # --- Connection 1: VIDEO DATA (byte 28 = 0x00) ---
        self.s_video = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_video.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.s_video.connect((self.projector_ip, config.PORT_VIDEO))
        self.s_video.settimeout(None)
        
        ctrl_init = payloads.get_video_init_payload_ctrl(self.my_ip)
        self.s_video.sendall(ctrl_init)
        print(f"[+]    Video channel OPEN (EPRD init byte28=0x00)")
        
        self.first_frame = True
        self.warmup_sent = False

    def _wait_for_streaming_signal(self):
        """
        Wait for the projector to send 0x0016 on port 3620.
        
        PCAP evidence (both tls.pcapng and archtest7.pcapng):
          - Source: 192.168.88.1 (PROJECTOR) -> 192.168.88.2 (CLIENT)
          - cmd=0x0016, 68 bytes total (20 header + 48 payload)
          - Arrives ~1.5s after video channels are opened
        
        The Windows client only TCP-ACKs this — it does NOT send any response.
        """
        print("[*] 3. Waiting for projector 0x0016 streaming signal...")
        self.s_auth.settimeout(10)  # Give projector time to process channels
        
        try:
            data = self.s_auth.recv(4096)
            if data and len(data) >= 20 and data[:8] == b'EEMP0100':
                cmd = struct.unpack('<I', data[12:16])[0]
                print(f"[+]    Received cmd=0x{cmd:04x} ({len(data)} bytes) from projector")
                if cmd == 0x0016:
                    print("[+]    Projector confirmed streaming ready (0x0016)!")
                else:
                    print(f"[*]    Expected 0x0016, got 0x{cmd:04x}. Continuing anyway...")
            elif data:
                print(f"[*]    Received {len(data)} bytes (not EEMP). Continuing...")
            else:
                print("[*]    No data received. Continuing anyway...")
        except socket.timeout:
            print("[*]    No 0x0016 received within timeout. Continuing anyway...")
        except Exception as e:
            print(f"[*]    Error waiting for streaming signal: {e}")
        
        self.s_auth.settimeout(5)

    def _send_aux_bundle(self, size: int):
        """
        Send an auxiliary packet (zeros or keepalive) matching Windows TCP segmentation:
        1. Send the 5-byte header (0xC9 + length) as a distinct packet.
        2. Send the actual payload data.
        """
        if not self.s_video_aux:
            return
            
        hdr = payloads.get_aux_header(size)
        self.s_video_aux.sendall(hdr)
        
        # Windows often sends the data immediately after, but the 5-byte header 
        # is definitely its own TCP segment in the PCAP.
        data = b'\x00' * size
        self.s_video_aux.sendall(data)

    def _send_warmup_buffers(self):
        """
        Send three zero-buffer warmup packets on the AUX channel.
        PCAP shows these are sent AFTER the first video frame starts.
        Sizes: 7276, 2646, 1764 bytes (all zeros).
        """
        if self.warmup_sent:
            return
            
        print("[*] 4. Sending warmup buffers on aux channel...")
        warmup_sizes = [7276, 2646, 1764]
        for i, size in enumerate(warmup_sizes):
            self._send_aux_bundle(size)
            print(f"[+]    Warmup buffer {i+1}: {size} zeros on aux channel")
            time.sleep(0.05)
        
        self.warmup_sent = True
        print("[+]    Warmup complete.")

    def send_video_frame(self, x_offset, y_offset, width, height, jpeg_bytes):
        if not self.s_video:
            print("[-] Cannot send video frame. Video socket not initialized!")
            return False
            
        def _send_chunked(sock, data, chunk_size=1460):
            """Send data in chunks matching Ethernet MSS (1460 bytes)."""
            total_sent = 0
            while total_sent < len(data):
                chunk = data[total_sent:total_sent+chunk_size]
                sock.sendall(chunk)
                total_sent += len(chunk)
                
        try:
            if self.first_frame:
                meta = payloads.get_display_config_meta(config.PROJECTOR_DISPLAY_WIDTH, config.PROJECTOR_DISPLAY_HEIGHT)
                meta_hdr = payloads.get_eprd_meta_header(self.my_ip, len(meta))
                
                frame_hdr = payloads.get_frame_header(4, x_offset, y_offset, width, height)
                jpeg_payload_size = len(frame_hdr) + len(jpeg_bytes)
                jpeg_hdr = payloads.get_eprd_jpeg_header(self.my_ip, jpeg_payload_size)

                print(f"[*]    Sending first frame: (meta_hdr=20 + meta=46 + jpeg_hdr=20 + frame_hdr=20 + jpeg={len(jpeg_bytes)})")
                
                # 1. Meta header
                self.s_video.sendall(meta_hdr)
                
                # 2. Meta data
                self.s_video.sendall(meta)
                
                # 3. JPEG header
                self.s_video.sendall(jpeg_hdr)
                
                # 4. Frame Type (4 bytes)
                self.s_video.sendall(frame_hdr[:4])
                
                # 5. Region info (16 bytes)
                self.s_video.sendall(frame_hdr[4:])
                
                # LAZY OPEN AUX CHANNEL (Matches Windows timing)
                if not self.s_video_aux:
                    print("[*]    Opening Aux channel now...")
                    self.s_video_aux = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                    self.s_video_aux.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
                    self.s_video_aux.connect((self.projector_ip, config.PORT_VIDEO))
                    self.s_video_aux.settimeout(None)
                    
                    data_init = payloads.get_video_init_payload_data(self.my_ip)
                    self.s_video_aux.sendall(data_init)
                    print(f"[+]    Aux channel OPEN (byte28=0x01)")

                # 6. JPEG Data
                _send_chunked(self.s_video, jpeg_bytes, chunk_size=1460)
                
                print(f"[+]    First frame data in flight. Triggering late warmup...")
                self._send_warmup_buffers()
                
                self.first_frame = False
            else:
                frame_hdr = payloads.get_frame_header(3, x_offset, y_offset, width, height)
                jpeg_payload_size = len(frame_hdr) + len(jpeg_bytes)
                jpeg_hdr = payloads.get_eprd_jpeg_header(self.my_ip, jpeg_payload_size)

                self.s_video.sendall(jpeg_hdr)
                self.s_video.sendall(frame_hdr[:4])
                self.s_video.sendall(frame_hdr[4:])
                _send_chunked(self.s_video, jpeg_bytes, chunk_size=1460)
                
            return True
        except Exception as e:
            import errno
            print(f"[-] Stream interrupted: {e}")
            print(f"[-]    Error type: {type(e).__name__}")
            if hasattr(e, 'errno'):
                print(f"[-]    Errno: {e.errno} ({errno.errorcode.get(e.errno, 'unknown')})")
            if self.first_frame:
                print(f"[-]    Failed on FIRST frame (buffer size would be {len(jpeg_bytes) + 106} bytes)")
            return False

    def send_keepalive(self):
        """
        Send a zero-buffer keepalive on the AUX channel.
        
        PCAP comparison (tshark analysis):
          Windows (tls.pcapng): sends 2646-byte zero buffers after each video frame
          Linux old (archtest7.pcapng): sent c900000000 (0-byte) — WRONG
        
        Must match Windows: 2646-byte zero buffers (same as warmup #2 size).
        """
        if self.s_video_aux:
            try:
                # Windows sends a 2646-byte zero buffer after each video frame
                # to keep the AUX channel alive and happy.
                self._send_aux_bundle(2646)
            except Exception:
                pass

    def disconnect(self):
        print("\n[*] Disconnecting client...")
        try:
            if self.s_auth:
                self.s_auth.close()
        except Exception as e:
            print(f"[*] Error closing auth socket: {e}")

        for sock in [self.s_video, self.s_video_aux]:
            if sock:
                try: sock.close()
                except: pass

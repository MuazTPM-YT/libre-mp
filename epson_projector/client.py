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
    
    def connect_and_negotiate(self):
        print(f"[*] Starting deterministic negotiation sequence with {self.projector_ip}...")
        
        try:
            # 1. Authentication (TCP 3620)
            self._authenticate_session()
            
            # 2. Complete post-auth handshake on 3620
            self._complete_auth_handshake()
            
            # 3. Open dual video channels (TCP 3621)
            self._open_video_channels()
            
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

        self.s_auth.settimeout(5)

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
                            # Don't break - continue to wait for 0x0016 status
                        
                        elif cmd == 0x0016:
                            print("[+]    Received 0x0016 'Display Pipeline Ready' status!")
                            status_received = True
                            break
                        
                        offset += msg_len
                    
                    if status_received:
                        break
                    
                except socket.timeout:
                    if ready_received:
                        # We got 0x0110 but 0x0016 didn't arrive yet
                        # Keep waiting (projector can take ~1.5s)
                        print("[*]    Waiting for projector display pipeline...")
                        continue
                    break
                except Exception as e:
                    print(f"[*]    Post-auth error: {e}")
                    break
        except Exception as e:
            print(f"[*]    Post-auth handshake note: {e}")
        
        self.s_auth.settimeout(5)
        
        if status_received:
            print("[+]    Post-auth handshake complete. Display pipeline confirmed ready!")
        elif ready_received:
            print("[+]    Post-auth handshake complete. Projector ready (no 0x0016 status, continuing).")
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
        Open TWO TCP connections to port 3621:
        
        CORRECTED from pcap re-analysis:
        - Connection 1 (s_video): byte28=0x00 — carries ACTUAL VIDEO DATA
        - Connection 2 (s_video_aux): byte28=0x01 — carries ZERO BUFFER padding
        
        Then send three zero-buffer warmup packets on Connection 2.
        """
        print(f"[*] 2. Opening Video Channels (Port 3621)...")
        time.sleep(0.3)
        
        # --- Connection 1: VIDEO DATA (byte 28 = 0x00) ---
        self.s_video = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_video.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.s_video.settimeout(5)
        self.s_video.connect((self.projector_ip, config.PORT_VIDEO))
        
        ctrl_init = payloads.get_video_init_payload_ctrl(self.my_ip)
        self.s_video.sendall(ctrl_init)
        print(f"[+]    Video channel OPEN (EPRD init byte28=0x00, carries video)")
        
        time.sleep(0.3)
        
        # --- Connection 2: ZERO BUFFER AUX (byte 28 = 0x01) ---
        self.s_video_aux = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_video_aux.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.s_video_aux.settimeout(5)
        self.s_video_aux.connect((self.projector_ip, config.PORT_VIDEO))
        
        data_init = payloads.get_video_init_payload_data(self.my_ip)
        self.s_video_aux.sendall(data_init)
        print(f"[+]    Aux channel OPEN (EPRD init byte28=0x01, carries zero buffers)")
        
        time.sleep(0.3)
        
        # --- Zero-buffer warmup on AUX connection ---
        # PCAP: three zero buffers before video starts
        warmup_sizes = [7276, 2646, 1764]
        for i, size in enumerate(warmup_sizes):
            buf = payloads.get_zero_buffer(size)
            self.s_video_aux.sendall(buf)
            print(f"[+]    Warmup buffer {i+1}: {size} zeros on aux channel")
            time.sleep(0.15)

        print("[+]    Video channels initialized. Ready to stream!")
        self.first_frame = True

    def send_video_frame(self, x_offset, y_offset, width, height, jpeg_bytes):
        """
        Wraps JPEG data in the Epson EPRD protocol and sends on the 
        FIRST video connection (s_video, byte28=0x00).
        
        Protocol per frame:
          1. EPRD meta header (20 bytes) + Display config (46 bytes) [first frame only]
          2. EPRD jpeg header (20 bytes) with JPEG size
          3. Frame type (4 bytes): 0x04=keyframe, 0x03=delta
          4. Region descriptor (16 bytes): x, y, w, h + flags + timestamp
          5. JPEG data
        """
        if not self.s_video:
            print("[-] Cannot send video frame. Video socket not initialized!")
            return False
            
        try:
            # First frame: send display configuration meta
            if self.first_frame:
                meta = payloads.get_display_config_meta(
                    config.PROJECTOR_DISPLAY_WIDTH, 
                    config.PROJECTOR_DISPLAY_HEIGHT
                )
                meta_header = payloads.get_eprd_meta_header(self.my_ip, len(meta))
                # Send as separate packets (matching Windows pcap frames 182+183)
                self.s_video.sendall(meta_header)
                self.s_video.sendall(meta)
            
            # EPRD header with payload size (pcap frame 184)
            # Size field = frame_type(4) + region(16) + JPEG data
            # This is the total size of ALL sub-frame data in this EPRD block
            payload_size = 4 + 16 + len(jpeg_bytes)
            jpeg_header = payloads.get_eprd_jpeg_header(self.my_ip, payload_size)
            self.s_video.sendall(jpeg_header)
            
            # Frame type - always keyframe (4) since we send full-screen JPEG
            # Windows only uses type 3 for partial region deltas, not full screen
            frame_type = 4
            frame_hdr = payloads.get_frame_header(
                frame_type, x_offset, y_offset, width, height
            )
            # Send frame_type (4 bytes) and region (16 bytes) separately 
            # matching pcap frames 185 + 186
            self.s_video.sendall(frame_hdr[:4])   # frame type
            self.s_video.sendall(frame_hdr[4:])   # region descriptor
            
            # JPEG data (pcap frame 187+)
            self.s_video.sendall(jpeg_bytes)
            
            if self.first_frame:
                self.first_frame = False
            
            return True
        except Exception as e:
            print(f"[-] Stream interrupted: {e}")
            return False

    def send_keepalive(self):
        """Send a zero-buffer keepalive on the AUX channel between frames."""
        if self.s_video_aux:
            try:
                buf = payloads.get_zero_buffer(1764)
                self.s_video_aux.sendall(buf)
            except Exception:
                pass

    def disconnect(self):
        """Tears down all connections safely."""
        print("[*] Disconnecting client...")
        for sock in [self.s_auth, self.s_video, self.s_video_aux]:
            if sock:
                try: sock.close()
                except: pass

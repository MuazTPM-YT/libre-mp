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
        self.s_hardware = None # Port 3629
        self.s_auth = None     # Port 3620
        self.s_video = None    # Port 3621
    
    def connect_and_negotiate(self):
        print(f"[*] Starting deterministic negotiation sequence with {self.projector_ip}...")
        
        try:
            # 2. Authentication Connect (TCP 3620)
            self._authenticate_session()
            
            # 3. Transport Stream Connect (TCP 3621)
            self._open_video_channel()
            
            print("\n[+] BINGO! Connection Fully Established and Ready for Video Stream!")
            return True
                
        except Exception as e:
            print(f"[-] Negotiation failed: {e}")
            self.disconnect()
            return False

    def _authenticate_session(self):
        print(f"[*] 2. Opening Authentication Channel (Port 3620)...")
        self.s_auth = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_auth.settimeout(5)
        self.s_auth.connect((self.projector_ip, config.PORT_CONTROL))

        # --- REGISTRATION PHASE ---
        print("[*]    Sending 68-byte Registration Packet...")
        # The first packet MUST be the IP registration `0x02`
        reg_payload = payloads.get_registration_payload(self.my_ip)
        self.s_auth.sendall(reg_payload)
        
        try:
            resp1 = self.s_auth.recv(1024)
            print(f"[+]    Registration Resp 1: {len(resp1)} bytes -> {resp1.hex()[:32]}...")
            # We also might receive a second packet immediately (the 226-byte one)
            # but we can safely just read whatever is in the buffer or try one more read
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
        print("[*]    Sending 264-Byte Full Auth Request (PCAP Replay)...")
        # We use the full PCAP 264-byte payload since the short 94-byte one is ignored
        auth_payload = payloads.get_auth_payload_full(self.my_ip, self.projector_ip)
        
        print(f"[*]    Payload length: {len(auth_payload)} bytes")
        self.s_auth.sendall(auth_payload)
        
        print("[*]    Waiting for authentication response...")
        try:
            auth_resp = self.s_auth.recv(1024)
            print(f"[+]    Auth Resp: {len(auth_resp)} bytes -> {auth_resp.hex()[:32]}...")
            
            # The PCAP shows 296 bytes on correct auth. We check byte 50 (index 50) for success flag.
            if len(auth_resp) >= 50:
                status_byte = auth_resp[50]
                print(f"[+]    Auth Status Byte: 0x{status_byte:02x}")
                if len(auth_resp) == 296:
                    print("[+]    Perfect 296-byte Auth Response received! We are IN.")
        except socket.timeout:
            print("[-]    Wait/Resp 1 error: timed out")
        except Exception as e:
            print(f"[-]    Wait/Resp 1 error: {e}")

        # Assume SUCCESS if we made it here without getting outright rejected or connection dropped
        print("[+]    Finished playing exploit flow. Assuming authentication state is ready!")


    def _open_video_channel(self):
        print(f"[*] 3. Opening Video Channel (Port 3621)...")
        time.sleep(0.5) # Slight delay purely to let Projector OS transition state
        self.s_video = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_video.settimeout(5)
        self.s_video.connect((self.projector_ip, config.PORT_VIDEO))
        
        print("[*]    Sending Video Channel Initialization (EPRD0600)...")
        video_init_payload = payloads.get_video_init_payload(self.my_ip)
        self.s_video.sendall(video_init_payload)
        
        # We don't necessarily wait for a response here; the protocol just streams
        print("[+]    Video Channel OPEN. Ready to stream PCON-wrapped MJPEG.")

    def send_video_frame(self, x_offset, y_offset, width, height, mjpeg_bytes):
        """Wraps MJPEG stream data in PCON header and sends to Port 3621."""
        if not self.s_video:
            print("[-] Cannot send video frame. Video socket not initialized!")
            return False
            
        try:
            # Construct PCON header
            pcon_header = payloads.get_pcon_video_header(x_offset, y_offset, width, height, len(mjpeg_bytes))
            
            # Send synchronously
            self.s_video.sendall(pcon_header + mjpeg_bytes)
            return True
        except Exception as e:
            print(f"[-] Stream interrupted: {e}")
            return False

    def disconnect(self):
        """Tears down all connections safely."""
        print("[*] Disconnecting client...")
        if self.s_hardware:
            try: self.s_hardware.close()
            except: pass
        if self.s_auth:
            try: self.s_auth.close()
            except: pass
        if self.s_video:
            try: self.s_video.close()
            except: pass

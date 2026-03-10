import socket
import time
import struct
from . import config
from . import payloads

class EpsonEasyMPClient:
    def __init__(self, projector_ip=None, my_ip=None):
        self.projector_ip = projector_ip or config.PROJECTOR_IP
        self.my_ip = my_ip or config.get_local_ip()
        
        # Sockets
        self.s_control = None
        self.s_video = None
        self.s_wake = None
        
        # Payloads
        self.payload_control_p1_part1 = payloads.get_control_payload(self.my_ip)
        self.payload_control_p1_part2 = payloads.get_control_payload_phase_1_part_2(self.my_ip)
        self.payload_control_p2 = payloads.get_control_payload_phase_2(self.my_ip, self.projector_ip)
        
        self.payload_video = payloads.get_video_payload(self.my_ip)
        self.payload_wakeup = payloads.get_wakeup_payload()
    
    def connect_and_negotiate(self):
        """Executes the full Epson EasyMP state-machine breach sequence."""
        print(f"[*] Starting negotiation sequence with {self.projector_ip}...")
        
        try:
            # --- STAGE 1: Control Channel (Phase 1) ---
            print(f"[*] 1. Opening Control Channel (Port {config.PORT_CONTROL}) Phase 1...")
            self.s_control = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.s_control.settimeout(5)
            self.s_control.connect((self.projector_ip, config.PORT_CONTROL))
            
            # Send 68 bytes
            self.s_control.send(self.payload_control_p1_part1)
            time.sleep(0.01)
            
            # Send 94 bytes immediately after
            print("[*] 1b. Sending Control Phase 1 Part 2 (94 bytes)...")
            self.s_control.send(self.payload_control_p1_part2)
            
            # Wireshark Frame 42: Projector replies with 226 bytes
            p1_resp = self.s_control.recv(1024)
            print(f"[+] Received Phase 1 Response (Length: {len(p1_resp)})")
            
            # Wireshark Frame 44: Host closes 3620
            print("[*] 1c. Closing Control Channel (Phase 1)...")
            self.s_control.close()
            self.s_control = None # Free the reference for Phase 2
            
            time.sleep(0.05)
            
            # --- STAGE 2: Video Channel ---
            print(f"[*] 2. Opening Video Channel (Port {config.PORT_VIDEO})...")
            self.s_video = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.s_video.settimeout(5)
            self.s_video.connect((self.projector_ip, config.PORT_VIDEO))
            self.s_video.send(self.payload_video)
            
            time.sleep(1.0) # Projector OS buffer allocation
            
            # --- STAGE 3: Wake/Trigger ---
            print(f"[*] 3. Firing State Trigger (Port {config.PORT_WAKE})...")
            self.s_wake = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.s_wake.settimeout(5)
            self.s_wake.connect((self.projector_ip, config.PORT_WAKE))
            self.s_wake.send(self.payload_wakeup)
            
            trigger_resp = self.s_wake.recv(1024)
            print(f"[+] Trigger acknowledged: {trigger_resp[:10]}")
            
            # --- STAGE 4: Force State Progression ---
            print(f"[*] 4. Closing Port {config.PORT_WAKE} to force progression...")
            
            # Send TCP RST instead of FIN (Linger Time = 0)
            l_onoff = 1
            l_linger = 0
            self.s_wake.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack('ii', l_onoff, l_linger))
            
            self.s_wake.close()
            
            # Wireshark Frame 70: Exactly 1 second delay before Phase 2.
            time.sleep(1.0) 
            
            # --- STAGE 5: Final Negotiation (Phase 2) ---
            print(f"[*] 5. Reopening Control Channel (Port {config.PORT_CONTROL}) Phase 2...")
            self.s_control = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.s_control.settimeout(5)
            self.s_control.connect((self.projector_ip, config.PORT_CONTROL))
            
            print("[*] 5b. Sending Control Phase 2 (264 bytes)...")
            self.s_control.send(self.payload_control_p2)
            
            final_resp = self.s_control.recv(1024)
            if final_resp:
                print("\n[+] BINGO! Connection Fully Established!")
                print(f"    Params received (Length {len(final_resp)}): {final_resp.hex()[:60]}...")
                return True
            else:
                print("[-] Protocol failure. Projector stayed silent.")
                return False
                
        except Exception as e:
            print(f"[-] Negotiation failed: {e}")
            self.disconnect()
            return False

    def send_video_frame(self, frame_payload):
        """Sends encrypted frame data to Port 3621."""
        if not self.s_video:
            print("[-] Cannot send video frame. Video socket not initialized!")
            return False
            
        try:
            self.s_video.sendall(frame_payload)
            return True
        except Exception as e:
            print(f"[-] Stream interrupted: {e}")
            return False

    def disconnect(self):
        """Tears down all connections safely."""
        print("[*] Disconnecting client...")
        if self.s_control:
            try: self.s_control.close()
            except: pass
        if self.s_video:
            try: self.s_video.close()
            except: pass
        if self.s_wake:
            try: self.s_wake.close()
            except: pass

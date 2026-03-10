import subprocess
import time

def scan_wifi_networks():
    print("Scanning for Wi-Fi networks... Please wait.")
    try:
        # Requesting an active scan can take a few seconds
        subprocess.run(["nmcli", "dev", "wifi", "rescan"], capture_output=True)
        time.sleep(2) # Give it a moment to find networks
    except Exception:
        pass # Ignore if rescan fails
        
    try:
        # Run nmcli to list networks, including BSSID which is more reliable for connections
        result = subprocess.run(
            ["nmcli", "-t", "-f", "SSID,BSSID,SECURITY,SIGNAL", "dev", "wifi"], 
            capture_output=True, 
            text=True, 
            check=True
        )
    except Exception as e:
        print(f"Could not scan Wi-Fi networks using nmcli: {e}")
        return []

    lines = result.stdout.strip().split('\n')
    networks = []
    seen = set()
    for line in lines:
        if not line:
            continue
        
        # nmcli escapes colons as \:, which BSSID relies heavily on.
        # We replace the escaped colons during splitting to avoid breaking the MAC addresses.
        parts = line.replace('\\:', '%%COLON%%').split(':')
        if len(parts) >= 4:
            ssid = parts[0].replace('%%COLON%%', ':')
            bssid = parts[1].replace('%%COLON%%', ':')
            security = parts[2]
            signal = parts[3]
            
            # We want to keep track of SSIDs so we don't display duplicates,
            # but we use the BSSID of the strongest signal.
            if ssid and ssid != '--' and ssid not in seen:
                seen.add(ssid)
                networks.append({"ssid": ssid, "bssid": bssid, "security": security, "signal": signal})
                
    # Sort by signal strength (descending)
    return sorted(networks, key=lambda x: int(x['signal']) if x['signal'].isdigit() else 0, reverse=True)

def connect_to_wifi(ssid, bssid, password=None):
    print(f"\nAttempting to connect to '{ssid}' (BSSID: {bssid})...")
    # Connecting using BSSID handles identical SSIDs or SSIDs with trailing spaces perfectly
    command = ["nmcli", "dev", "wifi", "connect", bssid]
    if password:
        command.extend(["password", password])
        
    try:
        result = subprocess.run(command, capture_output=True, text=True)
        if result.returncode == 0:
            print(f"[+] Successfully connected to '{ssid}'!")
            return True
        else:
            print(f"[-] Failed to connect: {result.stderr or result.stdout}")
            return False
    except Exception as e:
        print(f"[-] Error connecting to Wi-Fi: {e}")
        return False

def get_current_wifi():
    """Returns the UUID and NAME of the currently active Wi-Fi connection."""
    try:
        result = subprocess.run(
            ["nmcli", "-t", "-f", "UUID,TYPE,NAME", "connection", "show", "--active"],
            capture_output=True, text=True, check=True
        )
        for line in result.stdout.strip().split('\n'):
            parts = line.split(':')
            if len(parts) >= 3 and parts[1] == "802-11-wireless":
                return parts[0], parts[2] # UUID, NAME
    except Exception:
        pass
    return None, None

def revert_wifi(original_uuid, projector_ssid):
    """Reconnects to the original Wi-Fi and deletes the projector network profile."""
    if projector_ssid:
        print(f"\n[*] Forgetting projector network '{projector_ssid}'...")
        subprocess.run(["nmcli", "connection", "delete", projector_ssid], capture_output=True)
        
    if original_uuid:
        print(f"[*] Reconnecting to original Wi-Fi network...")
        subprocess.run(["nmcli", "connection", "up", "uuid", original_uuid], capture_output=True)
        print("[+] Network restored.")

def interactive_wifi_setup():
    # Save the current state before doing anything
    orig_uuid, orig_name = get_current_wifi()
    if orig_name:
        print(f"[*] Currently connected to: {orig_name}")

    networks = scan_wifi_networks()
    if not networks:
        print("No Wi-Fi networks found or Wi-Fi is disabled.")
        return False, orig_uuid, None
    
    print("\nAvailable Wi-Fi Networks:")
    for i, net in enumerate(networks):
        sec = net['security'] if net['security'] != '--' else 'Open'
        print(f"[{i+1}] {net['ssid']} (Signal: {net['signal']}%, Security: {sec})")
        
    choice = input("\nSelect a network to connect to (or 0 to skip): ").strip()
    try:
        choice_idx = int(choice) - 1
        if choice_idx == -1:
            print("Skipping Wi-Fi setup.")
            return True, orig_uuid, None # User skipped, which is fine
        if 0 <= choice_idx < len(networks):
            selected_net = networks[choice_idx]
            ssid = selected_net['ssid']
            bssid = selected_net['bssid']
            password = None
            if selected_net['security'] and selected_net['security'] != '--':
                password = input(f"Enter password for '{ssid}': ")
            
            connected = connect_to_wifi(ssid, bssid, password)
            
            # If we connected to the projector, we return its SSID so we can delete it later
            proj_ssid_to_delete = ssid if connected else None
            return connected, orig_uuid, proj_ssid_to_delete
        else:
            print("Invalid choice. Skipping Wi-Fi setup.")
            return False, orig_uuid, None
    except ValueError:
        print("Invalid input. Skipping Wi-Fi setup.")
        return False, orig_uuid, None

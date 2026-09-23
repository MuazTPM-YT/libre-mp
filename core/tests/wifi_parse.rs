// wi-fi tool output parsers, checked on every os

use libremp_core::wifi::{
    is_projector_ssid, netsh_current, parse_netsh_networks, parse_nmcli, parse_system_profiler,
};

// escaped colons, open net, hidden net, duplicate ssid keep strongest
#[test]
fn nmcli_list() {
    let out = "Home\\:Net:AA\\:BB\\:CC\\:DD\\:EE\\:01:WPA2:70\n\
               Cafe:AA\\:BB\\:CC\\:DD\\:EE\\:02::40\n\
               --:AA\\:BB\\:CC\\:DD\\:EE\\:03:WPA2:90\n\
               Home\\:Net:AA\\:BB\\:CC\\:DD\\:EE\\:04:WPA2:80\n\
               LAB-xK3pQ7vRt2Lm9Nz:AA\\:BB\\:CC\\:DD\\:EE\\:05:WPA2:55\n";
    let nets = parse_nmcli(out);
    let names: Vec<_> = nets.iter().map(|n| n.ssid.as_str()).collect();
    assert_eq!(names, ["Home:Net", "LAB-xK3pQ7vRt2Lm9Nz", "Cafe"]);
    assert_eq!(nets[0].bssid, "AA:BB:CC:DD:EE:04");
    assert_eq!(nets[0].signal, 80);
    assert_eq!(nets[2].security, "Open");
    assert!(nets[1].is_projector);
    assert!(!nets[0].is_projector);
}

// several bssids per ssid, colons inside ssid, open auth
#[test]
fn netsh_networks() {
    let out = r"
Interface name : Wi-Fi
There are 2 networks currently visible.

SSID 1 : Office: 5G
    Network type            : Infrastructure
    Authentication          : WPA2-Personal
    Encryption              : CCMP
    BSSID 1                 : aa:bb:cc:dd:ee:01
         Signal             : 40%
         Radio type         : 802.11ac
    BSSID 2                 : aa:bb:cc:dd:ee:02
         Signal             : 85%

SSID 2 : Guest
    Network type            : Infrastructure
    Authentication          : Open
    Encryption              : None
    BSSID 1                 : aa:bb:cc:dd:ee:03
         Signal             : 60%
";
    let nets = parse_netsh_networks(out);
    assert_eq!(nets.len(), 2);
    assert_eq!(nets[0].ssid, "Office: 5G");
    assert_eq!(nets[0].bssid, "aa:bb:cc:dd:ee:02");
    assert_eq!(nets[0].signal, 85);
    assert_eq!(nets[0].security, "WPA2-Personal");
    assert_eq!(nets[1].security, "Open");
}

// profile wins over ssid; disconnected adapter ignored; hosted status not a state
#[test]
fn netsh_current_profile() {
    let connected = r"
There is 1 interface on the system:

    Name                   : Wi-Fi
    State                  : connected
    SSID                   : HomeNet
    BSSID                  : aa:bb:cc:dd:ee:01
    Profile                : HomeNet 2

    Hosted network status  : Not available
";
    assert_eq!(netsh_current(connected).as_deref(), Some("HomeNet 2"));
    let german = "    Name : WLAN\n    Status : Verbunden\n    SSID : Heim\n    Profil : Heim\n";
    assert_eq!(netsh_current(german).as_deref(), Some("Heim"));
    let off = "    Name : Wi-Fi\n    State : disconnected\n";
    assert_eq!(netsh_current(off), None);
}

// visible + current networks, rssi to percent, security words
#[test]
fn system_profiler_json() {
    let json = r#"{"SPAirPortDataType":[{"spairport_airport_interfaces":[{
        "spairport_current_network_information":{"_name":"Home","spairport_signal_noise":"-50 dBm / -90 dBm","spairport_security_mode":"spairport_security_mode_wpa2_personal"},
        "spairport_airport_other_local_wireless_networks":[
            {"_name":"EPSON-proj","spairport_signal_noise":"-70 dBm / -90 dBm","spairport_security_mode":"spairport_security_mode_none"},
            {"_name":"","spairport_signal_noise":"-40 dBm / -90 dBm"}
        ]}]}]}"#;
    let nets = parse_system_profiler(json);
    assert_eq!(nets.len(), 2);
    assert_eq!((nets[0].ssid.as_str(), nets[0].signal, nets[0].security.as_str()), ("Home", 100, "WPA2"));
    assert_eq!((nets[1].ssid.as_str(), nets[1].signal, nets[1].security.as_str()), ("EPSON-proj", 60, "Open"));
    assert!(nets[1].is_projector);
    assert!(parse_system_profiler("not json").is_empty());
}

// epson quick connect names flagged, plain names not
#[test]
fn projector_names() {
    assert!(is_projector_ssid("PROJ42-Ab3dEf6hIj9kLm"));
    assert!(is_projector_ssid("DIRECT-EPSON-1234"));
    assert!(!is_projector_ssid("Home-Network"));
    assert!(!is_projector_ssid("my-wifi-abcdefghijkl"));
    assert!(!is_projector_ssid("-fE8DSypQz51AR2Q"));
}

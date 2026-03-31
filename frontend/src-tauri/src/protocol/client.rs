use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::protocol::config;
use crate::protocol::payloads;

#[derive(Debug)]
pub enum ProtocolError {
    ConnectFailed(String),
    AuthFailed(String),
    NetworkError(String),
}

pub struct EpsonClient {
    pub projector_ip: String,
    pub my_ip: String,
    pub password: String,
    pub ssid: String,
    pub s_auth: Option<Arc<Mutex<TcpStream>>>,
    pub s_video: Option<Arc<Mutex<TcpStream>>>,
    pub s_video_aux: Option<Arc<Mutex<TcpStream>>>,
}

impl EpsonClient {
    pub fn new(projector_ip: &str, password: &str, ssid: &str) -> Self {
        Self {
            projector_ip: projector_ip.to_string(),
            my_ip: config::get_local_ip(),
            password: password.to_string(),
            ssid: ssid.to_string(),
            s_auth: None,
            s_video: None,
            s_video_aux: None,
        }
    }

    pub async fn connect_and_negotiate(&mut self) -> Result<(), ProtocolError> {
        println!("[*] Starting deterministic negotiation sequence with {}...", self.projector_ip);
        
        self.authenticate_session().await?;
        self.complete_auth_handshake().await?;
        self.open_video_channels().await?;
        self.wait_for_streaming_signal().await?;
        self.send_warmup_buffers().await?;
        
        println!("\n[+] BINGO! Connection Fully Established and Ready for Video Stream!");
        Ok(())
    }

    async fn authenticate_session(&mut self) -> Result<(), ProtocolError> {
        println!("[*] 1. Opening Authentication Channel (Port 3620)...");
        let reg_addr = format!("{}:{}", self.projector_ip, config::PORT_CONTROL);
        
        // --- REGISTRATION PHASE ---
        let mut reg_stream = TcpStream::connect(&reg_addr).await
            .map_err(|e| ProtocolError::ConnectFailed(format!("Registration connection failed: {}", e)))?;
        reg_stream.set_nodelay(true).unwrap();

        println!("[*]    Sending 68-byte Registration Packet...");
        let reg_payload = payloads::get_registration_payload(&self.my_ip);
        reg_stream.write_all(&reg_payload).await
            .map_err(|e| ProtocolError::NetworkError(e.to_string()))?;

        let mut buf = vec![0u8; 1024];
        if let Ok(Ok(size)) = tokio::time::timeout(Duration::from_secs(2), reg_stream.read(&mut buf)).await {
            println!("[+]    Registration Resp 1: {} bytes", size);
        }

        println!("[*]    Closing Registration channel, opening Auth channel...");
        drop(reg_stream);
        tokio::time::sleep(Duration::from_millis(100)).await;

        // --- AUTHENTICATION SEQUENCE ---
        let mut auth_stream = TcpStream::connect(&reg_addr).await
            .map_err(|e| ProtocolError::ConnectFailed(format!("Auth connection failed: {}", e)))?;
        auth_stream.set_nodelay(true).unwrap();

        println!("[*]    Sending 264-Byte Full Auth Request...");
        let mac = self.password.replace([':', '-'], "").to_lowercase();
        let auth_payload = payloads::get_auth_payload_full(&self.my_ip, &self.projector_ip, &mac, &self.ssid);
        auth_stream.write_all(&auth_payload).await
            .map_err(|e| ProtocolError::NetworkError(e.to_string()))?;

        println!("[*]    Waiting for authentication response...");
        if let Ok(Ok(size)) = tokio::time::timeout(Duration::from_secs(3), auth_stream.read(&mut buf)).await {
            println!("[+]    Auth Resp: {} bytes", size);
            if size >= 50 {
                println!("[+]    Auth Status Byte: 0x{:02x}", buf[50]);
            }
        } else {
            println!("[-]    Auth response timed out (continuing anyway)");
        }

        self.s_auth = Some(Arc::new(Mutex::new(auth_stream)));
        println!("[+]    Authentication phase complete.");
        Ok(())
    }

    async fn complete_auth_handshake(&self) -> Result<(), ProtocolError> {
        println!("[*]    Completing post-auth handshake on Port 3620...");
        let s_auth_arc = self.s_auth.as_ref().unwrap();
        let mut s_auth = s_auth_arc.lock().await;
        
        let mut buf = vec![0u8; 4096];
        let mut already_responded = false;
        let mut ready_received = false;

        for _ in 0..10 {
            if ready_received { break; }
            let read_future = s_auth.read(&mut buf);
            let size = match tokio::time::timeout(Duration::from_secs(3), read_future).await {
                Ok(Ok(n)) if n > 0 => n,
                _ => break,
            };

            let data = &buf[..size];
            let mut offset = 0;
            
            while offset + 20 <= data.len() {
                if &data[offset..offset+8] != b"EEMP0100" { break; }
                
                let cmd_bytes: [u8; 4] = data[offset+12..offset+16].try_into().unwrap_or([0;4]);
                let cmd = u32::from_le_bytes(cmd_bytes);
                let payload_len_bytes: [u8; 4] = data[offset+16..offset+20].try_into().unwrap_or([0;4]);
                let payload_len = u32::from_le_bytes(payload_len_bytes);
                let msg_len = 20 + payload_len as usize;

                println!("[+]    Post-auth recv: cmd=0x{:04x}, {} bytes", cmd, msg_len);

                if cmd == 0x010E && !already_responded {
                    let response = self._build_0x0108_response();
                    if s_auth.write_all(&response).await.is_ok() {
                        already_responded = true;
                        println!("[+]    Sent 0x0108 response: {} bytes", response.len());
                    }
                } else if cmd == 0x0110 {
                    println!("[+]    Received 0x0110 'Ready to Stream' signal!");
                    ready_received = true;
                    break;
                }
                
                offset += msg_len;
            }
        }

        Ok(())
    }

    fn _build_0x0108_response(&self) -> Vec<u8> {
        let pcap_hex = "45454d5030313030c0a8580208010000480100000001000000000000000000000000000000000000\
                        00000000000000000000000000000000000000000000000000000000140100000500000038000000\
                        0200000004000000c0a858020c00000004000000010000000100000004000000500043000b000000\
                        04000000000000001c00000000000000070000004400000001000000050000003800000002000000\
                        04000000c0a858020c00000004000000010000000100000004000000500043000b00000004000000\
                        000000001c0000000000000008000000800000000400000005000000380000000200000004000000\
                        c0a858020c00000004000000010000000100000004000000500043000b0000000400000001010000\
                        1c00000000000000000000000c000000020000000400000002000000000000000c00000002000000\
                        0400000003000000000000000c000000020000000400000004000000";
        let mut raw = hex::decode(pcap_hex).unwrap();
        let ip_bytes = payloads::get_hex_ip_bytes(&self.my_ip);
        let pcap_ip = [0xc0, 0xa8, 0x58, 0x02]; // 192.168.88.2
        
        for i in 0..raw.len()-3 {
            if raw[i] == pcap_ip[0] && raw[i+1] == pcap_ip[1] && raw[i+2] == pcap_ip[2] && raw[i+3] == pcap_ip[3] {
                raw[i..i+4].copy_from_slice(&ip_bytes);
            }
        }
        raw
    }

    async fn open_video_channels(&mut self) -> Result<(), ProtocolError> {
        println!("[*] 2. Opening Video Channels (Port 3621)...");
        tokio::time::sleep(Duration::from_millis(300)).await;
        let addr = format!("{}:{}", self.projector_ip, config::PORT_VIDEO);

        // --- Video Channel ---
        let mut s_video = TcpStream::connect(&addr).await
            .map_err(|e| ProtocolError::ConnectFailed(e.to_string()))?;
        s_video.set_nodelay(true).unwrap();
        
        let ctrl_init = payloads::get_video_init_payload_ctrl(&self.my_ip);
        s_video.write_all(&ctrl_init).await
            .map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
        self.s_video = Some(Arc::new(Mutex::new(s_video)));

        // --- Aux Channel ---
        let mut s_aux = TcpStream::connect(&addr).await
            .map_err(|e| ProtocolError::ConnectFailed(e.to_string()))?;
        s_aux.set_nodelay(true).unwrap();
        
        let data_init = payloads::get_video_init_payload_data(&self.my_ip);
        s_aux.write_all(&data_init).await
            .map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
        self.s_video_aux = Some(Arc::new(Mutex::new(s_aux)));

        Ok(())
    }

    async fn wait_for_streaming_signal(&self) -> Result<(), ProtocolError> {
        println!("[*] 3. Waiting for projector 0x0016 streaming signal...");
        let s_auth_arc = self.s_auth.as_ref().unwrap();
        let mut s_auth = s_auth_arc.lock().await;

        let mut buf = vec![0u8; 4096];
        if let Ok(Ok(size)) = tokio::time::timeout(Duration::from_secs(10), s_auth.read(&mut buf)).await {
            let data = &buf[..size];
            if size >= 20 && &data[0..8] == b"EEMP0100" {
                let cmd = u32::from_le_bytes(data[12..16].try_into().unwrap_or([0;4]));
                if cmd == 0x0016 {
                    println!("[+]    Projector confirmed streaming ready (0x0016)!");
                }
            }
        }
        Ok(())
    }

    async fn send_aux_bundle(&self, size: u32) -> Result<(), ProtocolError> {
        if let Some(s_aux_arc) = &self.s_video_aux {
            let mut s_aux = s_aux_arc.lock().await;
            let hdr = payloads::get_aux_header(size);
            let zeros = vec![0u8; size as usize];
            s_aux.write_all(&hdr).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
            s_aux.write_all(&zeros).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
        }
        Ok(())
    }

    async fn send_warmup_buffers(&self) -> Result<(), ProtocolError> {
        println!("[*] 4. Sending warmup buffers on byte28=0x01...");
        let warmup_sizes = [7276, 2646, 1764];
        for size in warmup_sizes {
            self.send_aux_bundle(size).await?;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    pub async fn send_video_frame(&self, is_first: bool, x: u16, y: u16, w: u16, h: u16, jpeg_bytes: &[u8]) -> Result<(), ProtocolError> {
        let s_video_arc = self.s_video.as_ref().ok_or_else(|| ProtocolError::NetworkError("Video socket not open".into()))?;
        let mut s_video = s_video_arc.lock().await;

        if is_first {
            let meta = payloads::get_display_config_meta();
            let meta_hdr = payloads::get_eprd_meta_header(&self.my_ip, meta.len() as u32);
            let frame_type = 4u32;
            let frame_hdr = payloads::get_frame_header(frame_type, x, y, w, h);
            
            let jpeg_payload_size = frame_hdr.len() as u32 + jpeg_bytes.len() as u32;
            let jpeg_hdr = payloads::get_eprd_jpeg_header(&self.my_ip, jpeg_payload_size);

            s_video.write_all(&meta_hdr).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
            s_video.write_all(&meta).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
            s_video.write_all(&jpeg_hdr).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
            s_video.write_all(&frame_hdr).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;

            let mut offset = 0;
            while offset < jpeg_bytes.len() {
                let end = std::cmp::min(offset + 1460, jpeg_bytes.len());
                s_video.write_all(&jpeg_bytes[offset..end]).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
                offset = end;
            }
        } else {
            let frame_hdr = payloads::get_subsequent_frame_header(x, y, w, h);
            let jpeg_payload_size = frame_hdr.len() as u32 + jpeg_bytes.len() as u32;
            let jpeg_hdr = payloads::get_eprd_jpeg_header(&self.my_ip, jpeg_payload_size);

            s_video.write_all(&jpeg_hdr).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
            s_video.write_all(&frame_hdr).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
            
            let mut offset = 0;
            while offset < jpeg_bytes.len() {
                let end = std::cmp::min(offset + 1460, jpeg_bytes.len());
                s_video.write_all(&jpeg_bytes[offset..end]).await.map_err(|e| ProtocolError::NetworkError(e.to_string()))?;
                offset = end;
            }
        }
        Ok(())
    }

    pub async fn send_frame_keepalive(&self) -> Result<(), ProtocolError> {
        let _ = self.send_aux_bundle(2646).await;
        let _ = self.send_aux_bundle(1764).await;
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        self.s_auth = None;
        self.s_video = None;
        self.s_video_aux = None;
    }
}

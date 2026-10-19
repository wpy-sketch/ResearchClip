use researchclip_clipper_protocol::{self as p, Endpoint, Envelope, Reply, Request};
use std::{
    io::{self, Read},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};
fn config_root() -> io::Result<PathBuf> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("RESEARCHCLIP_TEST_SETTINGS_DIR") {
        return Ok(PathBuf::from(path));
    }
    Ok(PathBuf::from(
        std::env::var_os("APPDATA").ok_or_else(|| io::Error::other("APPDATA unavailable"))?,
    )
    .join("io.researchclip.desktop"))
}
fn run(origin: String) -> Reply {
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = p::read_frame::<Request>(&mut io::stdin().lock());
        let _ = send.send(result);
    });
    let request = match receive.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(r)) => r,
        _ => return Reply::error("INVALID_MESSAGE", "消息无效、超限或读取超时。"),
    };
    if let Err(message) = request.validate() {
        return Reply::error("INVALID_MESSAGE", message);
    }
    let result = (|| -> io::Result<Reply> {
        let root = config_root()?;
        let path = root.join("clipper-endpoint-v1.json");
        for part in [&root, &path] {
            let m = std::fs::symlink_metadata(part)?;
            if m.file_type().is_symlink() {
                return Err(io::Error::other("reparse rejected"));
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if m.file_attributes() & 0x400 != 0 {
                    return Err(io::Error::other("reparse rejected"));
                }
            }
        }
        let file = std::fs::File::open(path)?;
        if file.metadata()?.len() > 2048 {
            return Err(io::Error::other("endpoint rejected"));
        }
        let mut bytes = Vec::new();
        file.take(2049).read_to_end(&mut bytes)?;
        let e: Endpoint = serde_json::from_slice(&bytes)?;
        if e.version != p::VERSION
            || e.port == 0
            || e.token.len() != 64
            || !e.token.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(io::Error::other("endpoint rejected"));
        }
        let mut socket = TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], e.port)),
            Duration::from_secs(2),
        )?;
        socket.set_read_timeout(Some(Duration::from_secs(15)))?;
        socket.set_write_timeout(Some(Duration::from_secs(3)))?;
        p::write_frame(
            &mut socket,
            &Envelope {
                token: e.token,
                origin,
                request,
            },
        )?;
        p::read_frame(&mut socket)
    })();
    result.unwrap_or_else(|_|Reply::error("DESKTOP_UNAVAILABLE","未连接到 ResearchClip。请启动桌面应用，在设置中启用网页剪藏；通信中断时可用同一请求重试。"))
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let allowed = args.first().is_some_and(|s| p::origin_valid(s))
        && args.iter().skip(1).all(|s| {
            s.strip_prefix("--parent-window=")
                .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        });
    let reply = if allowed {
        run(args[0].clone())
    } else {
        Reply::error("ORIGIN_DENIED", "此扩展来源未获授权。")
    };
    // stdout is exclusively the native-messaging binary protocol; never log clip content.
    let _ = p::write_frame(&mut io::stdout().lock(), &reply);
}

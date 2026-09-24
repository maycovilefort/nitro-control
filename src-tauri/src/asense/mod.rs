pub mod protocol;

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use protocol::{parse_reply, Reply};

pub const SOCKET_PATH: &str = "/run/asense-control.sock";

#[derive(Debug, Clone, PartialEq)]
pub enum AsenseError {
    Unavailable(String),
    Busy,
    NotConnected,
    Timeout,
    Io(String),
    Protocol(String),
    Rejected(String),
}

impl fmt::Display for AsenseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsenseError::Unavailable(e) => write!(f, "ASense indisponível: {e}"),
            AsenseError::Busy => write!(f, "A interface do ASense está aberta — feche-a para usar o Nitro Control"),
            AsenseError::NotConnected => write!(f, "ASense desconectado"),
            AsenseError::Timeout => write!(f, "ASense não respondeu a tempo"),
            AsenseError::Io(e) => write!(f, "Erro de comunicação com o ASense: {e}"),
            AsenseError::Protocol(e) => write!(f, "Resposta inválida do ASense: {e}"),
            AsenseError::Rejected(e) => write!(f, "O ASense recusou o comando: {e}"),
        }
    }
}

pub trait Asense: Send {
    fn connected(&self) -> bool;
    fn connect(&mut self) -> Result<(), AsenseError>;
    /// Envia um comando e devolve o payload após `OK `.
    fn request(&mut self, cmd: &str) -> Result<String, AsenseError>;
}

pub struct Client {
    path: PathBuf,
    timeout: Duration,
    conn: Option<BufReader<UnixStream>>,
}

impl Client {
    pub fn new(path: PathBuf, timeout: Duration) -> Self {
        Client { path, timeout, conn: None }
    }
}

impl Asense for Client {
    fn connected(&self) -> bool {
        self.conn.is_some()
    }

    fn connect(&mut self) -> Result<(), AsenseError> {
        self.conn = None;
        let s = UnixStream::connect(&self.path).map_err(|e| AsenseError::Unavailable(e.to_string()))?;
        s.set_read_timeout(Some(self.timeout)).map_err(|e| AsenseError::Io(e.to_string()))?;
        s.set_write_timeout(Some(self.timeout)).map_err(|e| AsenseError::Io(e.to_string()))?;
        self.conn = Some(BufReader::new(s));
        match self.request("HELLO 2") {
            Ok(p) if p.starts_with("protocol=2") => Ok(()),
            Ok(p) => {
                self.conn = None;
                Err(AsenseError::Protocol(format!("handshake inesperado: {p}")))
            }
            Err(AsenseError::Timeout) => {
                self.conn = None;
                Err(AsenseError::Busy)
            }
            Err(e) => {
                self.conn = None;
                Err(e)
            }
        }
    }

    fn request(&mut self, cmd: &str) -> Result<String, AsenseError> {
        if cmd.len() > 192 || cmd.contains('\n') || cmd.contains('\r') {
            return Err(AsenseError::Protocol("comando inválido".into()));
        }
        let conn = self.conn.as_mut().ok_or(AsenseError::NotConnected)?;
        let res: io::Result<String> = (|| {
            conn.get_mut().write_all(format!("{cmd}\n").as_bytes())?;
            let mut line = String::new();
            if conn.read_line(&mut line)? == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "socket fechado"));
            }
            Ok(line)
        })();
        match res {
            Ok(line) => match parse_reply(&line) {
                Ok(Reply::Ok(p)) => Ok(p),
                Ok(Reply::Err(m)) => Err(AsenseError::Rejected(m)),
                Err(e) => {
                    self.conn = None;
                    Err(e)
                }
            },
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {
                self.conn = None;
                Err(AsenseError::Timeout)
            }
            Err(e) => {
                self.conn = None;
                Err(AsenseError::Io(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;

    /// Servidor falso: responde HELLO e ecoa comandos conhecidos; fecha depois de `max` comandos.
    fn fake_server(path: &std::path::Path, max: usize) -> thread::JoinHandle<()> {
        let listener = UnixListener::bind(path).unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut w = stream.try_clone().unwrap();
            let mut r = BufReader::new(stream);
            for _ in 0..max {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap() == 0 {
                    return;
                }
                let reply = match line.trim() {
                    "HELLO 2" => "OK protocol=2 daemon=0.3.0",
                    "PING" => "OK ready",
                    "FAN AUTO" => "OK fan=auto",
                    _ => "ERR unsupported command",
                };
                writeln!(w, "{reply}").unwrap();
            }
        })
    }

    fn client(path: &std::path::Path) -> Client {
        Client::new(path.to_path_buf(), Duration::from_millis(300))
    }

    #[test]
    fn handshake_and_commands() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("s.sock");
        let srv = fake_server(&sock, 10);
        let mut c = client(&sock);
        c.connect().unwrap();
        assert!(c.connected());
        assert_eq!(c.request("PING").unwrap(), "ready");
        assert_eq!(c.request("BOGUS"), Err(AsenseError::Rejected("unsupported command".into())));
        assert!(c.connected(), "ERR mantém a sessão");
        drop(c);
        srv.join().unwrap();
    }

    #[test]
    fn missing_socket_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = client(&dir.path().join("nada.sock"));
        assert!(matches!(c.connect(), Err(AsenseError::Unavailable(_))));
        assert_eq!(c.request("PING"), Err(AsenseError::NotConnected));
    }

    #[test]
    fn silent_server_means_busy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("s.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let _hold = thread::spawn(move || {
            let (_s, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_secs(2));
        });
        let mut c = client(&sock);
        assert_eq!(c.connect(), Err(AsenseError::Busy));
        assert!(!c.connected());
    }

    #[test]
    fn rejects_oversized_or_multiline_commands() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("s.sock");
        let _srv = fake_server(&sock, 10);
        let mut c = client(&sock);
        c.connect().unwrap();
        assert!(matches!(c.request(&"X".repeat(193)), Err(AsenseError::Protocol(_))));
        assert!(matches!(c.request("PING\nFAN MAXIMUM"), Err(AsenseError::Protocol(_))));
    }

    #[test]
    fn reconnects_after_server_restart() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("s.sock");
        let srv = fake_server(&sock, 2); // HELLO + PING, depois fecha
        let mut c = client(&sock);
        c.connect().unwrap();
        assert_eq!(c.request("PING").unwrap(), "ready");
        srv.join().unwrap();
        assert!(c.request("PING").is_err());
        assert!(!c.connected(), "queda derruba a conexão");
        std::fs::remove_file(&sock).unwrap();
        let _srv2 = fake_server(&sock, 10);
        c.connect().unwrap();
        assert_eq!(c.request("FAN AUTO").unwrap(), "fan=auto");
    }
}

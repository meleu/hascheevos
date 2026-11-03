//! A fake RetroAchievements server and a sandbox to run the binary in, so no
//! test reaches the real API or the user's own cache.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub const API_KEY: &str = "test-api-key";

#[derive(Clone, Debug)]
pub struct Request {
    pub path: String,
    /// Decoded `name=value` pairs, in the order they were sent.
    pub query: Vec<(String, String)>,
    pub api_key: Option<String>,
}

pub struct Reply {
    pub status: u16,
    pub body: String,
}

pub struct FakeServer {
    url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl FakeServer {
    pub fn start(handler: impl Fn(&Request) -> Reply + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake server");
        let url = format!("http://{}", listener.local_addr().expect("local address"));
        let requests = Arc::new(Mutex::new(Vec::new()));

        let recorded = Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.expect("accept connection");
                let request = read_request(&stream);
                recorded.lock().unwrap().push(request.clone());
                let reply = handler(&request);
                write!(
                    stream,
                    "HTTP/1.1 {} Fake\r\nContent-Type: application/vnd.api+json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    reply.status,
                    reply.body.len(),
                    reply.body
                )
                .expect("write reply");
            }
        });

        Self { url, requests }
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

fn read_request(stream: &std::net::TcpStream) -> Request {
    let mut lines = BufReader::new(stream).lines().map(|line| line.unwrap());
    let request_line = lines.next().expect("request line");
    let target = request_line.split(' ').nth(1).expect("request target");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    let mut api_key = None;
    for line in lines.take_while(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("x-api-key") {
                api_key = Some(value.trim().to_owned());
            }
        }
    }

    Request {
        path: path.to_owned(),
        query: query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .map(|(name, value)| (decode(name), decode(value)))
            .collect(),
        api_key,
    }
}

fn decode(text: &str) -> String {
    text.replace("%5B", "[")
        .replace("%5D", "]")
        .replace("%2C", ",")
}

/// The binary's environment for one test: its own server, cache and home.
pub struct Sandbox {
    pub server: FakeServer,
    dir: tempfile::TempDir,
}

impl Sandbox {
    pub fn new(server: FakeServer) -> Self {
        Self {
            server,
            dir: tempfile::tempdir().expect("create sandbox"),
        }
    }

    pub fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }

    pub fn xdg_cache(&self) -> PathBuf {
        self.dir.path().join("xdg-cache")
    }

    /// A command with a key and the sandbox's paths, ready for more changes.
    pub fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hascheevos"));
        command
            .args(args)
            .env("RA_API_URL", &self.server.url)
            .env("RA_API_KEY", API_KEY)
            .env("XDG_CACHE_HOME", self.xdg_cache())
            .env("HOME", self.home());
        command
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("run hascheevos")
    }
}

/// Every file under `dir`, however deep.
pub fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries {
        let path = entry.expect("read directory entry").path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// Sets back the modification time of every cache file, which is how `bkt`
/// measures age.
pub fn age_files(dir: &Path, age: Duration) {
    let time = SystemTime::now() - age;
    for file in files_under(dir) {
        std::fs::File::options()
            .write(true)
            .open(&file)
            .and_then(|file| file.set_modified(time))
            .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
    }
}

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{Value, json};

use crate::app::preferences::Preferences;
use crate::stream_mode::streamer::Subtitle;
use crate::translate_mode::translate::Translation;

const ADDRESS: &str = "127.0.0.1:7878";
const PAGE: &str = include_str!("../../assets/overlay.html");

pub struct Overlay {
    latest: Arc<Mutex<Value>>,
    next_id: u64,
}

impl Overlay {
    pub fn start() -> Result<Self, String> {
        let listener = TcpListener::bind(ADDRESS)
            .map_err(|error| t!("error.overlay_port", address = ADDRESS, error = error).to_string())?;

        let latest = Arc::new(Mutex::new(json!({ "id": 0 })));
        let shared_latest = latest.clone();

        thread::spawn(move || {
            for connection in listener.incoming().flatten() {
                let _ = respond(connection, &shared_latest);
            }
        });

        Ok(Self { latest, next_id: 1 })
    }

    pub fn url() -> String {
        format!("http://{ADDRESS}")
    }

    pub fn show(&mut self, subtitle: &Subtitle) {
        let translation = match &subtitle.translation {
            Ok(translation) => lines_json(translation),
            Err(_) => json!([]),
        };

        let message = json!({
            "id": self.next_id,
            "original": lines_json(&subtitle.original),
            "translation": translation,
        });

        self.next_id += 1;

        if let Ok(mut latest) = self.latest.lock() {
            *latest = message;
        }
    }
}

fn lines_json(text: &Translation) -> Value {
    match text {
        Translation::Spanish(spanish) => spanish
            .lines()
            .map(|line| json!([{ "text": line }]))
            .collect(),

        Translation::Japanese(lines) => lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|segment| json!({ "text": segment.text, "ruby": segment.ruby }))
                    .collect::<Value>()
            })
            .collect(),
    }
}

fn respond(connection: TcpStream, latest: &Mutex<Value>) -> std::io::Result<()> {
    let mut reader = BufReader::new(&connection);

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut header = String::new();
    while reader.read_line(&mut header)? > 2 {
        header.clear();
    }

    let path = request_line.split_whitespace().nth(1).unwrap_or("/");

    let (status, content_type, body) = match path {
        "/" => ("200 OK", "text/html; charset=utf-8", PAGE.to_string()),

        "/subtitle" => {
            let mut subtitle = latest
                .lock()
                .map(|latest| latest.clone())
                .unwrap_or_default();

            // Sent with every poll so style changes in Settings reach OBS without a reload.
            subtitle["style"] = Preferences::load().subtitle_style.to_json();

            ("200 OK", "application/json", subtitle.to_string())
        }

        _ => ("404 Not Found", "text/plain", "No encontrado".to_string()),
    };

    let mut connection = connection;

    write!(
        connection,
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )
}

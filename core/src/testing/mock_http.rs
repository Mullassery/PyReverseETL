//! Minimal local HTTP server for testing real outbound HTTP calls (webhook,
//! Salesforce, HubSpot, Marketo adapters) without needing a live account or an
//! extra mocking dependency.
//!
//! Deliberately synchronous (plain `std::net::TcpListener` on a background OS
//! thread, no Tokio) because the adapters under test use
//! `reqwest::blocking::Client` from a *synchronous* trait
//! (`DestinationAdapter`), and a synchronous client should be exercised
//! against a synchronous server -- this sidesteps any "blocking client inside
//! an async runtime" foot-guns entirely and keeps adapter tests as plain
//! `#[test]` functions.
//!
//! It records every request it receives (method, path, headers, body) so
//! tests can assert the adapter under test actually built the real API
//! request shape (correct endpoint, correct auth header, correct JSON body)
//! rather than merely "didn't crash".

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Debug, Clone, Default)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    /// Header names are stored lowercased (HTTP header names are case-insensitive
    /// per RFC 7230 3.2, and HTTP clients are free to send any casing) -- look
    /// up with a lowercase key, e.g. `headers.get("authorization")`.
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

/// A canned local HTTP server. Every request gets the same configured
/// status/body; all requests received are recorded for later assertions.
pub struct MockHttpServer {
    pub base_url: String,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    shutdown: Arc<Mutex<bool>>,
    handle: Option<JoinHandle<()>>,
}

impl MockHttpServer {
    /// Start a server on an OS-assigned local port, responding to every
    /// request with `status`/`body` (as `application/json`).
    pub fn start(status: u16, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock http server");
        listener.set_nonblocking(true).expect("set nonblocking");
        let port = listener.local_addr().expect("local addr").port();
        let base_url = format!("http://127.0.0.1:{port}");

        let requests = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(Mutex::new(false));

        let requests_clone = requests.clone();
        let shutdown_clone = shutdown.clone();
        let status_line = status_line_for(status);
        let body = body.to_string();

        let handle = std::thread::spawn(move || loop {
            if *shutdown_clone.lock().unwrap() {
                break;
            }
            match listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(false).ok();
                    if let Some(req) = read_request(&stream) {
                        requests_clone.lock().unwrap().push(req);
                    }
                    write_response(stream, &status_line, &body);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(_) => break,
            }
        });

        Self {
            base_url,
            requests,
            shutdown,
            handle: Some(handle),
        }
    }

    /// All requests received so far, in arrival order.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }

    pub fn last_request(&self) -> Option<RecordedRequest> {
        self.requests.lock().unwrap().last().cloned()
    }
}

impl Drop for MockHttpServer {
    fn drop(&mut self) {
        *self.shutdown.lock().unwrap() = true;
        // Wake the accept() loop out of its poll by connecting once.
        let _ = TcpStream::connect(self.base_url.trim_start_matches("http://"));
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn status_line_for(status: u16) -> String {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "OK",
    };
    format!("HTTP/1.1 {status} {reason}")
}

fn read_request(stream: &TcpStream) -> Option<RecordedRequest> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);

    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    if request_line.is_empty() {
        return None;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut headers = std::collections::HashMap::new();
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            // HTTP header names are case-insensitive (RFC 7230 3.2); normalize to
            // lowercase on the way in so `RecordedRequest::headers` lookups don't
            // depend on whatever casing the HTTP client happened to send.
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if key == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.insert(key, value);
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).ok()?;
    }

    Some(RecordedRequest {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

fn write_response(mut stream: TcpStream, status_line: &str, body: &str) {
    let response = format!(
        "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_method_path_headers_and_body() {
        let server = MockHttpServer::start(200, r#"{"ok":true}"#);
        let client = reqwest::blocking::Client::new();

        let resp = client
            .post(format!("{}/widgets", server.base_url))
            .header("Authorization", "Bearer abc123")
            .json(&serde_json::json!({"name": "gadget"}))
            .send()
            .unwrap();

        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.json::<serde_json::Value>().unwrap(),
            serde_json::json!({"ok": true})
        );

        let req = server.last_request().expect("one request recorded");
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/widgets");
        assert_eq!(
            req.headers.get("authorization"),
            Some(&"Bearer abc123".to_string())
        );
        assert!(req.body.contains("gadget"));
    }
}

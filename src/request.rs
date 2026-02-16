use anyhow::{bail, Result};
use log::{debug, error};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread::spawn;

type Key = String;
type Value = String;

#[derive(Debug)]
pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) http_version: String,
    pub(crate) headers: Vec<(Key, Value)>,
    pub(crate) body: Option<String>,
}

impl TryFrom<TcpStream> for Request {
    type Error = anyhow::Error;

    fn try_from(stream: TcpStream) -> Result<Self> {
        let mut request_buffer = BufReader::new(&stream);
        let mut request_line = String::new();
        request_buffer.read_line(&mut request_line)?;

        // Split the first line by space
        // The second token is the path
        // Example: `GET /index.html HTTP/1.1`
        let request_line = request_line.split_whitespace().collect::<Vec<&str>>();

        let (method, path, http_version) = match request_line[..] {
            [method, path, http_version] => {
                debug!("Correct format found: {}", request_line.join(" "));
                (
                    method.to_string(),
                    path.to_string(),
                    http_version.to_string(),
                )
            }
            _ => {
                bail!("Unknown request line: {}", request_line.join(" "));
            }
        };

        let mut headers: Vec<(Key, Value)> = Vec::new();
        loop {
            let mut header_line = String::new();
            let next_header = request_buffer.read_line(&mut header_line)?;

            if header_line == "\r\n" || next_header == 0 {
                break;
            }
            let Some((key, value)) = header_line.split_once(": ") else {
                error!("Invalid header. Request: `{header_line}`");
                continue;
            };
            headers.push((key.to_string(), value.trim_end().to_string()));
        }

        Ok(Request {
            method,
            path,
            http_version,
            headers,
            body: None,
        })
    }
}

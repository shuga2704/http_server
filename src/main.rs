use anyhow::{bail, Result};
use log::{debug, error};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::spawn;

const SUCCESS_RESPONSE: &str = "HTTP/1.1 200 OK\r\n\r\n";
const ERROR_RESPONSE: &str = "HTTP/1.1 404 Not Found\r\n\r\n";

type Key = String;
type Value = String;

struct Request {
    method: String,
    path: String,
    http_version: String,
    headers: Vec<(Key, Value)>,
    body: Option<String>,
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

fn handle_request(mut stream: TcpStream) -> Result<()> {
    debug!("Accepted new connection");

    let request = Request::try_from(stream.try_clone()?)?;

    let response = {
        if request.path == "/" {
            debug!("Root path");
            SUCCESS_RESPONSE.to_string()
        } else if request.path == "/user-agent" {
            let mut user_agent = None;
            for (key, value) in &request.headers {
                if key == "User-Agent" {
                    user_agent = Some(value);
                    break;
                }
            }
            match user_agent {
                None => {
                    error!("User-Agent not found");
                    ERROR_RESPONSE.to_string()
                }
                Some(user_agent) => {
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                        user_agent.len(),
                        user_agent
                    );
                    response
                }
            }
        } else if request.path.starts_with("/echo/") {
            let echo_path = request.path.split_once("/echo/");
            match echo_path {
                Some((_, path)) => {
                    debug!("Echo path requested: `{path}`");
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                        path.len(),
                        path
                    );

                    response
                }
                _ => {
                    error!("Invalid path");
                    format!("HTTP/1.1 400 Bad Request\r\n\r\n")
                }
            }
        } else {
            debug!("Unknown path: `{}`", request.path);
            format!("HTTP/1.1 404 Not Found\r\n\r\n")
        }
    };

    stream.write(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let mut response_handles = Vec::new();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let handle = spawn(|| handle_request(stream));
                response_handles.push(handle);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    for handle in response_handles {
        handle.join().unwrap()?;
    }

    Ok(())
}

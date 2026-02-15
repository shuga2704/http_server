use anyhow::Result;
use log::{debug, error};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::spawn;

const SUCCESS_RESPONSE: &str = "HTTP/1.1 200 OK\r\n\r\n";
const ERROR_RESPONSE: &str = "HTTP/1.1 404 Not Found\r\n\r\n";

type Key = String;
type Value = String;

fn handle_request(mut stream: TcpStream) -> Result<()> {
    debug!("Accepted new connection");

    let mut request_buffer = BufReader::new(&stream);
    let mut request_line = String::new();
    request_buffer.read_line(&mut request_line)?;

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

    // Split the first line by space
    // The second token is the path
    // Example: `GET /index.html HTTP/1.1`
    let path = request_line.split_whitespace().collect::<Vec<&str>>();

    let response = match path[..] {
        ["GET", path, "HTTP/1.1"] => {
            if path == "/" {
                debug!("Root path");
                SUCCESS_RESPONSE.to_string()
            } else if path == "/user-agent" {
                let mut user_agent = None;
                for (key, value) in headers {
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
            } else if path.starts_with("/echo/") {
                let echo_path = path.split_once("/echo/");
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
                        error!("Invalid path. Input: `{request_line}`");
                        format!("HTTP/1.1 400 Bad Request\r\n\r\n")
                    }
                }
            } else {
                debug!("Unknown path: `{path}`");
                format!("HTTP/1.1 404 Not Found\r\n\r\n")
            }
        }
        _ => {
            error!("Invalid path. Input: `{request_line}`");
            ERROR_RESPONSE.to_string()
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

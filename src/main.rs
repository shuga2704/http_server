use anyhow::{bail, Result};
use log::{debug, error};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread::spawn;

mod request;

use request::Request;

const SUCCESS_RESPONSE: &str = "HTTP/1.1 200 OK\r\n\r\n";
const ERROR_RESPONSE: &str = "HTTP/1.1 404 Not Found\r\n\r\n";

fn handle_request(file_directory: PathBuf, mut stream: TcpStream) -> Result<()> {
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
        } else if request.path.starts_with("/files/") {
            let file_path = request.path.split_once("/files/");

            std::fs::write("/tmp/foo", "mango banana apple")?;

            match file_path {
                Some((_, path)) => {
                    debug!("File path requested: `{path}`");

                    let full_path = file_directory.join(path);
                    println!("Full path: {}", full_path.display());
                    match std::fs::read_to_string(&full_path) {
                        Ok(body) => {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                body.len(),
                                body
                            )
                        }
                        Err(_e) => {
                            format!("HTTP/1.1 404 Not Found\r\n\r\n")
                        }
                    }
                }
                _ => {
                    error!("Invalid file path");
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

    let file_directory = std::env::args().nth(2).unwrap_or("/".to_string());
    println!("File directory: {}", file_directory);
    let file_directory = PathBuf::from(file_directory);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let mut response_handles = Vec::new();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let file_directory = file_directory.clone();
                let handle = spawn(|| handle_request(file_directory, stream));
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

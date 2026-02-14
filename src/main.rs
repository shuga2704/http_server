use anyhow::Result;
use log::{debug, error};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

const SUCCESS_RESPONSE: &[u8] = "HTTP/1.1 200 OK\r\n\r\n".as_bytes();
const ERROR_RESPONSE: &[u8] = "HTTP/1.1 404 Not Found\r\n\r\n".as_bytes();

fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                debug!("Accepted new connection");

                let mut request_buffer = BufReader::new(&stream);
                let mut request_line = String::new();
                request_buffer.read_line(&mut request_line)?;

                // Split the first line by space
                // The second token is the path
                // Example: `GET /index.html HTTP/1.1`
                let path = request_line.split_whitespace().collect::<Vec<&str>>();

                match path[..] {
                    ["GET", path, "HTTP/1.1"] => {
                        if path == "/" {
                            debug!("Root path");
                            stream.write(SUCCESS_RESPONSE)?;
                        } else {
                            debug!("Unknown path: `{path}`");
                            stream.write(ERROR_RESPONSE)?;
                        }
                    }
                    _ => {
                        error!("Invalid path. Input: `{request_line}`");
                        stream.write(ERROR_RESPONSE)?;
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}

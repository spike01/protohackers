use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

// TODO: check out blessed.rs

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    method: String,
    number: i32,
}

#[derive(Serialize, Debug)]
struct Response {
    method: String,
    prime: bool,
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    error: String,
    reason: String,
}

const METHOD: &str = "isPrime";

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").expect("unable to bind to port");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_connection(stream));
            }
            Err(err) => println!("{}", err),
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let reader = BufReader::new(stream.try_clone()?);

    println!("Opened stream ip={}", ip(&stream));

    for line in reader.lines() {
        // TODO: look at crates "log" and "tracing"
        let line = line?;

        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(err) => {
                let error = ErrorResponse {
                    error: "invalid_request".to_string(),
                    reason: err.to_string(),
                };
                let bytes_written = stream
                    .write(serialize_error(&error).as_bytes())
                    .expect("unable to write to stream");
                println!(
                    "ip={} bytes_written={} line={} error=invalid_request",
                    ip(&stream),
                    bytes_written,
                    line
                );
                break;
            }
        };

        if request.method != METHOD {
            let error = ErrorResponse {
                error: "invalid".to_string(),
                reason: "invalid method".to_string(),
            };
            let bytes_written = stream
                .write(serialize_error(&error).as_bytes())
                .expect("unable to write to stream");
            println!(
                "ip={} bytes_written={} line={} error=invalid_method",
                ip(&stream),
                bytes_written,
                line
            );
            break;
        };

        let prime = is_prime(request.number);
        let response = Response {
            method: "isPrime".to_string(),
            prime,
        };
        // TODO - look at serde_json:: - Pass stream into serde_json? Look at docs
        let bytes_written = stream
            .write(serialize(&response).as_bytes())
            .expect("unable to write to stream");
        println!(
            "ip={} bytes_written={} line={} prime={}",
            ip(&stream),
            bytes_written,
            line,
            prime
        );
    }

    stream.shutdown(Shutdown::Both)
}

fn ip(stream: &TcpStream) -> IpAddr {
    match stream.try_clone() {
        Ok(stream) => stream.local_addr().expect("could not get local addr").ip(),
        Err(_) => IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
    }
}

fn is_prime(n: i32) -> bool {
    if n <= 0 || n == 1 {
        return false;
    }
    (2..n).all(|a| n % a != 0)
}

// TODO: make generic over Response|ErrorResponse, extract out write_response()
fn serialize(response: &Response) -> String {
    format!(
        "{}\n",
        serde_json::to_string(&response).expect("could not serialize JSON")
    )
}

fn serialize_error(response: &ErrorResponse) -> String {
    format!(
        "{}\n",
        serde_json::to_string(&response).expect("could not serialize JSON")
    )
}

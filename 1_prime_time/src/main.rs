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

trait ResponseJson {
    fn write_to_stream(&self, mut stream: &TcpStream, ip: IpAddr, line: &String, conn: usize)
    where
        Self: Serialize,
    {
        let bytes_written = stream
            .write(ResponseJson::serialize(self).as_bytes())
            .expect("unable to write to stream");
        println!(
            "ip={} bytes_written={} line={} error=invalid_request conn={}",
            ip, bytes_written, line, conn
        );
    }

    fn serialize(&self) -> String
    where
        Self: Serialize,
    {
        format!(
            "{}\n",
            serde_json::to_string(&self).expect("could not serialize to JSON")
        )
    }
}

impl ResponseJson for Response {}
impl ResponseJson for ErrorResponse {}

const METHOD: &str = "isPrime";

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").expect("unable to bind to port");

    for (i, stream) in listener.incoming().enumerate() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_connection(stream, i));
            }
            Err(err) => println!("{}", err),
        }
    }
    Ok(())
}

fn handle_connection(stream: TcpStream, conn: usize) -> std::io::Result<()> {
    let reader = BufReader::new(stream.try_clone()?);
    let ip = ip(&stream);

    println!("Opened stream ip={} conn={}", ip, conn);

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
                error.write_to_stream(&stream, ip, &line, conn);
                break;
            }
        };

        if request.method != METHOD {
            let error = ErrorResponse {
                error: "invalid".to_string(),
                reason: "invalid method".to_string(),
            };
            error.write_to_stream(&stream, ip, &line, conn);
            break;
        };

        let prime = is_prime(request.number);
        let response = Response {
            method: METHOD.to_string(),
            prime,
        };
        // TODO - look at serde_json:: - Pass stream into serde_json? Look at docs
        response.write_to_stream(&stream, ip, &line, conn);
    }

    println!("Closing stream ip={} conn={}", ip, conn);
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
    if n != 2 && n % 2 == 0 {
        return false;
    }
    (2..n / 2).all(|a| n % a != 0)
}

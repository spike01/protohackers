use is_prime::is_prime as other_is_prime;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

// TODO: check out blessed.rs

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    method: String,
    number: String,
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

struct StreamedLine<'a> {
    line: &'a String,
    stream: &'a TcpStream,
    ip: IpAddr,
    conn: usize,
}

trait ResponseJson {
    fn write_to_stream(&self, mut sl: StreamedLine)
    where
        Self: Serialize,
    {
        let response = ResponseJson::serialize(self);
        let bytes_written = sl
            .stream
            .write(response.as_bytes())
            .expect("unable to write to stream");
        println!(
            "ip={} bytes_written={} conn={} line={} response={}",
            sl.ip, bytes_written, sl.conn, sl.line, response
        );
    }

    // `serialize` ensures that each response is terminated with a newline - otherwise the
    // Protohackers checker doesn't recognize responses
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
        let streamed_line = StreamedLine {
            line: &line,
            stream: &stream,
            ip,
            conn,
        };

        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(err) => {
                let error = ErrorResponse {
                    error: "invalid_request".to_string(),
                    reason: err.to_string(),
                };
                error.write_to_stream(streamed_line);
                break;
            }
        };

        if request.method != METHOD {
            let error = ErrorResponse {
                error: "invalid".to_string(),
                reason: "invalid_method".to_string(),
            };
            error.write_to_stream(streamed_line);
            break;
        };

        let prime = is_prime(&request.number);
        let response = Response {
            method: METHOD.to_string(),
            prime,
        };
        // TODO - look at serde_json:: - Pass stream into serde_json? Look at docs
        response.write_to_stream(streamed_line);
    }

    println!("Closing stream ip={} conn={}", ip, conn);
    stream.shutdown(Shutdown::Both)
}

fn ip(stream: &TcpStream) -> IpAddr {
    match stream.try_clone() {
        Ok(stream) => stream.local_addr().expect("could not get local addr").ip(),
        // "default" - would not make sense when deployed
        Err(_) => IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
    }
}

fn is_prime(n: &str) -> bool {
    if n.starts_with('-') {
        return false;
    }
    other_is_prime(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_prime() {
        for prime in vec!["-1", "0", "1", "4", "35934601", "64404236", "9153233"].iter() {
            assert!(!is_prime(*prime), "{} was prime", *prime)
        }
    }

    #[test]
    fn test_prime() {
        for prime in vec!["2", "23693849", "41973671", "71688731"].iter() {
            assert!(is_prime(*prime), "{} was not prime", *prime)
        }
        assert!(is_prime("25896203"));
        assert!(is_prime("32407513"))
    }

    #[test]
    fn test_big_numbers() {
        let big_number = "2393406893135508689922562474977817653928744432857246008";
        assert!(!is_prime(big_number));
    }
}

use is_prime::is_prime as other_is_prime;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

// TODO: check out blessed.rs

#[derive(Serialize, Deserialize, Debug)]
struct Request<'a> {
    method: String,
    #[serde(borrow)]
    number: &'a RawValue,
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
        // TODO: look at crates "log" and "tracing" instead of println!
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
        let line = line?;
        let streamed_line = StreamedLine {
            line: &line,
            stream: &stream,
            ip,
            conn,
        };

        let request: Request = match sanitize_request(&line) {
            Ok(request) => request,
            Err(err) => {
                err.write_to_stream(streamed_line);
                break;
            }
        };

        let prime = is_prime(&request.number.to_string());
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

fn sanitize_request(line: &str) -> Result<Request, ErrorResponse> {
    let request: Request = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(err) => {
            return Err(ErrorResponse {
                error: "invalid_request".to_string(),
                reason: err.to_string(),
            });
        }
    };

    if request.method != METHOD {
        return Err(ErrorResponse {
            error: "invalid".to_string(),
            reason: "invalid_method".to_string(),
        });
    };

    let number_string = &request.number.to_string();
    let mut chars = number_string.chars();
    let first_char = match chars.next() {
        Some(c) => c,
        None => {
            return Err(ErrorResponse {
                error: "invalid".to_string(),
                reason: "empty_field".to_string(),
            });
        }
    };

    if !(first_char == '+' || first_char == '-' || first_char.is_numeric()) {
        return Err(ErrorResponse {
            error: "invalid".to_string(),
            reason: "not_a_number".to_string(),
        });
    }

    if !chars.all(|c| c.is_numeric()) {
        return Err(ErrorResponse {
            error: "invalid".to_string(),
            reason: "not_a_number".to_string(),
        });
    }
    Ok(request)
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

    #[test]
    fn test_parse_json_number_to_string() {
        let big_number = "2393406893135508689922562474977817653928744432857246008";
        let big_num_json = r#"{"number":2393406893135508689922562474977817653928744432857246008,"method":"isPrime"}"#;

        let request: Request = serde_json::from_str(big_num_json).unwrap();

        assert_eq!(request.number.to_string(), big_number);
    }
}

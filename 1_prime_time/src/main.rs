use primal::is_prime as primal_is_prime;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

// TODO: check out blessed.rs

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    method: String,
    number: f32,
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
        let response = ResponseJson::serialize(self);
        let bytes_written = stream
            .write(response.as_bytes())
            .expect("unable to write to stream");
        println!(
            "ip={} bytes_written={} conn={} line={} response={}",
            ip, bytes_written, conn, line, response
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
                reason: "invalid_method".to_string(),
            };
            error.write_to_stream(&stream, ip, &line, conn);
            break;
        };

        if request.number.fract() != 0.0 {
            let response = Response {
                method: METHOD.to_string(),
                prime: false,
            };
            response.write_to_stream(&stream, ip, &line, conn);
        } else {
            let number = request.number as i32;
            let prime = is_prime(number);
            let response = Response {
                method: METHOD.to_string(),
                prime,
            };
            // TODO - look at serde_json:: - Pass stream into serde_json? Look at docs
            response.write_to_stream(&stream, ip, &line, conn);
        }
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

fn is_prime(n: i32) -> bool {
    if n <= 0 {
        return false;
    }
    primal_is_prime(n.try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_prime() {
        for prime in vec![-1, 0, 1, 4, 35934601, 64404236, 9153233].iter() {
            assert!(!is_prime(*prime), "{} was prime", *prime)
        }
    }

    #[test]
    fn test_prime() {
        for prime in vec![2, 23693849, 41973671, 71688731].iter() {
            assert!(is_prime(*prime), "{} was not prime", *prime)
        }
    }

    #[test]
    fn test_casting() {
        let f = 3.7_f32;
        assert!(!(f.fract() == 0.0));

        let g = 3.0_f32;
        assert!(g.fract() == 0.0);
        assert!(is_prime(g as i32));
    }
}

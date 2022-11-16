use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

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
    let mut buf = [0; 1024];
    let ip = stream.local_addr().expect("could not get local addr").ip();

    println!("Opened stream ip={}", ip);

    loop {
        let bytes_read = stream.read(&mut buf).expect("unable to read from stream");

        // Reached EOF
        if bytes_read == 0 {
            break;
        }
        let bytes_written = stream
            .write(&buf[..bytes_read])
            .expect("unable to write to stream");
        println!(
            "ip={} bytes_read={} bytes_written={}",
            ip, bytes_read, bytes_written
        );
        buf = [0; 1024];
    }
    stream.shutdown(Shutdown::Both)
}

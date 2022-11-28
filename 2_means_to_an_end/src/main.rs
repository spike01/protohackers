use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;

    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            match handle_client(stream) {
                Ok(_) => (),
                Err(err) => println!("{}", err),
            };
        });
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buf = [0; 9];
    loop {
        let bytes_read = stream.read(&mut buf)?;
        match bytes_read {
            0 => {
                println!("EOF");
                break;
            }
            9 => handle_message(&buf),
            _ => {
                println!("Wrong message length");
                break;
            }
        }
        let _bytes_written = stream.write(&buf)?;
        buf = [0; 9];
    }
    Ok(())
}

#[derive(Debug)]
enum MessageType {
    Insert,
    Query,
    Undefined
}

// Byte:  |  0  |  1     2     3     4  |  5     6     7     8  |
// Type:  |char |         int32         |         int32         |
#[derive(Debug)]
struct Message {
    type_: MessageType,
    timestamp: i32,
    price: i32,
}

fn handle_message(buf: &[u8; 9]) {
    let type_byte = buf[0];
    let mut timestamp_bytes: [u8; 4] = [0; 4];
    let mut price_bytes: [u8; 4] = [0; 4];

    timestamp_bytes.clone_from_slice(&buf[1..5]);
    price_bytes.clone_from_slice(&buf[5..9]);

    let message = Message {
        type_: match type_byte {
            0x49 => MessageType::Insert,
            0x51 => MessageType::Query,
            _ => MessageType::Undefined,
        },
        timestamp: i32::from_be_bytes(timestamp_bytes),
        price: i32::from_be_bytes(price_bytes),
    };

    match message {
        Message { type_: MessageType::Insert, timestamp, price } => insert(timestamp, price),
        Message { type_: MessageType::Query, timestamp, price } => query(timestamp, price),
        Message { type_: MessageType::Undefined, ..} => println!("Invalid!")
    }
}

fn insert(timestamp: i32, price: i32) {
    println!("I: ({},{})", timestamp, price);
}

fn query(timestamp: i32, price: i32) {
    println!("Q: ({},{})", timestamp, price);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_insert() {
        // Hexadecimal: 49    00 00 30 39    00 00 00 65
        // Decoded:      I          12345            101
        let input = [49,00,00,30,39,00,00,00,65];
        handle_message(&input)
    }

    #[test]
    fn test_handle_query() {
        // Hexadecimal: 51    00 00 03 e8    00 01 86 a0
        // Decoded:      Q           1000         100000
        let input = [51,00,00,03,112,00,01,86,48];
        handle_message(&input)
    }
}

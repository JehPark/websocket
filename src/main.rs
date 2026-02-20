use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    let peer = stream.peer_addr()?;
    println!("client connected: {peer}");

    let mut buffer = [0u8; 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("client disconnected: {peer}");
                return Ok(());
            }
            Ok(n) => {
                stream.write_all(&buffer[..n])?;
            }
            Err(e) => {
                eprintln!("read error on {peer}: {e}");
                return Err(e);
            }
        }
    }
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    println!("blocking echo server running on 127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_client(stream) {
                    eprintln!("failed to handle client: {e}");
                }
            }
            Err(e) => {
                eprintln!("accept error: {e}");
            }
        }
    }

    Ok(())
}

use mio::event::Event;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{self, ErrorKind, Read, Write};

const SERVER: Token = Token(0);

struct Connection {
    stream: TcpStream,
    token: Token,
    write_queue: Vec<u8>,
    write_cursor: usize,
}

impl Connection {
    fn new(stream: TcpStream, token: Token) -> Self {
        Self {
            stream,
            token,
            write_queue: Vec::new(),
            write_cursor: 0,
        }
    }

    fn has_pending_writes(&self) -> bool {
        self.write_cursor < self.write_queue.len()
    }

    fn queue_echo(&mut self, data: &[u8]) {
        self.write_queue.extend_from_slice(data);
    }

    fn flush_writes(&mut self) -> io::Result<bool> {
        while self.has_pending_writes() {
            match self.stream.write(&self.write_queue[self.write_cursor..]) {
                Ok(0) => break,
                Ok(n) => {
                    self.write_cursor += n;
                    if self.write_cursor == self.write_queue.len() {
                        self.write_queue.clear();
                        self.write_cursor = 0;
                        break;
                    }
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Err(err),
            }
        }

        Ok(self.has_pending_writes())
    }

    fn wants_events(&self) -> Interest {
        if self.has_pending_writes() {
            Interest::READABLE | Interest::WRITABLE
        } else {
            Interest::READABLE
        }
    }
}

fn remove_connection(
    poll: &mut Poll,
    connections: &mut HashMap<Token, Connection>,
    token: Token,
    reason: &str,
) {
    if let Some(mut connection) = connections.remove(&token) {
        if let Err(err) = poll.registry().deregister(&mut connection.stream) {
            eprintln!("[{token:?}] deregister error after {reason}: {err}");
        }
    }
}

fn handle_connection_readable(
    poll: &mut Poll,
    event: &Event,
    connection: &mut Connection,
    connections: &mut HashMap<Token, Connection>,
) {
    let token = event.token();
    let mut buffer = [0u8; 4096];
    let mut saw_data = false;

    loop {
        match connection.stream.read(&mut buffer) {
            Ok(0) => {
                println!("[{}] client closed", token.0);
                remove_connection(poll, connections, token, "read eof");
                return;
            }
            Ok(n) => {
                saw_data = true;
                connection.queue_echo(&buffer[..n]);
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(err) => {
                eprintln!("[{token:?}] read error: {err}");
                remove_connection(poll, connections, token, "read error");
                return;
            }
        }
    }

    if saw_data {
        if let Err(err) = connection.flush_writes() {
            eprintln!("[{token:?}] write error: {err}");
            remove_connection(poll, connections, token, "write error");
            return;
        }

        let interests = connection.wants_events();
        if let Err(err) =
            poll.registry()
                .reregister(&mut connection.stream, connection.token, interests)
        {
            eprintln!("[{token:?}] reregister error: {err}");
            remove_connection(poll, connections, token, "reregister error");
            return;
        }
    }
}

fn main() -> io::Result<()> {
    let address: std::net::SocketAddr = "127.0.0.1:7878"
        .parse()
        .expect("valid listener socket address");
    let mut listener = TcpListener::bind(address)?;
    let mut poll = Poll::new()?;
    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)?;
    let mut events = Events::with_capacity(128);
    let mut connections: HashMap<Token, Connection> = HashMap::new();
    let mut next_token = 1usize;

    println!("non-blocking epoll-style echo server on 127.0.0.1:7878");

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    match listener.accept() {
                        Ok((stream, addr)) => {
                            let token = Token(next_token);
                            next_token += 1;

                            let mut connection = Connection::new(stream, token);
                            if let Err(err) = poll.registry().register(
                                &mut connection.stream,
                                connection.token,
                                connection.wants_events(),
                            ) {
                                eprintln!("failed to register client {addr}: {err}");
                                continue;
                            }

                            connections.insert(token, connection);
                            println!("[{}] accepted {addr}", token.0);
                        }
                        Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                        Err(err) => {
                            eprintln!("accept error: {err}");
                            break;
                        }
                    }
                },
                token => {
                    if let Some(connection) = connections.get_mut(&token) {
                        if event.is_readable() {
                            handle_connection_readable(
                                &mut poll,
                                &event,
                                connection,
                                &mut connections,
                            );
                        }

                        if connection.has_pending_writes() && event.is_writable() {
                            if let Err(err) = connection.flush_writes() {
                                eprintln!("[{token:?}] write error: {err}");
                                remove_connection(
                                    &mut poll,
                                    &mut connections,
                                    token,
                                    "write error",
                                );
                                continue;
                            }

                            let interests = connection.wants_events();
                            if let Err(err) = poll.registry().reregister(
                                &mut connection.stream,
                                connection.token,
                                interests,
                            ) {
                                eprintln!("[{token:?}] reregister error: {err}");
                                remove_connection(
                                    &mut poll,
                                    &mut connections,
                                    token,
                                    "reregister error",
                                );
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }
}

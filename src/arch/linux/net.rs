use alloc::vec::Vec;
use std::net;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;

use mio::unix::{EventedFd, UnixReady};
use mio::Ready;
use nix::sys::socket;
use nix::sys::uio;

use log::{debug, info, trace};
use socket2::{Domain, Socket, Type};

use crate::arch::{CmdArgs, CpuId, ThreadId, Transport};
use crate::SashStore;

pub fn server_loop(core: CpuId, tid: ThreadId, config: &CmdArgs, kvstore: &mut SashStore) {
    debug!("Inside server loop on {}", core);
    let connections = connect(tid, config);
    debug!("Opened connection on {:?}", connections);

    let poll = mio::Poll::new().expect("Can't create poll.");

    for (idx, connection) in connections.iter().enumerate() {
        poll.register(
            &EventedFd(&connection.as_raw_fd()),
            mio::Token(idx),
            Ready::readable() | UnixReady::error(),
            mio::PollOpt::edge() | mio::PollOpt::oneshot(),
        )
        .expect("Can't register events.");
    }

    let mut events = mio::Events::with_capacity(10);

    loop {
        poll.poll(&mut events, None).expect("Can't poll channel");
        for event in events.iter() {
            let raw_fd: RawFd = connections[event.token().0].as_raw_fd();
            trace!("event = {:?}", event);

            if event.readiness().is_readable() {
                const MSG_MAX_LEN: usize = 1500;
                let mut recv_buf: Vec<u8> = Vec::with_capacity(MSG_MAX_LEN);
                recv_buf.resize(MSG_MAX_LEN, 0);

                let msg = match socket::recvmsg(
                    raw_fd,
                    &[uio::IoVec::from_mut_slice(&mut recv_buf)],
                    None,
                    socket::MsgFlags::empty(),
                ) {
                    Ok(msg) => msg,
                    Err(e) => panic!("Unexpected error during socket::recvmsg {:?}", e),
                };
                if msg.bytes == 0 {
                    info!("Got 0 bytes, in TCP this means connection got shut-down");
                    return;
                }
                assert!(
                    msg.bytes <= MSG_MAX_LEN,
                    "Got a message bigger than expected"
                );
                // Throw away zeroes at the end of the buffer:
                recv_buf.truncate(msg.bytes);
                let sender: socket::SockAddr = msg.address.unwrap();

                trace!(
                    "recv_buf = {:?}",
                    recv_buf.iter().map(|c| *c as char).collect::<Vec<char>>()
                );

                let send_buf = kvstore.handle_network_request(recv_buf);
                let sent = match socket::sendto(
                    raw_fd,
                    &send_buf.as_slice(),
                    &sender,
                    socket::MsgFlags::empty(),
                ) {
                    Ok(bytes_sent) => bytes_sent,
                    Err(e) => panic!("Unexpected error during socket::send {:?}", e),
                };
                assert!(sent > 0);
            }

            poll.reregister(
                &EventedFd(&raw_fd),
                mio::Token(event.token().0),
                Ready::readable(),
                mio::PollOpt::edge() | mio::PollOpt::oneshot(),
            )
            .expect("Can't re-register events.");
        }
    }
}

#[derive(Debug)]
pub enum Connection {
    Datagram(net::UdpSocket),
    Stream(net::TcpStream),
}

impl AsRawFd for Connection {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Connection::Datagram(s) => s.as_raw_fd(),
            Connection::Stream(s) => s.as_raw_fd(),
        }
    }
}

/// Create a single, TCP or UDP socket
pub fn make_socket(config: &CmdArgs) -> Socket {
    let socket = match config.transport {
        Transport::Tcp => Socket::new(
            Domain::ipv4(),
            Type::stream(),
            Some(socket2::Protocol::tcp()),
        ),
        Transport::Udp => Socket::new(Domain::ipv4(), Type::dgram(), None),
    }
    .expect("Can't create socket");

    socket
        .set_nonblocking(true)
        .expect("Can't set it to blocking mode");
    socket
        .set_reuse_address(true)
        .expect("Can't set reuse addr mode");

    socket
}

fn connect(tid: ThreadId, config: &CmdArgs) -> Vec<Connection> {
    let address = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), (config.port + tid) as u16);
    let mut connections: Vec<Connection> = Default::default();

    match config.transport {
        Transport::Tcp => {
            let socket = make_socket(config);
            socket
                .set_nonblocking(false)
                .expect("Can't unset nonblocking mode for listener");
            socket.bind(&address.into()).expect("Can't bind to address");
            socket.listen(123).expect("Can't listen?");

            let listener = socket.into_tcp_listener();
            for _incoming in 0..config.tcp_connections_per_port {
                info!("Waiting for connection...");
                let (stream, addr) = listener.accept().expect("Waiting for incoming connection");
                info!("Incoming connection from {}", addr);
                stream
                    .set_nonblocking(true)
                    .expect("Can't set nonblocking for incoming connection");
                connections.push(Connection::Stream(stream));
            }
        }
        Transport::Udp => {
            let socket = make_socket(config);
            socket.bind(&address.into()).expect("Can't bind to address");
            connections.push(Connection::Datagram(socket.into_udp_socket()));
        }
    };

    connections
}

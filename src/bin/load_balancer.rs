use core::net::SocketAddr;
use http_load_balancer::read_http_request;
use std::{collections::HashMap, env, io};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> io::Result<()> {
    let (lb_addr, server_addrs) = {
        let mut addrs = env::args()
            .skip(1)
            .map(|addr| addr.parse::<SocketAddr>())
            .filter(|result| result.is_ok() )
            .map(|result| result.unwrap());

        (addrs.next(), addrs.collect::<Vec<SocketAddr>>())
    };

    if lb_addr.is_none() || server_addrs.len() == 0 {
        eprintln!("Usage: {} LB_ADDR SERVER_ADDRS\n", env::args().next().unwrap());
        eprintln!("LB_ADDR      : address and port for the load balancer to listen to");
        eprintln!("SERVER_ADDRS : list of server IP addresses and port");
        return Ok(());
    }
    let lb_addr = lb_addr.unwrap();
    let db = Arc::new(Mutex::new(HashMap::<String, SocketAddr>::new()));


    let listener = TcpListener::bind(&lb_addr).await?;
    println!("[L] Listening on {}", lb_addr);

    let mut next_server_index = 0;

    loop {
        let (mut socket, client_address) = match listener.accept().await {
            Err(e) => {
                println!("[L] Failed to accept client. {:?}", e);
                continue;
            }
            Ok((s, addr)) => {
                println!("[L] New connection to {}", addr);
                (s, addr)
            }
        };

        let server_address = server_addrs[next_server_index].clone();
        next_server_index = (next_server_index + 1) % server_addrs.len();

        tokio::spawn(async move {
            let (request, bytes) = match read_http_request(&mut socket).await {
                Err(e) => {
                    println!("[L] Failed to read HTTP request header. {:?}", e);
                    return;
                }
                Ok(x) => x,
            };

            if let Err(e) = handle_client(socket, client_address, server_address, &bytes).await {
                println!("[L] Failed to connect client {} to server {}. {:?}",
                         client_address,
                         server_address,
                         e
                );
            }
        });
    }
}

async fn handle_client(
    mut client_socket: TcpStream,
    client_addr: SocketAddr,
    server_addr: SocketAddr,
    request_bytes: &[u8],
) -> io::Result<(u64, u64)> {
    let mut server_socket = TcpStream::connect(server_addr).await?;
    println!("[L] Connected client at {} to server at {}.", client_addr, server_addr);

    server_socket.write_all(request_bytes).await?;

    tokio::io::copy_bidirectional(&mut client_socket, &mut server_socket).await
}

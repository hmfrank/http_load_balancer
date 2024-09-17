use http_bytes::{http::{Response, StatusCode}, response_header_to_vec};
use http_load_balancer::{get_session_id, read_http_request_header};
use rand::random;
use std::env;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};


#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = {
        let args: Vec<_> = env::args().collect();

        if args.len() < 2 {
            "127.0.0.1:8080".to_string()
        } else {
            args.into_iter().skip(1).next().unwrap()
        }
    };

    let listener = TcpListener::bind(&addr).await?;
    println!("[S] Listening on {}", addr);

    loop {
        let (socket, client_address) = match listener.accept().await {
            Err(e) => {
                println!("[S] Failed to accept client. {:?}", e);
                continue;
            }
            Ok((s, addr)) => {
                println!("[S] New connection to {}", addr);
                (s, addr)
            }
        };

        let addr = addr.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, &addr).await {
                println!("[S] Failed to handle client at {}. {:?}", client_address, e);
            }
        });
    }
}

async fn handle_client(mut socket: TcpStream, listen_addr: &str) -> io::Result<()> {
    let (request, _) = read_http_request_header(&mut socket).await?;

    match request.uri().path() {
        "/" | "/index.html" => {
            let response_header = Response::builder()
                .status(StatusCode::OK)
                .body(()).unwrap();

            socket.write_all(response_header_to_vec(&response_header).as_slice()).await?;
            socket.write_all(
                &format!(
                    "<html>\n\
                        <head>\n\
                            <title>{}</title>\n\
                        </head>\n\
                        <body>\n\
                            Hello from {}.\n\
                        </body>\n\
                    </html>\n",
                    listen_addr,
                    listen_addr,
                )
                .into_bytes()
            ).await?;
        }
        "/session" => {
            let mut builder = Response::builder();
            let mut response_header = builder.status(StatusCode::OK);

            let sesh_id = get_session_id(request.headers(), "Cookie");
            let set_cookie = sesh_id.is_none();
            let sesh_id = match sesh_id {
                Some(id) => id.to_string(),
                None => {
                    let new_id = format!("{:#034}", random::<u32>());
                    response_header = response_header.header(
                        "Set-Cookie",
                        format!("sessionID={}", new_id)
                    );
                    new_id
                }
            };

            let response_header = response_header.body(()).unwrap();

            socket.write_all(response_header_to_vec(&response_header).as_slice()).await?;
            socket.write_all(
                &format!(
                    "<html>\n\
                        <head>\n\
                            <title>{}</title>\n\
                        </head>\n\
                        <body>\n\
                            Hello from {}.<br>\n\
                            {}\
                            Your session ID is: {}.\n\
                        </body>\n\
                    </html>\n",
                    listen_addr,
                    listen_addr,
                    match set_cookie {
                        true => "A new session has been created.<br>\n",
                        false => "",
                    },
                    sesh_id,
                )
                    .into_bytes()
            ).await?;
        }
        _ => {
            let response_header = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(()).unwrap();

            socket.write_all(response_header_to_vec(&response_header).as_slice()).await?;
            socket.write_all(
                &format!(
                    "<html>\n\
                        <head>\n\
                            <title>{}</title>\n\
                        </head>\n\
                        <body>\n\
                            <img alt=\"404 NOT FOUND\" src=\"https://http.cat/images/404.jpg\">\n\
                        </body>\n\
                    </html>\n",
                    listen_addr,
                )
                    .into_bytes()
            ).await?;
        }
    }

    Ok(())
}

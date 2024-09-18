use crate::{get_session_id, read_http_request_header};
use http_bytes::{http::{Response, StatusCode}, response_header_to_vec};
use rand::random;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

pub struct Server {
	/// Server address to listen to.
	address: String,
	
	/// If true, status messages will be printed to stdout.
	verbose: bool,
}

impl Server {
	pub fn new(address: &str, verbose: bool) -> Self {
		Server {
			address: address.to_string(),
			verbose
		}
	}

	pub async fn run(&self) -> io::Result<()>  {
		// start TCP listener
		let listener = TcpListener::bind(&self.address).await?;
		if self.verbose { println!("[S] Listening on {}", self.address); }

		loop {
			// wait for client
			let (socket, client_address) = match listener.accept().await {
				Err(e) => {
					if self.verbose { println!("[S] Failed to accept client. {:?}", e); }
					continue;
				}
				Ok((s, addr)) => {
					if self.verbose { println!("[S] New connection to {}", addr); }
					(s, addr)
				}
			};

			// handle client
			let address_str = self.address.clone().to_string();
			let verbose = self.verbose;
			tokio::spawn(async move {
				if let Err(e) = Server::handle_client(socket, &address_str).await {
					if verbose {
						println!("[S] Failed to handle client at {}. {:?}", client_address, e);
					}
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
}
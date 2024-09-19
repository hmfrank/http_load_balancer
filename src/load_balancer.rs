use crate::{
	get_session_id,
	read_http_request_header, read_http_response_header
};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

/// HTTP load balancer using a sticky round-robin algorithm.
pub struct LoadBalancer {
	/// Bind address of the load balancer.
	address: SocketAddr,

	/// Stores sticky connections (session ID --> server address)
	db: Arc<Mutex<HashMap<String, SocketAddr>>>,

	/// List of available servers.
	servers: Vec<SocketAddr>,

	/// Index of the server to use for the next connection.
	next_server_index: Arc<Mutex<usize>>,

	/// If true, status messages will be printed to stdout.
	verbose: bool
}

impl LoadBalancer {
	/// Creates a new load balancer.
	///
	/// `server_addrs` must contain at least 1 element, otherwise `None` will be returned.
	pub fn new(address: SocketAddr, server_addrs: &[SocketAddr], verbose: bool) -> Option<Self> {
		if server_addrs.len() == 0 {
			None
		} else {
			Some(LoadBalancer {
				address,
				db: Arc::new(Mutex::new(HashMap::new())),
				servers: server_addrs.to_vec(),
				next_server_index: Arc::new(Mutex::new(0)),
				verbose
			})
		}
	}

	pub async fn run(&self) -> io::Result<()> {
		let listener = TcpListener::bind(&self.address).await?;
		if self.verbose { println!("[L] Listening on {}", self.address); }

		loop {
			// wait for client to connect
			let (mut socket, client_address) = match listener.accept().await {
				Err(e) => {
					if self.verbose { println!("[L] Failed to accept client. {:?}", e); }
					continue;
				}
				Ok((s, addr)) => {
					if self.verbose { println!("[L] New connection to {}", addr); }
					(s, addr)
				}
			};

			// copy variables to move into new task (because we can't move self into the new task).
			let next_server_index = self.next_server_index.clone();
			let serves = self.servers.clone(); // TODO: use reference
			let db = self.db.clone();
			let verbose = self.verbose;

			// start new task to handle client
			tokio::spawn(async move {
				// parse HTTP request header
				let (request, received_bytes) = match read_http_request_header(&mut socket).await {
					Err(e) => {
						if verbose { println!("[L] Failed to read HTTP request header. {:?}", e); }
						return;
					}
					Ok(x) => x,
				};

				// choose server to connect to
				let server_address = {
					let mut create_db_entry = None;

					// choose server based on session ID
					let server_address = match get_session_id(
						request.headers(),
						"Cookie"
					) {
						Some(id) => {
							let db = db.lock().unwrap();

							if db.contains_key(id) {
								Some(db[id])
							} else {
								create_db_entry = Some(id);
								None
							}
						}
						None => None,
					};

					// use round-robin if no session ID was found
					let server_address = match server_address {
						Some(addr) => {
							if verbose {
								println!(
									"[L] Assigned client at {} to server at {} (sticky session).",
									client_address, addr
								);
							}
							addr
						},
						None => {
							let mut index = next_server_index.lock().unwrap();
							let addr = serves[*index];
							*index = (*index + 1) % serves.len();
							if verbose {
								println!(
									"[L] Assigned client at {} to server at {} (round robin).",
									 client_address, addr
								);
							}
							addr
						}
					};

					// create new DB entry, if necessary
					if let Some(id) = create_db_entry {
						let mut db = db.lock().unwrap();
						db.insert(id.to_string(), server_address);

						if verbose {
							println!(
								"[L] Unknown session ID. Added sticky session to DB: {} -> {}",
								id, server_address
							);
						}
					}

					server_address
				};

				// forward client to the selected server
				if let Err(e) = LoadBalancer::handle_client(
					socket, server_address, &received_bytes, db, verbose
				).await {
					if verbose {
						println!(
							"[L] Failed to connect client {} to server {}. {:?}",
							client_address, server_address, e
						);
					}
				}
			});
		}
	}

	async fn handle_client(
		mut client_socket: TcpStream,
		server_address: SocketAddr,
		received_bytes: &[u8],
		db: Arc<Mutex<HashMap<String, SocketAddr>>>,
		verbose: bool,
	) -> io::Result<()> {
		// connect to server
		let mut server_socket = TcpStream::connect(server_address).await?;

		// send already received bytes
		server_socket.write_all(received_bytes).await?;

		// wait for response header
		let (response, received_bytes) = read_http_response_header(
			&mut server_socket
		).await?;

		// create new DB entry, if necessary
		if let Some(id) = get_session_id(response.headers(), "Set-Cookie") {
			let mut db = db.lock().unwrap();
			db.insert(id.to_string(), server_address);

			if verbose {
				println!("[L] Added sticky session to DB: {} -> {}", id, server_address);
			}
		}

		// send already received bytes
		client_socket.write_all(received_bytes.as_slice()).await?;

		// forward the rest of the communication
		tokio::io::copy_bidirectional(&mut client_socket, &mut server_socket).await?;

		Ok(())
	}
}

use http_bytes::{http::Request, parse_request_header_easy};
use std::io;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub async fn read_http_request(socket: &mut TcpStream) -> io::Result<(Request<()>, Vec<u8>)> {
	let mut buffer = vec![0; 4096];
	let mut request_bytes = vec![];

	loop {
		let n = socket.read(buffer.as_mut_slice()).await?;
		if n == 0 {
			break Err(io::Error::new(
				io::ErrorKind::ConnectionReset,
				"socket.read() returned 0"
			));
		}

		request_bytes.extend_from_slice(&buffer[..n]);

		match parse_request_header_easy(&request_bytes) {
			Err(e) => break Err(io::Error::other(e)),
			Ok(None) => {},
			Ok(Some((request, remaining_bytes))) => {
				break Ok((request, request_bytes));
			}
		}
	}
}
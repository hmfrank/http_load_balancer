use http_bytes::{
	Error,
	http::{Request, Response, HeaderMap, HeaderValue},
	parse_request_header_easy, parse_response_header_easy,
};
use std::io;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

trait RequestOrResponse {}

impl<T> RequestOrResponse for Request<T> {}
impl<T> RequestOrResponse for Response<T> {}


pub fn get_session_id<'a, 'b>(
	headers: &'a HeaderMap<HeaderValue>,
	header_name: &'b str
) -> Option<&'a str> {
	for header_val in headers.get_all(header_name).iter() {
		let header_val = match header_val.to_str() {
			Ok(val) => val,
			Err(_) => {
				continue;
			}
		};

		for cookie in header_val.split(";").map(|s| s.trim()) {
			if let Some(index) = cookie.find("=") {
				let name = &cookie[0..index];
				let value = &cookie[index + 1..];

				if name == "sessionID" {
					return Some(value);
				}
			}
		}
	}

	None
}

pub async fn read_http_request_header(socket: &mut TcpStream)
	-> io::Result<(Request<()>, Vec<u8>)> {
	read_http_header::<Request<()>>(socket, parse_request_header_easy).await
}

pub async fn read_http_response_header(socket: &mut TcpStream)
									  -> io::Result<(Response<()>, Vec<u8>)> {
	read_http_header::<Response<()>>(socket, parse_response_header_easy).await
}

async fn read_http_header<R: RequestOrResponse>(
	socket: &mut TcpStream,
	parse_fn: impl Fn(&[u8]) -> Result<Option<(R, &[u8])>, Error>
)
	-> io::Result<(R, Vec<u8>)> {

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

		match parse_fn(&request_bytes) {
			Err(e) => break Err(io::Error::other(e)),
			Ok(None) => {},
			Ok(Some((request, _))) => {
				break Ok((request, request_bytes));
			}
		}
	}
}
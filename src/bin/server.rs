use http_load_balancer::Server;
use std::{ env, io };


#[tokio::main]
async fn main() -> io::Result<()> {
    // parse command line
    let addr = {
        let args: Vec<_> = env::args().collect();

        if args.len() < 2 {
            "127.0.0.1:8080".to_string()
        } else {
            args.into_iter().skip(1).next().unwrap()
        }
    };

    // start server
    let server = Server::new(&addr, true);
    server.run().await
}
extern crate structopt;
#[macro_use] extern crate structopt_derive;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use structopt::StructOpt;
use futures::{Future,Stream};
use futures::future;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core,Handle};
use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response, Service};
use std::path::PathBuf;
use std::net::SocketAddr;

// the command line opts we allow. This
// pulls things from args, and complains
// if it fails to parse them.
#[derive(StructOpt, Debug)]
#[structopt(name = "master", about = "The master node", author = "James Wilson")]
struct Opts {
    #[structopt(short = "w", long = "watcher-address", help = "address for the master to listen on for watcher input", default_value = "0.0.0.0:10000")]
    watcher_address: SocketAddr,

    #[structopt(short = "c", long = "client-address", help = "address for the master to listen on for client requests", default_value = "0.0.0.0:80")]
    client_address: SocketAddr,

    #[structopt(short = "s", long = "static", help = "address to serve static content from (for client)", parse(from_os_str))]
    static_files: PathBuf,
}

fn main() {
    let opts = Opts::from_args();

    println!("Starting master:");
    println!("- watcher address: {}", opts.watcher_address);
    println!("- client address:  {}", opts.client_address);
    println!("- static file loc: {:?}", opts.static_files);

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // kick off our watcher and client servers onto the event loop:
    spawn_server(&opts.client_address, ClientServer{ files: opts.static_files }, handle.clone());
    spawn_server(&opts.watcher_address, WatcherServer, handle);

    // run the core forever; this will ensure any futures
    // spawned onto it from above are handled.
    core.run(future::empty::<(), ()>()).unwrap();

}

// A helper function to bind our server to a socket address and an event loop handle
fn spawn_server<S>(addr: &SocketAddr, factory: S, handle: Handle)
    where S: Service<Request = Request, Response = Response, Error = hyper::Error> + Clone + 'static
 {

    let http = Http::new();
    let listener = TcpListener::bind(&addr, &handle).unwrap();

    handle.clone().spawn(
        listener.incoming().for_each(move |(socket, addr)| {
            http.bind_connection(&handle, socket, addr, factory.clone());
            Ok(())
        }).map_err(|_| ())
    );
}

// a client server:
#[derive(Clone)]
struct ClientServer {
    files: PathBuf
}

impl Service for ClientServer {

    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, _req: Request) -> Self::Future {
        Box::new(futures::future::ok(
            Response::new()
                .with_header(ContentLength("client".len() as u64))
                .with_body("client")
        ))
    }
}

// a watcher server:
#[derive(Clone,Copy)]
struct WatcherServer;

impl Service for WatcherServer {

    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, _req: Request) -> Self::Future {
        Box::new(futures::future::ok(
            Response::new()
                .with_header(ContentLength("watcher".len() as u64))
                .with_body("watcher")
        ))
    }
}
extern crate structopt;
#[macro_use] extern crate structopt_derive;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde;
#[macro_use] extern crate serde_json;
extern crate hyper_staticfile;

extern crate lib;

use lib::shared::types::*;
use lib::shared::timings;
use lib::master::state::State;

use structopt::StructOpt;
use futures::{Future, Stream};
use futures::future;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle, Interval};
use hyper::{Method, StatusCode};
use hyper::header::{ContentType, ContentLength};
use hyper::server::{Http, Request, Response, Service};
use std::path::{PathBuf, Path};
use std::net::SocketAddr;
use hyper_staticfile::Static;

// the command line opts we allow. This
// pulls things from args, and complains
// if it fails to parse them.
#[derive(StructOpt, Debug)]
#[structopt(name = "master", about = "The master node", author = "James Wilson")]
struct Opts {
    #[structopt(short = "w", long = "watcher-address", help = "address for the master to listen on for watcher input", default_value = "0.0.0.0:10000")]
    watcher_address: SocketAddr,

    #[structopt(short = "c", long = "client-address", help = "address for the master to listen on for client requests", default_value = "0.0.0.0:8080")]
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

    // these get moved into the ClientServer closure, below, so that
    // we can use references to them each time it runs.
    let handle2 = handle.clone();
    let static_files = opts.static_files;

    // shared state; makea few copies to hand around to things:
    let state = State::new();
    let state2 = state.clone();
    let state3 = state.clone();

    // kick off our watcher and client servers on the event loop:
    spawn_server(&opts.client_address, move || ClientServer::new(state.clone(), &static_files, &handle2), handle.clone());
    spawn_server(&opts.watcher_address, move || WatcherServer::new(state2.clone()), handle.clone());
    periodic_cleanup(state3, &handle);

    // run the core forever; this will ensure any futures
    // spawned onto it from above are handled.
    core.run(future::empty::<(), ()>()).unwrap();

}

//
// cleanup handler:
// ################

fn periodic_cleanup(state: State, handle: &Handle) {
    let expiration = timings::expiration();
    let cleanup = Interval::new(expiration, &handle).unwrap().for_each(move |_| {
        state.remove_older_than(expiration);
        Ok(())
    }).map_err(|_| ());

    handle.spawn(cleanup);
}

//
// a client server:
// ################

struct ClientServer {
    state: State,
    static_files: Static<NotFoundHandler>
}

impl ClientServer {
    fn new(state: State, static_files: &Path, handle: &Handle) -> ClientServer {
        ClientServer {
            state: state,
            static_files: Static::with_upstream(handle, static_files, NotFoundHandler)
        }
    }
}

impl Service for ClientServer {

    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        match (req.method(), req.path()) {
            // return our current file list:
            (&Method::Get, "/api/list") => {

                // we wrap the list of items in a struct to match
                // the Go implementation:
                let out = json!({
                    "Files": self.state.list()
                });

                let out = serde_json::to_string(&out).unwrap();
                let res = Response::new()
                    .with_header(ContentLength(out.len() as u64))
                    .with_header(ContentType::json())
                    .with_body(out);

                Box::new(future::ok(res))

            },
            // try to serve static content if we ask for anything else:
            _ => {
                self.static_files.call(req)
            }
        }
    }
}

//
// a watcher server:
// #################

#[derive(Clone)]
struct WatcherServer {
    state: State
}

impl WatcherServer {
    fn new(state: State) -> WatcherServer {
        WatcherServer {
            state: state
        }
    }
}

impl Service for WatcherServer {

    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {

        if req.method() != &Method::Post {
            return response_str(StatusCode::MethodNotAllowed, "Only POST requests allowed");
        }

        if req.headers().get::<ContentType>() != Some(&ContentType::json()) {
            return response_str(StatusCode::UnsupportedMediaType, "Expected application/json Content-Type");
        }

        let state = self.state.clone();
        let res = req.body().concat2().and_then(move |body| {

            let out: FromWatcher = match serde_json::from_slice(&body) {
                Err(_) => return response_str(StatusCode::BadRequest, "Couldn't decode JSON"),
                Ok(diff) => diff
            };

            if out.first {
                state.set(out.id, out.diff.added);
            } else {
                state.update(out.id, out.diff);
            }

            response_str(StatusCode::Ok, "Thanks!")

        });

        Box::new(res)
    }
}

//
// Not Found handler:
// ##################

#[derive(Clone)]
pub struct NotFoundHandler;

impl Service for NotFoundHandler {

    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let (code, body) = match req.method() {
            &Method::Head | &Method::Get => (
                StatusCode::NotFound,
                "404 Not found :("
            ),
            _ => (
                StatusCode::BadRequest,
                "403 Bad Request :'("
            )
        };

        response_str(code, body)
    }
}

// a quick util to generate a response given a status code and message string:
fn response_str(status: StatusCode, message: &'static str) -> Box<Future<Item=Response, Error=hyper::Error>> {
    Box::new(future::ok(
        Response::new()
            .with_status(status)
            .with_header(ContentLength(message.len() as u64))
            .with_body(message)
    ))
}

// A helper function to bind our server to a socket address and an event loop handle
fn spawn_server<S, F>(addr: &SocketAddr, factory: F, handle: Handle)
    where S: Service<Request = Request, Response = Response, Error = hyper::Error> + 'static,
          F: Fn() -> S + 'static
 {

    let http = Http::new();
    let listener = TcpListener::bind(&addr, &handle).expect("failed to bind TCP port");

    handle.clone().spawn(
        listener.incoming().for_each(move |(socket, addr)| {
            http.bind_connection(&handle, socket, addr, factory());
            Ok(())
        }).map_err(|_| ())
    );
}
extern crate structopt;
#[macro_use] extern crate structopt_derive;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate futures_cpupool;
extern crate rand;
extern crate serde;
extern crate serde_json;

extern crate lib;

use lib::shared::types::*;
use lib::shared::timings;

use rand::{thread_rng, Rng};
use structopt::StructOpt;
use std::path::{PathBuf,Path};
use futures::{Future,Stream};
use futures_cpupool::CpuPool;
use tokio_core::reactor::{Core,Interval};
use hyper::{Client,Uri,Request,Method};
use hyper::header::{ContentType,ContentLength};
use std::fs;
use std::io;
use std::fmt;
use std::hash::Hash;
use std::collections::HashSet;
use std::rc::Rc;
use std::cell::Cell;
use std::sync::{Mutex,Arc};

// the command line opts we allow. This
// pulls things from args, and complains
// if it fails to parse them.
#[derive(StructOpt, Debug)]
#[structopt(name = "watcher", about = "A watcher node", author = "James Wilson")]
struct Opts {
    #[structopt(short = "f", long = "folder", help = "point to the folder you'd like to watch", default_value = ".", parse(from_os_str))]
    folder: PathBuf,

    #[structopt(short = "m", long = "master", help = "address of the master", default_value = "http://127.0.0.1:10000")]
    master: Uri,

    #[structopt(short = "id", long = "id")]
    id: Option<String>,
}

fn main() {
    let opts = Opts::from_args();

    let id = opts.id.unwrap_or_else(|| thread_rng().gen_ascii_chars().take(10).collect());
    let master = opts.master;
    let folder = Arc::new(opts.folder);

    println!("Starting watcher:");
    println!("- ID:     {}", id);
    println!("- master: {}", master);
    println!("- folder: {:?}", folder);

    // we don't really need Futures and things, and could happily run such a
    // simple thing using basic sync code, but I want to have a play with them
    // so here goes :)
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let pool = CpuPool::new(1);
    let interval = Interval::new(timings::update(), &handle).unwrap();

    // reuse the same client for connection pooling etc:
    let client = Client::new(&handle);

    // our state; keep track of what the last files we saw were, and whether
    // this is a "first" response.
    let last_files = Arc::new(Mutex::new(vec![]));
    let is_first = Rc::new(Cell::new(true));

    // repeat this every 500ms:
    let work = interval.for_each(|_| {

        let id = id.clone();
        let folder = folder.clone();
        let uri = master.clone();
        let client = client.clone();

        let last_files = last_files.clone();
        let last_files2 = last_files.clone();

        let is_first = is_first.clone();
        let is_first2 = is_first.clone();

        let work = pool.spawn_fn(move || {

            // on another thread, get last_files and calculate
            // the diff, returning it if successful. This is such
            // a small job it's hardly worth the effort of a cpupool,
            // but It's here to have a go at handling an expensive task
            // effectively
            let mut last_files = last_files.lock().unwrap();
            let curr = list_files_in_dir(&folder).map_err(Error::Io)?;
            let diff = owned_diff(&last_files, &curr);
            *last_files = curr;
            Ok(diff)

        }).and_then(move |diff| {

            // produce our output and JSONify it:
            let out = FromWatcher {
                id: id,
                diff: diff,
                first: is_first.get()
            };

            let mut req = Request::new(Method::Post, uri);
            let files_json = serde_json::to_string(&out).unwrap();

            // set up the client request and make it:
            req.headers_mut().set(ContentLength(files_json.len() as u64));
            req.headers_mut().set(ContentType::json());
            req.set_body(files_json);

            client.request(req)
                .map_err(Error::Hyper)
                .and_then(|res| {
                    if !res.status().is_success() {
                        Err(Error::BadResponse(res.status()))
                    } else {
                        Ok(res)
                    }
                })

        }).then(move |res| {

            // check for response success/error and complain if it
            // wasn't successful/an error occurred, setting our state
            // back to initial values to resend a first thing on failure:
            match res {
                Err(e) => {
                    println!("{}", e);
                    is_first2.set(true);
                    *last_files2.lock().unwrap() = vec![];
                },
                Ok(_) => {
                    is_first2.set(false);
                }
            };
            futures::future::ok(())

        });

        // spawn the "work" off to separate it from the interval,
        // so that the interval is not blocked. This means that, even if
        // sending the file list off took 250ms, we'd still send one off
        // every 500ms. Otherwise, the interval would block and take 750ms
        // each run. It's here to have a play more than anything though,
        // since we don't handle the case where listing files takes longer than
        // 500ms and causes work to gradually build up.
        handle.spawn(work);
        Ok(())

    });

    // we never expect to get past this line, since work is
    // a never ending stream, but if we do, print the error
    // before exiting:
    if let Err(e) = core.run(work) {
        println!("{}", e);
    }

}

// list files in the path provided, complaining if we hit a snag:
fn list_files_in_dir(dir: &Path) -> io::Result<Vec<Item>> {

    let mut items = vec![];
    for file in fs::read_dir(dir)? {

        let item = file?;
        let is_dir = item.path().is_dir();
        let name = item.file_name().to_string_lossy().into_owned();

        items.push(Item {
            name: name,
            ty: if is_dir { Type::Folder } else { Type::File }
        })
    }
    Ok(items)

}

// create an owned diff between two T's. Futures make it annoying for said diff
// not to be one of owned values:
fn owned_diff<'a, T: Eq + Hash + Clone>(old: &'a [T], new: &'a [T]) -> Diff<T> {

    let old_set: HashSet<&T> = old.iter().collect();
    let new_set: HashSet<&T> = new.iter().collect();

    let added = new_set.difference(&old_set).map(|a| *a).cloned().collect();
    let removed = old_set.difference(&new_set).map(|a| *a).cloned().collect();

    Diff {
        added: added,
        removed: removed
    }

}

#[derive(Debug)]
enum Error {
    Hyper(hyper::Error),
    Io(io::Error),
    BadResponse(hyper::StatusCode)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::BadResponse(status) => write!(f, "Bad response code: {}", status.as_u16()),
            &Error::Hyper(ref e) => write!(f, "HTTP Error: {}", e),
            &Error::Io(ref e) => write!(f, "IO Error: {}", e),
        }
    }
}
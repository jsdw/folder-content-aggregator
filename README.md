# Aggregated Folder List

A quick and robust toy implementation wherein multiple nodes, each watching a single folder, send the current content listings to a single master node (in the form of diffs). The master keeps track of the final list of files, and makes it available for a (very quick and dirty) client, whose sole task is displaying said list.

The implementation is robust in the face of any watcher node or the master node going down for any period of time, and adapts as watchers are added or removed.

# Install

The backend bits require Go 1.9. The client requires a relatively modern browser to accomodate `fetch` and arrow functions.

```
cd server/go
export GOPATH=$(pwd)
go install ...
```

If you'd like to install the rust version of the watcher, you'll need to have `cargo` installed, and can then do so with:

```
cd server/rust
cargo install
```

# Running

Each watcher node can be started like so:

```
./server/go/bin/watcher --folder /folder/to/watch --id unique-id-for-this-watcher
```

Where "id" is optional and will be auto-generated if not provided (but is useful for consistency in the face of watchers being killed and restarted).

The rust watcher runs in the same way, and allows for the same arguments to be provided.

The master can be started like so:

```
./server/go/bin/master --static client/ --client-address 0.0.0.0:9090
```

This allows one to visit the quick client implementation (client/index.html) at `localhost:9090` (or your machines IP address) to view the current aggregated file list.

Run either binary with `--help` to see all possible commands, including how to customise which port the master and clients communicate on.

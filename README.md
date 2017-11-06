# Aggregated Folder List

A quick and robust toy implementation wherein multiple nodes, each watching a single folder, send the current content listings to a single master node (in the form of diffs). The master keeps track of the final list of files, and makes it available for a (very quick and dirty) client, whose sole task is displaying said list.

The implementation is robust in the face of any watcher node or the master node going down for any period of time, and adapts as watchers are added or removed.

# Install

Requires Go 1.9

```
cd server
export GOPATH=$(pwd)
go install ...
```

# Running

Each watcher node can be started like so:

```
./server/bin/watcher --folder /folder/to/watch --id unique-id-for-this-watcher
```

Where "id" is optional and will be auto-generated if not provided (but is useful for consistency in the face of watchers being killed and restarted).

The master can be started like so:

```
./server/bin/master --static client/ --client-address 0.0.0.0:9090
```

This allows one to visit the quick client implementation (client/index.html) at `localhost:9090` (or your machines IP address) to view the current aggregated file list.

Run either binary with `--help` to see all possible commands, including how to customise which port the master and clients communicate on.

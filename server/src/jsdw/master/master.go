package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io/ioutil"
	"jsdw/master/files"
	"jsdw/types"
	"log"
	"net/http"
	"os"
	"time"
)

func main() {

	watcherAddress := flag.String("watcher-address", "0.0.0.0:10000", "address for the master to listen on for watcher input")
	clientAddress := flag.String("client-address", "0.0.0.0:80", "address for the master to listen on for client requests")
	staticFiles := flag.String("static", "", "address to serve static content from (for client)")

	flag.Parse()

	opts := options{
		watcherAddress: *watcherAddress,
		clientAddress:  *clientAddress,
		staticFiles:    *staticFiles,
	}

	if opts.staticFiles == "" {
		log.Printf("Need to provide a static file path using --static")
		os.Exit(1)
	}

	log.Println("Starting master:")
	log.Printf("- watcher address: %s", opts.watcherAddress)
	log.Printf("- client address:  %s", opts.clientAddress)
	log.Printf("- static file loc: %s", opts.staticFiles)

	master(opts)
}

type options struct {
	watcherAddress string
	clientAddress  string
	staticFiles    string
}

func master(opts options) {
	details := files.Details{}
	go startClientServer(opts, &details)
	go startWatcherServer(opts, &details)
	startPeriodicCleanup(&details)
}

func startPeriodicCleanup(details *files.Details) {

	for {
		now := time.Now()
		details.Set(func(list files.Files) files.Files {

			newInfo := files.Files{}
			for id, info := range list {
				if now.Sub(info.LastUpdated) < 10*time.Second {
					newInfo[id] = info
				}
			}
			return newInfo

		})
		time.Sleep(10 * time.Second)
	}

}

func startClientServer(opts options, details *files.Details) {

	mux := http.NewServeMux()

	// Get the list of files:
	mux.Handle("/api/list", http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {

		if req.URL.Path != "/api/list" {
			http.NotFound(w, req)
			return
		}
		if req.Method != http.MethodGet {
			w.WriteHeader(http.StatusMethodNotAllowed)
			return
		}

		now := time.Now()

		details.Get(func(list files.Files) {

			type OutputItem struct {
				Name  string
				Type  string
				From  string
				Stale bool
			}
			type Output struct {
				Files []OutputItem
			}

			out := Output{}

			for id, info := range list {
				isStale := now.Sub(info.LastUpdated) > 2*time.Second
				for _, file := range info.Files {
					out.Files = append(out.Files, OutputItem{
						Name:  file.Name,
						Type:  file.Type,
						From:  id,
						Stale: isStale,
					})
				}
			}

			res, err := json.Marshal(out)
			if err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				return
			}

			w.Header().Add("Content-Type", "application/json")
			w.Write(res)

		})

	}))

	// Serve static content:
	mux.Handle("/", http.FileServer(http.Dir(opts.staticFiles)))

	server := http.Server{
		Handler: mux,
		Addr:    opts.clientAddress,
	}

	err := server.ListenAndServe()
	if err != nil {
		log.Printf("Error starting client server: %v", err)
	}
}

func startWatcherServer(opts options, details *files.Details) {

	// a route which, given diffs to apply, updates our current files.Details
	// based on them.
	handler := http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {

		if req.Method != http.MethodPost {
			w.WriteHeader(http.StatusMethodNotAllowed)
			w.Write([]byte("Only POST requests allowed"))
			return
		}

		if req.Header.Get("Content-Type") != "application/json" {
			w.WriteHeader(http.StatusUnsupportedMediaType)
			w.Write([]byte("Expected application/json Content-Type"))
			return
		}

		body, err := ioutil.ReadAll(req.Body)
		if err != nil {
			w.WriteHeader(http.StatusBadRequest)
			w.Write([]byte(fmt.Sprintf("Unable to read body: %v", err)))
		}

		diff := types.FromWatcher{}
		err = json.Unmarshal(body, &diff)
		if err != nil {
			w.WriteHeader(http.StatusBadRequest)
			w.Write([]byte(fmt.Sprintf("Unable to decode body into valid diffs: %v", err)))
		}

		applyDiffToFiles(diff, details)

	})

	server := http.Server{
		Handler: handler,
		Addr:    opts.watcherAddress,
	}

	err := server.ListenAndServe()
	if err != nil {
		log.Printf("Error starting watcher server: %v", err)
	}

}

func applyDiffToFiles(diff types.FromWatcher, details *files.Details) {

	removed := diff.Diff.Removed
	removedHash := map[string]struct{}{}
	for _, file := range removed {
		removedHash[file.Name] = struct{}{}
	}

	details.Set(func(list files.Files) files.Files {

		now := time.Now()
		curr := list[diff.ID]

		// we'll update our file list based on the diff, so start from empty
		newFiles := []types.FileInfo{}

		// add any files to our new list that haven't been removed in the diff,
		// unless this is our first time getting the list, in which case we start fresh
		if !diff.First {
			for _, file := range curr.Files {
				if _, removed := removedHash[file.Name]; !removed {
					newFiles = append(newFiles, file)
				}
			}
		}

		// append added files:
		for _, file := range diff.Diff.Added {
			newFiles = append(newFiles, file)
		}

		// dedupe by name:
		newFiles = dedupeByName(newFiles)

		// add to the aggregate list and return it:
		list[diff.ID] = files.Info{
			LastUpdated: now,
			Files:       newFiles,
		}
		return list

	})

}

func dedupeByName(list []types.FileInfo) []types.FileInfo {

	newList := []types.FileInfo{}
	seen := map[string]struct{}{}

	for _, val := range list {
		if _, seenBefore := seen[val.Name]; !seenBefore {
			newList = append(newList, val)
		} else {
			seen[val.Name] = struct{}{}
		}
	}

	return newList

}

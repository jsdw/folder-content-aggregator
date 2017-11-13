package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io/ioutil"
	"jsdw/master/diff"
	"jsdw/master/files"
	"jsdw/shared/timings"
	"jsdw/shared/types"
	"log"
	"net/http"
	"os"
	"time"
)

type options struct {
	watcherAddress string
	clientAddress  string
	staticFiles    string
}

func main() {

	watcherAddress := flag.String("watcher-address", "0.0.0.0:10000", "address for the master to listen on for watcher input")
	clientAddress := flag.String("client-address", "0.0.0.0:8080", "address for the master to listen on for client requests")
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

func master(opts options) {
	details := files.New()
	go startClientServer(opts, details)
	go startWatcherServer(opts, details)
	startPeriodicCleanup(details)
}

// serve the static content and provide a simple API to get the current file list
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

		// access the file list and read it into our output struct:
		details.Access(func(list *files.Files) {
			for id, info := range *list {
				isStale := now.Sub(info.LastUpdated) > timings.Stale
				for _, file := range info.Files {
					out.Files = append(out.Files, OutputItem{
						Name:  file.Name,
						Type:  file.Type,
						From:  id,
						Stale: isStale,
					})
				}
			}
		})

		res, err := json.Marshal(out)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			w.Write([]byte("Unable to encode list into valid JSON"))
			return
		}

		w.Header().Add("Content-Type", "application/json")
		w.Write(res)

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

// start the watcher server to take status updates from watchers
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
			return
		}

		diffs := types.FromWatcher{}
		err = json.Unmarshal(body, &diffs)
		if err != nil {
			w.WriteHeader(http.StatusBadRequest)
			w.Write([]byte(fmt.Sprintf("Unable to decode body into valid diffs: %v", err)))
			return
		}

		diff.ApplyToFiles(diffs, details)

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

// remove files whose status has not been updated lately
func startPeriodicCleanup(details *files.Details) {

	for {
		now := time.Now()
		details.Access(func(list *files.Files) {

			newInfo := files.Files{}
			for id, info := range *list {
				if now.Sub(info.LastUpdated) < timings.Expiration {
					newInfo[id] = info
				}
			}
			*list = newInfo

		})
		time.Sleep(10 * time.Second)
	}

}

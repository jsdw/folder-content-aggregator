package main

import (
	"bytes"
	"crypto/rand"
	"encoding/json"
	"flag"
	"fmt"
	"io/ioutil"
	"jsdw/shared/timings"
	"jsdw/shared/types"
	"jsdw/watcher/diff"
	"log"
	"net/http"
	"time"
)

func main() {

	folderPath := flag.String("folder", ".", "point to the folder you'd like to watch")
	masterAddress := flag.String("master", "http://127.0.0.1:10000", "the address and port of the master")
	uniqueID := flag.String("id", uuid(), "unique ID identifying this watcher")

	flag.Parse()

	log.Println("Starting watcher:")
	log.Printf("- ID:     %s", *uniqueID)
	log.Printf("- master: %s", *masterAddress)
	log.Printf("- folder: %s", *folderPath)

	watcher(*folderPath, *masterAddress, *uniqueID)
}

func watcher(path string, address string, uuid string) {

	isFirst := true
	lastFiles := []types.FileInfo{}
	client := http.Client{Timeout: 100 * time.Millisecond}

	for {

		nextFiles, err := listFilesInDir(path)
		if err != nil {
			log.Printf("Error reading directory (%s): %v", path, err)
		}

		// we can continue despite an error (the directory will just look empty).
		diff := diff.Make(lastFiles, nextFiles)

		// try sending diff to master. if we succeed, update for next diff, else
		// prepare to re-send everything on next attempt incase master restarted.
		err = sendDiffToMaster(isFirst, client, diff, address, uuid)
		if err != nil {
			log.Printf("Error sending diff to master (%s): %v", address, err)
			isFirst = true
			lastFiles = []types.FileInfo{}
		} else {
			lastFiles = nextFiles
			isFirst = false
		}

		// wait a little before trying again. One area for improvement could be to
		// spin off one goroutine to update the file listing, and another to periodically
		// send updates to the master; the worry as it stands is that, if getting the file
		// list becomes slow, updates to the master will become slow enough that the master
		// marks the files as stale.
		time.Sleep(timings.UpdateInterval)

	}

}

func sendDiffToMaster(isFirst bool, client http.Client, diff types.FileInfoDiff, address string, uuid string) error {

	res := types.FromWatcher{
		ID:    uuid,
		Diff:  diff,
		First: isFirst,
	}

	b, err := json.Marshal(res)
	if err != nil {
		return err
	}

	resp, err := client.Post(address, "application/json", bytes.NewReader(b))
	if err != nil {
		return err
	}

	if resp.StatusCode < 200 || resp.StatusCode > 299 {
		body, _ := ioutil.ReadAll(resp.Body)
		return fmt.Errorf("unexpected response from master (%d %s): %s", resp.StatusCode, resp.Status, string(body))
	}

	resp.Body.Close()
	return nil

}

// lists the files in the provided directory:
func listFilesInDir(path string) ([]types.FileInfo, error) {
	files, err := ioutil.ReadDir(path)
	if err != nil {
		return nil, err
	}

	names := make([]types.FileInfo, 0, len(files))
	for _, file := range files {

		ty := "file"
		if file.IsDir() {
			ty = "directory"
		}

		f := types.FileInfo{
			Name: file.Name(),
			Type: ty,
		}

		names = append(names, f)
	}

	return names, nil
}

// generate a unique random token:
func uuid() (uuid string) {

	b := make([]byte, 16)
	_, err := rand.Read(b)
	if err != nil {
		fmt.Println("Error: ", err)
		return
	}

	uuid = fmt.Sprintf("%X%X%X%X%X", b[0:4], b[4:6], b[6:8], b[8:10], b[10:])

	return
}

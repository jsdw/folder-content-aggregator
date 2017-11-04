package files

import (
	"jsdw/types"
	"sync"
	"time"
)

// Details stores a list of files
type Details struct {
	lock  sync.Mutex
	files Files
}

// Files is a map from uuid to file info
type Files = map[string]Info

// Info is a list of file details plus last updated timestamp
type Info struct {
	LastUpdated time.Time
	Files       []types.FileInfo
}

// Get allows safe access to the files contained within:
func (list *Details) Get(fn func(Files)) {
	list.lock.Lock()
	defer list.lock.Unlock()
	fn(list.files)
}

// Set allows safe access to the files contained within:
func (list *Details) Set(fn func(Files) Files) {
	list.lock.Lock()
	defer list.lock.Unlock()
	list.files = fn(list.files)
}

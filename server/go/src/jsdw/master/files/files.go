package files

import (
	"jsdw/shared/types"
	"sync"
	"time"
)

// Details stores a list of files
type Details struct {
	sync.Mutex
	files Files
}

// Files is a map from uuid to file info
type Files = map[string]Info

// Info is a list of file details plus last updated timestamp
type Info struct {
	LastUpdated time.Time
	Files       []types.FileInfo
}

// New creates a new Info struct
func New() *Details {
	return &Details{
		files: map[string]Info{},
	}
}

// Access allows safe access to the files contained within:
func (list *Details) Access(fn func(*Files)) {
	list.Lock()
	defer list.Unlock()

	files := list.files
	fn(&files)
	list.files = files
}

package types

// FileInfo is some basic file information
type FileInfo struct {
	Name string
	Type string
}

// FileInfoDiff describes the difference between two lists of file info
type FileInfoDiff struct {
	Added   []FileInfo
	Removed []FileInfo
}

// FromWatcher is sent from watcher to master
type FromWatcher struct {
	ID    string
	Diff  FileInfoDiff
	First bool
}

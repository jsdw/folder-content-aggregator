package diff

import (
	"jsdw/master/files"
	"jsdw/shared/types"
	"time"
)

// ApplyToFiles takes a diff and file Details, and updates file details accordingly
func ApplyToFiles(diff types.FromWatcher, details *files.Details) {

	removed := diff.Diff.Removed
	removedHash := map[string]struct{}{}
	for _, file := range removed {
		removedHash[file.Name] = struct{}{}
	}

	details.Access(func(list *files.Files) {

		now := time.Now()
		curr := (*list)[diff.ID]

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
		(*list)[diff.ID] = files.Info{
			LastUpdated: now,
			Files:       newFiles,
		}

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

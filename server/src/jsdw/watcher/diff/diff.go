package diff

import "jsdw/shared/types"

// Make creates a diff based on the difference between two lists of files
func Make(a []types.FileInfo, b []types.FileInfo) types.FileInfoDiff {
	return types.FileInfoDiff{
		Added:   diffAdded(a, b),
		Removed: diffRemoved(a, b),
	}
}

func diffAdded(a []types.FileInfo, b []types.FileInfo) []types.FileInfo {

	added := []types.FileInfo{}

	aHash := map[string]struct{}{}
	for _, file := range a {
		aHash[file.Name] = struct{}{}
	}

	for _, file := range b {
		if _, found := aHash[file.Name]; !found {
			added = append(added, file)
		}
	}

	return added

}

func diffRemoved(a []types.FileInfo, b []types.FileInfo) []types.FileInfo {

	removed := []types.FileInfo{}

	bHash := map[string]struct{}{}
	for _, file := range b {
		bHash[file.Name] = struct{}{}
	}

	for _, file := range a {
		if _, found := bHash[file.Name]; !found {
			removed = append(removed, file)
		}
	}

	return removed

}

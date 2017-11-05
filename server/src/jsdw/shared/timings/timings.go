package timings

import "time"

// UpdateInterval is how often the watcher waits between updates to master
const UpdateInterval = 500 * time.Millisecond

// Expiration is how long to files live before being removed
const Expiration = UpdateInterval * 10

// Stale is how long a file takes to become stale when not updated
const Stale = UpdateInterval * 4

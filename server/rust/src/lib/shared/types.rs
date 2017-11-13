// everything that can be sent is mangled to look like the Go values sent
// from the Go version of this watcher (though capitalisation shouldn't strictly be
// necessary).

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct FromWatcher {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Diff")]
    pub diff: Diff<Item>,
    #[serde(rename = "First")]
    pub first: bool
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Diff<T> {
    #[serde(rename = "Added")]
    pub added: Vec<T>,
    #[serde(rename = "Removed")]
    pub removed: Vec<T>
}

#[derive(Debug,Serialize,Deserialize,Clone,Hash,Eq,Ord,PartialEq,PartialOrd)]
pub struct Item {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub ty: Type
}

#[derive(Debug,Serialize,Deserialize,Copy,Clone,Hash,Eq,Ord,PartialEq,PartialOrd)]
pub enum Type {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "directory")]
    Folder
}
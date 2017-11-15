use std::sync::{Arc, Mutex};
use std::collections::{HashMap,HashSet};
use std::time::{Instant,Duration};
use shared::types::*;

//
// our shared state:
//
#[derive(Clone)]
pub struct State {
    state: Arc<Mutex<HashMap<String,Info>>>
}
pub struct Info {
    last_updated: Instant,
    files: Vec<Item>
}

impl State {
    pub fn new() -> State {
        State {
            state: Arc::new(Mutex::new(HashMap::new()))
        }
    }
    pub fn list(&self) -> Vec<ItemList> {

        let state = self.state.lock().unwrap();

        let mut out = vec![];
        let now = Instant::now();
        for (key,info) in state.iter() {
            let is_stale = now.duration_since(info.last_updated) > Duration::from_millis(2000);
            for file in &info.files {
                out.push(ItemList {
                    name: file.name.clone(),
                    ty: file.ty,
                    from: key.clone(),
                    stale: is_stale
                })
            }
        }
        out
    }
    pub fn set(&self, id: String, items: Vec<Item>) {

        let mut state = self.state.lock().unwrap();

        let now = Instant::now();
        state.insert(id, Info {
            last_updated: now,
            files: items
        });

    }
    pub fn update(&self, id: String, diff: Diff<Item>) {

        let mut state = self.state.lock().unwrap();

        // start with any items we find, filtered by those removed
        // or those about to be added.ß
        let mut new_files: Vec<Item> = {

            let removed: HashSet<&Item> = diff.removed.iter().collect();
            let added: HashSet<&Item> = diff.added.iter().collect();

            state
                .remove(&id)
                .map(|info| info.files)
                .unwrap_or(vec![])
                .into_iter()
                .filter(|item| !removed.contains(item) && !added.contains(item))
                .collect()
        };

        // add the remaining items:
        for item in diff.added {
            new_files.push(item);
        }

        state.insert(id, Info {
            last_updated: Instant::now(),
            files: new_files
        });

    }
    pub fn remove_older_than(&self, duration: Duration) {

        let mut state = self.state.lock().unwrap();

        let now = Instant::now();
        state.retain(|_, info| {
            now.duration_since(info.last_updated) < duration
        })

    }
}

#[derive(Debug,Serialize,Deserialize)]
pub struct ItemList {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub ty: Type,
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "Stale")]
    pub stale: bool
}
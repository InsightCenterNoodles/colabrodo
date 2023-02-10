use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub enum Asset {
    InMemory(Vec<u8>),
    OnDisk(PathBuf),
}

#[derive(Default)]
pub struct AssetStore {
    asset_list: HashMap<uuid::Uuid, Arc<Asset>>,
}

impl AssetStore {
    pub fn add_asset(&mut self, asset: Asset) -> uuid::Uuid {
        let new_ident = uuid::Uuid::new_v4();

        self.asset_list.insert(new_ident, Arc::new(asset));

        new_ident
    }

    pub fn delete_asset(&mut self, id: &uuid::Uuid) {
        self.asset_list.remove(id);
    }

    pub fn fetch_asset(&self, id: &uuid::Uuid) -> Option<Arc<Asset>> {
        self.asset_list.get(id).cloned()
    }
}

// async fn send_file(path: &std::path::Path) {

// }

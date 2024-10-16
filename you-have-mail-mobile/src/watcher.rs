use sqlite_watcher::watcher::DropRemoveTableObserverHandle;

#[derive(uniffi::Object)]
pub struct WatchHandle {
    _h: DropRemoveTableObserverHandle,
}

impl From<DropRemoveTableObserverHandle> for WatchHandle {
    fn from(value: DropRemoveTableObserverHandle) -> Self {
        Self { _h: value }
    }
}

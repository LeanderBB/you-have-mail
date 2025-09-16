use you_have_mail_common::exports::rusqlite::params;
use you_have_mail_common::state::Error as DBError;

/// Notification ids associated with a given account.
#[derive(Debug, Eq, PartialEq, Copy, Clone, uniffi::Record)]
pub struct AccountNotificationIds {
    /// For notification grouping.
    pub group: i32,
    /// For status updates, such as logout.
    pub status: i32,
    /// For error reporting.
    pub error: i32,
}

/// Extension trait to store additional state for the android application.
pub trait StateExtension {
    /// Create the android specific tables.
    ///
    /// # Errors
    ///
    /// Returns error if the queries failed.
    fn android_init_tables(&self) -> Result<(), DBError>;

    /// Get the next unique notification id for the given user.
    ///
    /// # Errors
    ///
    /// Returns error if the queries failed.
    fn android_next_mail_notification_id(&self, email: &str) -> Result<i32, DBError>;

    /// Get or create the 3 stable notification ids for a user with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    fn android_get_or_create_notification_ids(
        &self,
        email: &str,
    ) -> Result<AccountNotificationIds, DBError>;
}

impl StateExtension for you_have_mail_common::state::State {
    fn android_init_tables(&self) -> Result<(), DBError> {
        self.db_write(|tx| {
            tx.execute(
                r"
CREATE TABLE IF NOT EXISTS android_notification_ids(
    email TEXT PRIMARY KEY,
    group_id INTEGER NOT NULL,
    status_id INTEGER NOT NULL,
    error_id INTEGER NOT NULL
)
           ",
                params![],
            )?;

            tx.execute(
                r"
CREATE TABLE IF NOT EXISTS android_next_mail_notification_id(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL UNIQUE
)
           ",
                params![],
            )?;
            Ok(())
        })
    }

    fn android_next_mail_notification_id(&self, email: &str) -> Result<i32, DBError> {
        self.db_write(|tx| {
            Ok(tx.query_row(
                "INSERT OR REPLACE INTO android_next_mail_notification_id (email) VALUES (?) RETURNING id",
                params![email],
                |r| r.get(0),
            ).map(|v:u64| {
               i32::try_from(v % (i32::MAX as u64)).expect("Should never fail")
            })?)
        })
    }

    fn android_get_or_create_notification_ids(
        &self,
        email: &str,
    ) -> Result<AccountNotificationIds, DBError> {
        self.db_write(|tx| {
            Ok(tx.query_row(
                r"
WITH cte (gid) AS (
    SELECT IFNULL(MAX(group_id)+3,100) FROM android_notification_ids
)
INSERT INTO android_notification_ids (
    email, group_id, status_id, error_id
) VALUES (?,
    (SELECT cte.gid FROM cte),
    (SELECT cte.gid+1 FROM cte),
    (SELECT cte.gid+2 FROM cte)
)
ON CONFLICT (email) DO UPDATE SET email=email
RETURNING group_id, status_id, error_id
           ",
                params![email],
                |r| {
                    Ok(AccountNotificationIds {
                        group: r.get(0)?,
                        status: r.get(1)?,
                        error: r.get(2)?,
                    })
                },
            )?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlite_watcher::watcher::Watcher;
    use std::sync::Arc;
    use tempfile::TempDir;
    use you_have_mail_common::encryption::Key;
    use you_have_mail_common::state::State;

    #[test]
    fn check_get_or_create_notification_ids() {
        let (state, _tmp_dir) = new_state();

        const START_GROUP_ID: i32 = 100;

        // Insert first time should start with START_GROUP_ID
        let ids = state.android_get_or_create_notification_ids("foo").unwrap();

        assert_eq!(ids.group, START_GROUP_ID);
        assert_eq!(ids.status, START_GROUP_ID + 1);
        assert_eq!(ids.error, START_GROUP_ID + 2);

        // Insert the second time, no changes
        let ids2 = state.android_get_or_create_notification_ids("foo").unwrap();
        assert_eq!(ids, ids2);

        // Other account
        let ids3 = state.android_get_or_create_notification_ids("bar").unwrap();
        assert_eq!(ids3.group, START_GROUP_ID + 3);
        assert_eq!(ids3.status, START_GROUP_ID + 4);
        assert_eq!(ids3.error, START_GROUP_ID + 5);
    }

    #[test]
    fn check_next_mail_notification_id() {
        let (state, _tmp_dir) = new_state();

        let id_foo_1 = state.android_next_mail_notification_id("foo").unwrap();
        let id_foo_2 = state.android_next_mail_notification_id("foo").unwrap();
        let id_bar_1 = state.android_next_mail_notification_id("bar").unwrap();
        let id_foo_3 = state.android_next_mail_notification_id("foo").unwrap();
        let id_bar_2 = state.android_next_mail_notification_id("bar").unwrap();
        assert!(id_foo_1 < id_foo_2);
        assert!(id_foo_2 < id_bar_1);
        assert!(id_bar_1 < id_foo_3);
        assert!(id_foo_3 < id_bar_2);
    }

    fn new_state() -> (Arc<State>, TempDir) {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("sqlite.db");
        let watcher = Watcher::new().unwrap();
        let state = State::without_init(db_path, Key::new(), watcher).unwrap();
        state.android_init_tables().unwrap();

        (state, tmp_dir)
    }
}

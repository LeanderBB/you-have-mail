//! Database storage for applications state.
use parking_lot::{Mutex, MutexGuard};
use rusqlite::{Params, Result, Row, Statement};
use sqlite_watcher::connection::Connection as WatchedConnection;
use sqlite_watcher::watcher::Watcher;
use std::path::PathBuf;
use std::sync::Arc;

/// Database pool which maintains a small amount of open connections.
pub struct Pool {
    connections: Mutex<Vec<WatchedConnection<rusqlite::Connection>>>,
    writer_lock: Mutex<()>,
    path: PathBuf,
    watcher: Arc<Watcher>,
}

const MAX_DB_CONNECTIONS: usize = 4;

impl Pool {
    /// Create new instance for a database at `path`.
    #[must_use]
    pub fn new(path: PathBuf, watcher: Arc<Watcher>) -> Arc<Self> {
        Arc::new(Self {
            connections: Mutex::new(Vec::with_capacity(MAX_DB_CONNECTIONS)),
            writer_lock: Mutex::new(()),
            path,
            watcher,
        })
    }

    /// Get database watcher instance.
    pub fn watcher(&self) -> &Arc<Watcher> {
        &self.watcher
    }

    /// Get a new connection from the pool.
    ///
    /// If no existing connections exist, a new one will be created.
    ///
    /// # Errors
    ///
    /// Returns error if we fail to create a new connection.
    pub fn connection(self: &Arc<Self>) -> Result<Connection> {
        let mut guard = self.connections.lock();
        if let Some(conn) = guard.pop() {
            return Ok(Connection {
                pool: Arc::clone(self),
                conn: Some(conn),
            });
        }

        let conn = self.new_connection()?;
        Ok(Connection {
            pool: Arc::clone(self),
            conn: Some(conn),
        })
    }

    /// Retrieve a connection from the pool and run the given `closure` on it.
    ///
    /// Connection is automatically released back to the pool after the closure is finished
    /// executing.
    /// # Errors
    ///
    /// Returns error if we failed to get a connection or if the closure failed to execute.
    #[inline]
    pub fn with_connection<T, E: From<rusqlite::Error>>(
        self: &Arc<Self>,
        closure: impl FnOnce(&mut Connection) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut conn = self.connection()?;
        closure(&mut conn)
    }

    /// Retrieve a connection from the pool, create a transaction and run the given `closure` on it.
    ///
    /// Connection is automatically released back to the pool after the closure is finished
    /// executing.
    ///
    /// The transaction is automatically committed if the closure does not fail to execute.
    /// # Errors
    ///
    /// Returns error if we failed to get a connection or if the closure failed to execute.
    #[inline]
    pub fn with_transaction<T, E: From<rusqlite::Error>>(
        self: &Arc<Self>,
        closure: impl FnOnce(&mut Transaction) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut conn = self.connection()?;
        conn.with_transaction(closure)
    }

    /// Return a `conn` back to the pool.
    fn release(&self, conn: WatchedConnection<rusqlite::Connection>) {
        let mut guard = self.connections.lock();
        if guard.len() < MAX_DB_CONNECTIONS {
            guard.push(conn);
        }
    }

    /// Create a new connection.
    fn new_connection(&self) -> Result<WatchedConnection<rusqlite::Connection>> {
        let conn = rusqlite::Connection::open(&self.path)?;
        conn.pragma_update(None, "journal", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        WatchedConnection::new(conn, Arc::clone(&self.watcher))
    }
}

/// Pooled connection.
///
/// This wraps [`rusqlite::Connection`] and ensures that transactions are exclusive to avoid
/// conflicts in the sqlite database.
pub struct Connection {
    pool: Arc<Pool>,
    conn: Option<WatchedConnection<rusqlite::Connection>>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.release(conn);
        }
    }
}
impl Connection {
    /// See [`rusqlite::Connection::query_row`].
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    #[inline]
    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> Result<T>,
    {
        self.conn().query_row(sql, params, f)
    }

    /// See [`rusqlite::Connection::prepare`].
    ///
    /// # Errors
    ///
    /// Returns error if the statement could not be constructed.
    #[inline]
    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>> {
        self.conn().prepare(sql)
    }

    /// Creates a new Transaction.
    ///
    /// # Errors
    ///
    /// Returns error if we failed to create the transaction.
    #[inline]
    #[allow(clippy::missing_panics_doc)]
    fn transaction(&mut self) -> Result<Transaction<'_, '_>> {
        let guard = self.pool.writer_lock.lock();
        let tx = self.conn.as_mut().unwrap().transaction()?;
        Ok(Transaction { tx, _guard: guard })
    }

    /// Create a new transaction and execute `closure` on it. The transaction is commited after
    /// successful execution of the closure.
    ///
    /// # Errors
    ///
    /// Returns errors if we fail to execute the closure, create the transaction or commit
    /// the transaction.
    #[inline]
    pub fn with_transaction<T, E: From<rusqlite::Error>>(
        &mut self,
        closure: impl FnOnce(&mut Transaction) -> Result<T, E>,
    ) -> Result<T, E> {
        self.conn_mut().sync_watcher_tables()?;
        let mut tx = self.transaction()?;
        let result = closure(&mut tx)?;
        tx.commit()?;
        self.conn_mut().publish_watcher_changes()?;
        Ok(result)
    }
    #[inline]
    fn conn(&self) -> &WatchedConnection<rusqlite::Connection> {
        // This is always valid while the type is alive.
        self.conn.as_ref().unwrap()
    }

    #[inline]
    fn conn_mut(&mut self) -> &mut WatchedConnection<rusqlite::Connection> {
        // This is always valid while the type is alive.
        self.conn.as_mut().unwrap()
    }
}

/// Transaction wrapper.
///
/// Only one transaction can run per pool. This enforces the single write limitation that is
/// present in sqlite.
pub struct Transaction<'c, 'l> {
    tx: rusqlite::Transaction<'c>,
    _guard: MutexGuard<'l, ()>,
}

impl Transaction<'_, '_> {
    /// See [`rusqlite::Connection::query_row`].
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    #[inline]
    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> Result<T>,
    {
        self.tx.query_row(sql, params, f)
    }

    /// See [`rusqlite::Connection::prepare`].
    ///
    /// # Errors
    ///
    /// Returns error if the statement could not be constructed.
    #[inline]
    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>> {
        self.tx.prepare(sql)
    }

    /// See [`rusqlite::Connection::execute`].
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    #[inline]
    pub fn execute<P: Params>(&self, sql: &str, params: P) -> Result<usize> {
        self.tx.execute(sql, params)
    }

    /// See [`rusqlite::Transaction::commit`].
    ///
    /// # Errors
    ///
    /// Returns error if the transaction failed to commit
    pub fn commit(self) -> Result<()> {
        self.tx.commit()
    }
}

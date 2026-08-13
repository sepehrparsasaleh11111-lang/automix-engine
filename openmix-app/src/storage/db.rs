pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(include_str!("schema.sql"))
}

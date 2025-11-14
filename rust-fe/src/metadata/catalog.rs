use dashmap::DashMap;
use lazy_static::lazy_static;
use std::sync::Arc;
use tracing::info;

use super::schema::{Database, Table, ColumnDef};

lazy_static! {
    static ref GLOBAL_CATALOG: CatalogManager = {
        let catalog = CatalogManager::new();

        // Initialize with TPC-H schema
        info!("Initializing catalog with TPC-H schema");
        catalog.add_database(Database::create_tpch_schema());

        // Add default databases
        catalog.add_database(Database::new("information_schema".to_string()));
        catalog.add_database(Database::new("mysql".to_string()));

        info!("Catalog initialized with {} databases", catalog.list_databases().len());

        catalog
    };
}

/// Global catalog access
pub fn catalog() -> &'static CatalogManager {
    &GLOBAL_CATALOG
}

pub struct CatalogManager {
    databases: DashMap<String, Arc<Database>>,
}

impl CatalogManager {
    pub fn new() -> Self {
        Self {
            databases: DashMap::new(),
        }
    }

    pub fn add_database(&self, mut db: Database) {
        let name = db.name.clone();
        info!("Adding database '{}' to catalog", name);
        self.databases.insert(name, Arc::new(db));
    }

    pub fn get_database(&self, name: &str) -> Option<Arc<Database>> {
        self.databases.get(name).map(|entry| entry.value().clone())
    }

    pub fn get_database_mut(&self, name: &str) -> Option<dashmap::mapref::one::RefMut<String, Arc<Database>>> {
        self.databases.get_mut(name)
    }

    pub fn drop_database(&self, name: &str) -> Option<Arc<Database>> {
        self.databases.remove(name).map(|(_, db)| db)
    }

    pub fn list_databases(&self) -> Vec<String> {
        self.databases.iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn database_exists(&self, name: &str) -> bool {
        self.databases.contains_key(name)
    }

    pub fn get_table(&self, db_name: &str, table_name: &str) -> Option<Table> {
        self.get_database(db_name)?
            .get_table(table_name)
            .cloned()
    }

    pub fn add_table(&self, db_name: &str, table: Table) -> Result<(), String> {
        // Since Database contains tables in a HashMap and we need to modify it,
        // and Arc<Database> is immutable, we need to handle this differently
        // For now, we'll need to replace the entire database

        let db = self.get_database(db_name)
            .ok_or_else(|| format!("Database '{}' not found", db_name))?;

        let mut new_db = (*db).clone();
        new_db.add_table(table);

        self.databases.insert(db_name.to_string(), Arc::new(new_db));

        Ok(())
    }

    pub fn drop_table(&self, db_name: &str, table_name: &str) -> Result<(), String> {
        let db = self.get_database(db_name)
            .ok_or_else(|| format!("Database '{}' not found", db_name))?;

        let mut new_db = (*db).clone();
        new_db.drop_table(table_name)
            .ok_or_else(|| format!("Table '{}' not found", table_name))?;

        self.databases.insert(db_name.to_string(), Arc::new(new_db));

        Ok(())
    }

    pub fn list_tables(&self, db_name: &str) -> Result<Vec<String>, String> {
        let db = self.get_database(db_name)
            .ok_or_else(|| format!("Database '{}' not found", db_name))?;

        Ok(db.list_tables())
    }

    pub fn table_exists(&self, db_name: &str, table_name: &str) -> bool {
        self.get_table(db_name, table_name).is_some()
    }

    pub fn get_table_columns(&self, db_name: &str, table_name: &str) -> Option<Vec<ColumnDef>> {
        let table = self.get_table(db_name, table_name)?;
        Some(table.columns.clone())
    }
}

impl Default for CatalogManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::types::DataType;

    #[test]
    fn test_catalog_basic() {
        let catalog = CatalogManager::new();

        // Create a test database
        let mut db = Database::new("test_db".to_string());
        let table = Table::new("test_table".to_string())
            .add_column(ColumnDef::new("id".to_string(), DataType::Int).primary_key())
            .add_column(ColumnDef::new("name".to_string(), DataType::Varchar { length: 100 }));

        db.add_table(table);
        catalog.add_database(db);

        // Test database operations
        assert!(catalog.database_exists("test_db"));
        assert_eq!(catalog.list_databases().len(), 1);

        // Test table operations
        assert!(catalog.table_exists("test_db", "test_table"));
        let tables = catalog.list_tables("test_db").unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0], "test_table");

        // Test getting table columns
        let columns = catalog.get_table_columns("test_db", "test_table").unwrap();
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[1].name, "name");
    }

    #[test]
    fn test_tpch_schema() {
        let catalog = catalog(); // Get global catalog

        // Check that TPCH database exists
        assert!(catalog.database_exists("tpch"));

        // Check that all TPC-H tables exist
        let expected_tables = vec![
            "nation", "region", "part", "supplier",
            "partsupp", "customer", "orders", "lineitem"
        ];

        for table_name in &expected_tables {
            assert!(catalog.table_exists("tpch", table_name),
                   "Table '{}' should exist in TPC-H schema", table_name);
        }

        // Check lineitem table structure
        let lineitem = catalog.get_table("tpch", "lineitem").unwrap();
        assert_eq!(lineitem.columns.len(), 16);

        // Check some specific columns
        assert!(lineitem.get_column("l_orderkey").is_some());
        assert!(lineitem.get_column("l_partkey").is_some());
        assert!(lineitem.get_column("l_quantity").is_some());
    }
}

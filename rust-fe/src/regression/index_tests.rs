//! Index Tests - B-Tree, Inverted Index, Bloom Filter
//! Inspired by Doris regression-test/suites/index_p0/, inverted_index_p0/, bloom_filter_p0/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // Basic Index Tests (15 tests)

    #[test]
    fn test_create_index_basic() {
        let sql = "CREATE INDEX idx_name ON users(name)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_index_multi_column() {
        let sql = "CREATE INDEX idx_name_email ON users(name, email)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_unique_index() {
        let sql = "CREATE UNIQUE INDEX idx_email ON users(email)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_index_if_not_exists() {
        let sql = "CREATE INDEX IF NOT EXISTS idx_name ON users(name)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_index_with_using() {
        let sql = "CREATE INDEX idx_name ON users(name) USING BTREE";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_drop_index() {
        let sql = "DROP INDEX idx_name ON users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_drop_index_if_exists() {
        let sql = "DROP INDEX IF EXISTS idx_name ON users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_indexes() {
        let sql = "SHOW INDEXES FROM users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_query_with_index() {
        let sql = "SELECT * FROM users WHERE name = 'Alice'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_query_with_index_range() {
        let sql = "SELECT * FROM users WHERE age BETWEEN 20 AND 30";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_query_with_index_in() {
        let sql = "SELECT * FROM users WHERE id IN (1, 2, 3, 4, 5)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_query_with_index_like() {
        let sql = "SELECT * FROM users WHERE name LIKE 'A%'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_index_desc() {
        let sql = "CREATE INDEX idx_age ON users(age DESC)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_add_index() {
        let sql = "ALTER TABLE users ADD INDEX idx_phone(phone)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_drop_index() {
        let sql = "ALTER TABLE users DROP INDEX idx_phone";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Inverted Index Tests (20 tests)

    #[test]
    fn test_create_inverted_index() {
        let sql = r#"
            CREATE INDEX idx_inverted ON documents(content)
            USING INVERTED
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_inverted_index_with_properties() {
        let sql = r#"
            CREATE INDEX idx_inverted ON documents(content)
            USING INVERTED
            PROPERTIES("parser" = "english")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_match() {
        let sql = r#"
            SELECT * FROM documents
            WHERE content MATCH 'search query'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_match_all() {
        let sql = r#"
            SELECT * FROM documents
            WHERE content MATCH_ALL 'word1 word2'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_match_any() {
        let sql = r#"
            SELECT * FROM documents
            WHERE content MATCH_ANY 'word1 word2'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_match_phrase() {
        let sql = r#"
            SELECT * FROM documents
            WHERE content MATCH_PHRASE 'exact phrase match'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_with_tokenizer() {
        let sql = r#"
            CREATE INDEX idx_chinese ON documents(content)
            USING INVERTED
            PROPERTIES("parser" = "chinese")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_multi_column() {
        let sql = r#"
            CREATE INDEX idx_multi ON documents(title, content)
            USING INVERTED
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_with_stopwords() {
        let sql = r#"
            CREATE INDEX idx_stop ON documents(content)
            USING INVERTED
            PROPERTIES("parser" = "standard", "stopword_file" = "stopwords.txt")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_case_insensitive() {
        let sql = r#"
            CREATE INDEX idx_case ON documents(content)
            USING INVERTED
            PROPERTIES("parser" = "standard", "lowercase" = "true")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_with_score() {
        let sql = r#"
            SELECT *, MATCH_SCORE(content, 'search query') as score
            FROM documents
            WHERE content MATCH 'search query'
            ORDER BY score DESC
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_boolean_query() {
        let sql = r#"
            SELECT * FROM documents
            WHERE content MATCH '+required -excluded optional'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_wildcard() {
        let sql = r#"
            SELECT * FROM documents
            WHERE content MATCH 'test*'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_fuzzy() {
        let sql = r#"
            SELECT * FROM documents
            WHERE content MATCH 'word~2'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_proximity() {
        let sql = r#"
            SELECT * FROM documents
            WHERE content MATCH_PHRASE 'word1 word2'~5
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_boost() {
        let sql = r#"
            SELECT * FROM documents
            WHERE title MATCH 'important^2 OR content MATCH 'normal'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_range_query() {
        let sql = r#"
            SELECT * FROM documents
            WHERE numeric_field >= 10 AND numeric_field <= 100
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_inverted_index_exists() {
        let sql = r#"
            SELECT * FROM documents
            WHERE field_name EXISTS
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_drop_inverted_index() {
        let sql = "DROP INDEX idx_inverted ON documents";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_inverted_indexes() {
        let sql = "SHOW INDEXES FROM documents WHERE Index_type = 'INVERTED'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Bloom Filter Tests (15 tests)

    #[test]
    fn test_create_table_with_bloom_filter() {
        let sql = r#"
            CREATE TABLE bloom_table (
                id INT,
                email VARCHAR(100)
            )
            PROPERTIES ("bloom_filter_columns" = "email")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_multi_column() {
        let sql = r#"
            CREATE TABLE bloom_multi (
                id INT,
                email VARCHAR(100),
                phone VARCHAR(20)
            )
            PROPERTIES ("bloom_filter_columns" = "email,phone")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_add_bloom_filter() {
        let sql = r#"
            ALTER TABLE users
            SET ("bloom_filter_columns" = "email")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_with_fpp() {
        let sql = r#"
            CREATE TABLE bloom_fpp (
                id INT,
                value VARCHAR(100)
            )
            PROPERTIES (
                "bloom_filter_columns" = "value",
                "bloom_filter_fpp" = "0.01"
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_query_with_bloom_filter_eq() {
        let sql = "SELECT * FROM bloom_table WHERE email = 'test@example.com'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_query_with_bloom_filter_in() {
        let sql = r#"
            SELECT * FROM bloom_table
            WHERE email IN ('a@test.com', 'b@test.com', 'c@test.com')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_with_join() {
        let sql = r#"
            SELECT b.*
            FROM bloom_table b
            INNER JOIN users u ON b.email = u.email
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_with_subquery() {
        let sql = r#"
            SELECT * FROM bloom_table
            WHERE email IN (SELECT email FROM verified_users)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_statistics() {
        let sql = r#"
            SELECT COUNT(*) FROM bloom_table
            WHERE email = 'rare@example.com'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_with_not_equals() {
        let sql = "SELECT * FROM bloom_table WHERE email != 'excluded@test.com'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_with_exists() {
        let sql = r#"
            SELECT * FROM users u
            WHERE EXISTS (
                SELECT 1 FROM bloom_table b WHERE b.email = u.email
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_with_not_in() {
        let sql = r#"
            SELECT * FROM users
            WHERE email NOT IN (SELECT email FROM bloom_table)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bloom_filter_performance_hint() {
        let sql = "SELECT /*+ USE_BLOOM_FILTER(bloom_table) */ * FROM bloom_table WHERE email = 'test@example.com'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_bloom_filter_info() {
        let sql = r#"
            SHOW CREATE TABLE bloom_table
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_remove_bloom_filter() {
        let sql = r#"
            ALTER TABLE bloom_table
            SET ("bloom_filter_columns" = "")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}

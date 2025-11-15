-- TPC-DS Query 26
-- TODO: Implement full query from TPC-DS specification v3.2.0
-- Template placeholder for automated testing

SELECT
    'Q26' as query_id,
    COUNT(*) as placeholder_count
FROM store_sales
WHERE ss_sold_date_sk IS NOT NULL
LIMIT 100;

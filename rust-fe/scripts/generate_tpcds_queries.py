#!/usr/bin/env python3
"""
Generate TPC-DS query templates for all 99 queries
Full query definitions from TPC-DS specification v3.2.0
"""

import os
from pathlib import Path

# TPC-DS query templates - representative patterns
# Full specification: http://www.tpc.org/tpc_documents_current_versions/pdf/tpc-ds_v3.2.0.pdf

TPCDS_QUERIES = {
    2: """-- TPC-DS Query 2: Date analysis with multiple tables
WITH wscs AS (
    SELECT sold_date_sk, sales_price
    FROM (
        SELECT ws_sold_date_sk AS sold_date_sk, ws_ext_sales_price AS sales_price
        FROM web_sales
        UNION ALL
        SELECT cs_sold_date_sk AS sold_date_sk, cs_ext_sales_price AS sales_price
        FROM catalog_sales
    ) x
),
wswscs AS (
    SELECT d_week_seq,
        SUM(CASE WHEN (d_day_name='Sunday') THEN sales_price ELSE NULL END) sun_sales,
        SUM(CASE WHEN (d_day_name='Monday') THEN sales_price ELSE NULL END) mon_sales,
        SUM(CASE WHEN (d_day_name='Tuesday') THEN sales_price ELSE NULL END) tue_sales,
        SUM(CASE WHEN (d_day_name='Wednesday') THEN sales_price ELSE NULL END) wed_sales,
        SUM(CASE WHEN (d_day_name='Thursday') THEN sales_price ELSE NULL END) thu_sales,
        SUM(CASE WHEN (d_day_name='Friday') THEN sales_price ELSE NULL END) fri_sales,
        SUM(CASE WHEN (d_day_name='Saturday') THEN sales_price ELSE NULL END) sat_sales
    FROM wscs, date_dim
    WHERE d_date_sk = sold_date_sk
    GROUP BY d_week_seq
)
SELECT d_week_seq1,
    ROUND(sun_sales1/sun_sales2, 2) AS sun_ratio,
    ROUND(mon_sales1/mon_sales2, 2) AS mon_ratio,
    ROUND(tue_sales1/tue_sales2, 2) AS tue_ratio,
    ROUND(wed_sales1/wed_sales2, 2) AS wed_ratio,
    ROUND(thu_sales1/thu_sales2, 2) AS thu_ratio,
    ROUND(fri_sales1/fri_sales2, 2) AS fri_ratio,
    ROUND(sat_sales1/sat_sales2, 2) AS sat_ratio
FROM (
    SELECT wswscs.d_week_seq AS d_week_seq1,
        sun_sales AS sun_sales1,
        mon_sales AS mon_sales1,
        tue_sales AS tue_sales1,
        wed_sales AS wed_sales1,
        thu_sales AS thu_sales1,
        fri_sales AS fri_sales1,
        sat_sales AS sat_sales1
    FROM wswscs, date_dim
    WHERE date_dim.d_week_seq = wswscs.d_week_seq AND d_year = 2001
) y,
(
    SELECT wswscs.d_week_seq AS d_week_seq2,
        sun_sales AS sun_sales2,
        mon_sales AS mon_sales2,
        tue_sales AS tue_sales2,
        wed_sales AS wed_sales2,
        thu_sales AS thu_sales2,
        fri_sales AS fri_sales2,
        sat_sales AS sat_sales2
    FROM wswscs, date_dim
    WHERE date_dim.d_week_seq = wswscs.d_week_seq AND d_year = 2002
) z
WHERE d_week_seq1 = d_week_seq2 - 53
ORDER BY d_week_seq1;
""",

    3: """-- TPC-DS Query 3: Sales analysis by brand
SELECT dt.d_year, item.i_brand_id brand_id, item.i_brand brand,SUM(ss_ext_sales_price) sum_agg
FROM date_dim dt, store_sales, item
WHERE dt.d_date_sk = store_sales.ss_sold_date_sk
    AND store_sales.ss_item_sk = item.i_item_sk
    AND item.i_manufact_id = 436
    AND dt.d_moy = 12
GROUP BY dt.d_year, item.i_brand, item.i_brand_id
ORDER BY dt.d_year, sum_agg DESC, brand_id
LIMIT 100;
""",

    7: """-- TPC-DS Query 7: Promotional sales analysis
SELECT i_item_id,
    AVG(ss_quantity) agg1,
    AVG(ss_list_price) agg2,
    AVG(ss_coupon_amt) agg3,
    AVG(ss_sales_price) agg4
FROM store_sales, customer_demographics, date_dim, item, promotion
WHERE ss_sold_date_sk = d_date_sk
    AND ss_item_sk = i_item_sk
    AND ss_cdemo_sk = cd_demo_sk
    AND ss_promo_sk = p_promo_sk
    AND cd_gender = 'M'
    AND cd_marital_status = 'S'
    AND cd_education_status = 'College'
    AND (p_channel_email = 'N' OR p_channel_event = 'N')
    AND d_year = 2000
GROUP BY i_item_id
ORDER BY i_item_id
LIMIT 100;
""",

    10: """-- TPC-DS Query 10: Customer segment analysis
SELECT cd_gender, cd_marital_status, cd_education_status,
    COUNT(*) cnt1,
    cd_purchase_estimate,
    COUNT(*) cnt2,
    cd_credit_rating,
    COUNT(*) cnt3,
    cd_dep_count,
    COUNT(*) cnt4,
    cd_dep_employed_count,
    COUNT(*) cnt5,
    cd_dep_college_count,
    COUNT(*) cnt6
FROM customer c, customer_address ca, customer_demographics
WHERE c.c_current_addr_sk = ca.ca_address_sk
    AND ca_county IN ('Rush County', 'Toole County', 'Jefferson County', 'Dona Ana County', 'La Porte County')
    AND cd_demo_sk = c.c_current_cdemo_sk
    AND EXISTS (
        SELECT * FROM store_sales, date_dim
        WHERE c.c_customer_sk = ss_customer_sk
            AND ss_sold_date_sk = d_date_sk
            AND d_year = 2002
            AND d_moy BETWEEN 1 AND 4
    )
    AND (NOT EXISTS (
        SELECT * FROM web_sales, date_dim
        WHERE c.c_customer_sk = ws_bill_customer_sk
            AND ws_sold_date_sk = d_date_sk
            AND d_year = 2002
            AND d_moy BETWEEN 1 AND 4
    )
    AND NOT EXISTS (
        SELECT * FROM catalog_sales, date_dim
        WHERE c.c_customer_sk = cs_ship_customer_sk
            AND cs_sold_date_sk = d_date_sk
            AND d_year = 2002
            AND d_moy BETWEEN 1 AND 4
    ))
GROUP BY cd_gender, cd_marital_status, cd_education_status,
    cd_purchase_estimate, cd_credit_rating,
    cd_dep_count, cd_dep_employed_count, cd_dep_college_count
ORDER BY cd_gender, cd_marital_status, cd_education_status,
    cd_purchase_estimate, cd_credit_rating,
    cd_dep_count, cd_dep_employed_count, cd_dep_college_count
LIMIT 100;
""",

    # Add templates for remaining queries
}

# Generate simple template queries for queries not manually defined
def generate_template_query(query_num):
    """Generate a template query for TPC-DS"""
    if query_num in TPCDS_QUERIES:
        return TPCDS_QUERIES[query_num]

    # Generate a simple template
    return f"""-- TPC-DS Query {query_num}
-- TODO: Implement full query from TPC-DS specification v3.2.0
-- Template placeholder for automated testing

SELECT
    'Q{query_num}' as query_id,
    COUNT(*) as placeholder_count
FROM store_sales
WHERE ss_sold_date_sk IS NOT NULL
LIMIT 100;
"""

def main():
    script_dir = Path(__file__).parent
    queries_dir = script_dir / "tpcds" / "queries"
    queries_dir.mkdir(parents=True, exist_ok=True)

    print(f"Generating TPC-DS queries in {queries_dir}")

    # Generate all 99 queries
    for i in range(1, 100):
        query_file = queries_dir / f"q{i}.sql"

        # Skip if already exists and is not a template
        if query_file.exists():
            content = query_file.read_text()
            if "TODO: Implement" not in content and "placeholder" not in content.lower():
                print(f"  Skipping Q{i} (already exists with real query)")
                continue

        query_content = generate_template_query(i)
        query_file.write_text(query_content)
        print(f"  Created Q{i}")

    print(f"\n✓ Generated {len(list(queries_dir.glob('*.sql')))} TPC-DS queries")
    print(f"\nNote: Queries {sorted(TPCDS_QUERIES.keys())} have full implementations.")
    print(f"      Remaining queries have templates for testing.")
    print(f"\nTo implement full TPC-DS queries:")
    print(f"  1. Download TPC-DS specification: http://www.tpc.org/tpcds/")
    print(f"  2. Extract query templates from specification")
    print(f"  3. Replace template queries with full implementations")

if __name__ == "__main__":
    main()

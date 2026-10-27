# Security Vulnerability Audit - PyReverseETL

**Audit Date:** 2026-10-27
**Repository:** /Users/georgimullassery/Desktop/Projects/PyReverseETL
**Total Findings:** 15

## Summary
- **Critical:** 1
- **High:** 14

## Findings

### 1. [Critical] Hardcoded Secret
- **File:** docs/archive/SECURITY_AUDIT_REPORT_2026-07-26_STALE.md:315
- **Description:** Password Assignment detected
- **Evidence (Line):** `// let password = "super_secret";  // Not present`
- **Exploit Scenario:** Attacker could extract credentials and gain unauthorized access
- **Fix Recommendation:** Move secrets to environment variables or secret manager, rotate compromised credentials
- **Confidence:** High

### 2. [High] .env File with Secrets
- **File:** .env.example:1
- **Description:** .env.example contains potential secrets
- **Evidence (Line):** `# Environment variables PyReverseETL actually reads. Source and destination`
- **Exploit Scenario:** Secrets in committed .env files exposed in git history
- **Fix Recommendation:** Add .env to .gitignore, remove from git history, rotate secrets
- **Confidence:** High

### 3. [High] SQL Injection
- **File:** python/pyreverseetl/connectors.py:237
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `connection.execute(f"DROP TABLE {_quote(table)}")`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 4. [High] SQL Injection
- **File:** python/pyreverseetl/connectors.py:248
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `connection.execute(`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 5. [High] SQL Injection
- **File:** python/pyreverseetl/connectors.py:418
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `connection.execute(f"DROP TABLE {_quote(table)}")`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 6. [High] SQL Injection
- **File:** python/pyreverseetl/connectors.py:429
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `connection.execute(`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 7. [High] SQL Injection
- **File:** python/pyreverseetl/connectors.py:625
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `cursor.execute(f"DROP TABLE {_quote(table)}")`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 8. [High] SQL Injection
- **File:** python/pyreverseetl/connectors.py:638
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `cursor.execute(`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 9. [High] SQL Injection
- **File:** python/pyreverseetl/connectors.py:616
- **Description:** SQL % formatting
- **Evidence (Line):** `cursor.execute(`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 10. [High] SQL Injection
- **File:** tests/test_connector_plugins.py:38
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `return [dict(row) for row in connection.execute(f'SELECT * FROM "{table}"')]`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 11. [High] SQL Injection
- **File:** tests/test_duckdb_postgres_connectors.py:57
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `cursor = connection.execute(f'SELECT * FROM "{table}" ORDER BY id')`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 12. [High] SQL Injection
- **File:** tests/test_duckdb_postgres_connectors.py:175
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `cursor.execute(`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 13. [High] SQL Injection
- **File:** tests/test_duckdb_postgres_connectors.py:179
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `cursor.executemany(`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 14. [High] SQL Injection
- **File:** tests/test_duckdb_postgres_connectors.py:207
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `cursor.execute(f'DROP TABLE IF EXISTS "{source_table}"')`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

### 15. [High] SQL Injection
- **File:** tests/test_duckdb_postgres_connectors.py:208
- **Description:** SQL f-string interpolation
- **Evidence (Line):** `cursor.execute(f'DROP TABLE IF EXISTS "{destination_table}"')`
- **Exploit Scenario:** Attacker could manipulate SQL queries to access/modify/delete data
- **Fix Recommendation:** Use parameterized queries/prepared statements
- **Confidence:** Medium

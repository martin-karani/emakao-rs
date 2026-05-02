-- Creates the tenant database alongside the platform database.
-- This runs automatically when the postgres container first starts
-- (mounted into /docker-entrypoint-initdb.d/).
--
-- POSTGRES_DB in docker-compose creates emakao_platform automatically.
-- We only need to create emakao_tenant here.

SELECT 'CREATE DATABASE emakao_tenant OWNER emakao'
WHERE NOT EXISTS (
    SELECT FROM pg_database WHERE datname = 'emakao_tenant'
)\gexec
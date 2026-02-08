-- PostgreSQL init script for Upsert integration tests

CREATE TABLE IF NOT EXISTS customers (
    id SERIAL PRIMARY KEY,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    phone VARCHAR(20),
    balance NUMERIC(18,2) DEFAULT 0.00,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    notes TEXT,
    profile_image BYTEA,
    customer_uuid UUID DEFAULT gen_random_uuid()
);

CREATE TABLE IF NOT EXISTS orders (
    id BIGSERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    order_date DATE NOT NULL DEFAULT CURRENT_DATE,
    total_amount NUMERIC(19,4) NOT NULL,
    tax_amount NUMERIC(10,4),
    status SMALLINT DEFAULT 0,
    shipping_weight REAL,
    discount_pct DOUBLE PRECISION,
    order_xml XML,
    order_data JSONB,
    CONSTRAINT fk_orders_customer FOREIGN KEY (customer_id) REFERENCES customers(id)
);

CREATE TABLE IF NOT EXISTS type_showcase (
    col_boolean BOOLEAN,
    col_smallint SMALLINT,
    col_integer INTEGER,
    col_bigint BIGINT,
    col_serial SERIAL,
    col_bigserial BIGSERIAL,
    col_real REAL,
    col_double DOUBLE PRECISION,
    col_numeric NUMERIC(18,4),
    col_money MONEY,
    col_char CHAR(10),
    col_varchar VARCHAR(255),
    col_text TEXT,
    col_bytea BYTEA,
    col_date DATE,
    col_time TIME,
    col_timestamp TIMESTAMP,
    col_timestamptz TIMESTAMPTZ,
    col_interval INTERVAL,
    col_uuid UUID,
    col_json JSON,
    col_jsonb JSONB,
    col_xml XML,
    col_inet INET,
    col_cidr CIDR,
    col_macaddr MACADDR,
    col_int_array INTEGER[],
    col_text_array TEXT[]
);

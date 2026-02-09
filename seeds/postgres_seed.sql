-- =============================================================
-- Upsert Test Database - PostgreSQL Seed Script (Complex)
-- Database: upsert_test_target
-- Designed to stress-test schema comparison, data diff, and migration
--
-- Run: psql -U postgres -f postgres_seed.sql
-- =============================================================

-- Drop tables if they exist (reverse FK order)
DROP TABLE IF EXISTS order_items CASCADE;
DROP TABLE IF EXISTS reviews CASCADE;
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS customers CASCADE;
DROP TABLE IF EXISTS shipping_zones CASCADE;
DROP TABLE IF EXISTS shipping_rates CASCADE;
DROP TABLE IF EXISTS product_images CASCADE;
DROP TABLE IF EXISTS wishlists CASCADE;
DROP TYPE IF EXISTS order_status CASCADE;

-- =============================================================
-- Custom ENUM type (SQL Server uses NVARCHAR for status)
-- DIFF: PG uses enum, SQL Server uses NVARCHAR(20)
-- =============================================================
CREATE TYPE order_status AS ENUM ('pending', 'processing', 'shipped', 'delivered', 'cancelled', 'refunded', 'on_hold');

-- =============================================================
-- Table: categories
-- DIFF: PG has 'slug' column (missing in SQL Server)
--       SQL Server has 'sort_order' column (missing here)
--       'description' is TEXT here vs NVARCHAR(500) in SQL Server
--       No 'parent_id' self-ref here (flat structure vs hierarchical)
-- =============================================================
CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(100) NOT NULL UNIQUE,               -- ONLY in PostgreSQL
    description TEXT,                                  -- DIFF: TEXT here, NVARCHAR(500) in SQL Server
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),     -- DIFF: TIMESTAMPTZ here, DATETIME2 in SQL Server
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- DIFF: No ix_categories_parent index (no parent_id column)

INSERT INTO categories (name, slug, description, is_active) VALUES
('Electronics', 'electronics', 'Phones, laptops, tablets and accessories', TRUE),
('Books', 'books', 'Physical and digital books', TRUE),
('Clothing', 'clothing', 'Apparel and fashion accessories', TRUE),
('Home & Garden', 'home-garden', 'Furniture, decor and garden supplies', TRUE),
('Sports', 'sports', 'Sporting goods and fitness equipment', TRUE),
-- DIFF: is_active=TRUE here, 0/false in SQL Server
('Toys', 'toys', 'Games, puzzles and children toys', TRUE),
-- ONLY in PostgreSQL (no sub-categories, but extra top-level)
('Automotive', 'automotive', 'Car parts, accessories, and tools', TRUE),
('Health', 'health', 'Vitamins, supplements, and wellness', FALSE);

-- =============================================================
-- Table: customers
-- DIFF: No 'middle_name' (SQL Server has it)
--       Has 'date_of_birth' DATE column (SQL Server doesn't)
--       'loyalty_points' is BIGINT here, INT in SQL Server
--       'phone' is VARCHAR(30) here, NVARCHAR(20) in SQL Server
--       'email' max length 200 here, 255 in SQL Server
--       Column named 'street_address' here, 'address_line1' in SQL Server
--       No 'address_line2' column
--       Column named 'region' here, 'state' in SQL Server
--       Column named 'postal_code' here, 'zip_code' in SQL Server
--       No 'credit_limit' MONEY column
--       Timestamp precision TIMESTAMPTZ(6) here, DATETIME2(3) in SQL Server
-- =============================================================
CREATE TABLE customers (
    id SERIAL PRIMARY KEY,
    email VARCHAR(200) NOT NULL UNIQUE,                -- DIFF: 200 here, 255 in SQL Server
    first_name VARCHAR(100) NOT NULL,
    -- No middle_name column
    last_name VARCHAR(100) NOT NULL,
    phone VARCHAR(30),                                 -- DIFF: VARCHAR(30) here, NVARCHAR(20) in SQL Server
    date_of_birth DATE,                                -- ONLY in PostgreSQL
    street_address VARCHAR(500),                       -- DIFF: named 'street_address' here, 'address_line1' in SQL Server
    -- No address_line2 column
    city VARCHAR(100),
    region VARCHAR(50),                                -- DIFF: named 'region' here, 'state' in SQL Server
    postal_code VARCHAR(15),                           -- DIFF: named 'postal_code' (VARCHAR 15) here, 'zip_code' (NVARCHAR 10) in SQL Server
    country VARCHAR(2) NOT NULL DEFAULT 'US',
    loyalty_points BIGINT NOT NULL DEFAULT 0,          -- DIFF: BIGINT here, INT in SQL Server
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    -- No credit_limit MONEY column
    notes TEXT,                                        -- DIFF: TEXT here, NVARCHAR(MAX) in SQL Server
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),     -- DIFF: TIMESTAMPTZ here, DATETIME2(3) in SQL Server
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ix_customers_region ON customers(region);  -- DIFF: PG indexes 'region', SQL Server indexes 'state'

INSERT INTO customers (email, first_name, last_name, phone, date_of_birth, street_address, city, region, postal_code, country, loyalty_points, notes) VALUES
('alice@example.com', 'Alice', 'Johnson', '555-0101', '1990-05-15', '123 Main St Apt 4B', 'Springfield', 'IL', '62701', 'US', 1500, NULL),
('bob@example.com', 'Bob', 'Smith', '555-0102', '1985-11-22', '456 Oak Ave', 'Portland', 'OR', '97201', 'US', 820, 'Preferred customer'),
-- DIFF: Carol has loyalty_points=2400 here, 2100 in SQL Server; different phone format
('carol@example.com', 'Carol', 'Williams', '555-0103', '1992-08-03', '789 Pine Rd', 'Austin', 'TX', '73301', 'US', 2400, NULL),
('dave@example.com', 'Dave', 'Brown', '555-0104', NULL, '321 Elm St', 'Denver', 'CO', '80201', 'US', 450, NULL),
-- DIFF: Eve has phone='555-9999' here, '555-0105' in SQL Server
('eve@example.com', 'Eve', 'Davis', '555-9999', '1988-01-30', '654 Maple Dr Suite 200', 'Seattle', 'WA', '98101', 'US', 3200, 'VIP customer'),
('frank@example.com', 'Frank', 'Miller', '555-0106', '1995-07-14', '987 Cedar Ln', 'Miami', 'FL', '33101', 'US', 150, NULL),
('grace@example.com', 'Grace', 'Wilson', '555-0107', '1991-12-25', '147 Birch Ct', 'Boston', 'MA', '02101', 'US', 920, NULL),
-- DIFF: Hank has different address ('999 River Rd' here vs '258 Walnut Pl')
('hank@example.com', 'Hank', 'Moore', '555-0108', '1983-04-10', '999 River Rd', 'Chicago', 'IL', '60601', 'US', 1750, NULL),
-- NOT in SQL Server (Ivan/Julia are there instead)
('karen@example.com', 'Karen', 'Thomas', '555-0111', '1997-09-18', '555 Lake Ave', 'San Diego', 'CA', '92101', 'US', 1100, NULL),
('leo@example.com', 'Leo', 'Jackson', '555-0112', '1989-03-07', '666 Hill St', 'Atlanta', 'GA', '30301', 'US', 275, NULL),
-- Unicode customers (same emails as SQL Server but PG uses UTF-8 natively)
('münchen@example.de', 'Müller', 'Strauß', '+49-89-12345', '1980-06-15', 'Königstraße 42', 'München', 'BY', '80331', 'DE', 100, 'German customer — special chars: äöüß €'),
('tokyo@example.jp', '太郎', '山田', '+81-3-1234-5678', '1975-02-28', '東京都渋谷区神宮前1-2-3', '東京', NULL, '150-0001', 'JP', 200, 'Japanese customer: 日本語テスト');

-- =============================================================
-- Table: products
-- DIFF: 'sku' is CHAR(12) fixed-width here, NVARCHAR(50) in SQL Server
--       'description' is VARCHAR(2000) here, NVARCHAR(MAX) in SQL Server
--       'price' is NUMERIC(12,4) here, DECIMAL(10,2) in SQL Server
--       'weight_kg' is REAL here, DECIMAL(6,2) in SQL Server
--       Has 'color' and 'size' columns (SQL Server doesn't)
--       No 'reorder_point' column (SQL Server has it)
--       Different CHECK constraint names
-- =============================================================
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    sku CHAR(12) NOT NULL UNIQUE,                      -- DIFF: CHAR(12) fixed-width here, NVARCHAR(50) in SQL Server
    name VARCHAR(200) NOT NULL,
    description VARCHAR(2000),                         -- DIFF: VARCHAR(2000) here, NVARCHAR(MAX) in SQL Server
    category_id INTEGER NOT NULL REFERENCES categories(id),
    price NUMERIC(12, 4) NOT NULL,                     -- DIFF: NUMERIC(12,4) here, DECIMAL(10,2) in SQL Server
    cost NUMERIC(10, 2),
    stock_quantity INTEGER NOT NULL DEFAULT 0,
    -- No reorder_point column
    weight_kg REAL,                                    -- DIFF: REAL here, DECIMAL(6,2) in SQL Server
    color VARCHAR(50),                                 -- ONLY in PostgreSQL
    size VARCHAR(20),                                  -- ONLY in PostgreSQL
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT products_price_check CHECK (price > 0),   -- DIFF: different constraint name
    CONSTRAINT products_stock_check CHECK (stock_quantity >= 0)
);

CREATE INDEX ix_products_category ON products(category_id);
-- DIFF: No ix_products_price index here (SQL Server has it)
CREATE INDEX ix_products_sku ON products(sku);          -- DIFF: PG has this, SQL Server doesn't

INSERT INTO products (sku, name, description, category_id, price, cost, stock_quantity, weight_kg, color, size) VALUES
('ELEC-001   ', 'Smartphone Pro X', '6.7 inch OLED display, 128GB storage, 5G capable', 1, 999.9900, 650.00, 45, 0.19, 'Midnight Black', NULL),
('ELEC-002   ', 'Laptop Ultra 15', '15.6 inch 4K display, 16GB RAM, 512GB NVMe SSD', 1, 1299.9900, 850.00, 22, 1.80, 'Silver', '15.6"'),
-- DIFF: Earbuds price=139.9900 here, 149.99 in SQL Server (price drop)
('ELEC-003   ', 'Wireless Earbuds', 'Active noise cancelling, 24hr battery life with case', 1, 139.9900, 55.00, 150, 0.05, 'White', NULL),
('BOOK-001   ', 'The Rust Programming Language', 'Official Rust book, 2nd edition', 2, 39.9900, 12.00, 200, 0.45, NULL, NULL),
('BOOK-002   ', 'Database Internals', 'Deep dive into storage engines, distributed systems', 2, 54.9900, 18.00, 85, 0.60, NULL, NULL),
('CLTH-001   ', 'Waterproof Jacket', 'All-weather outdoor jacket. Gore-Tex membrane', 3, 89.9900, 35.00, 60, 0.70, 'Navy Blue', 'L'),
-- DIFF: Running Shoes stock=28 here, 35 in SQL Server
('CLTH-002   ', 'Running Shoes', 'Lightweight performance shoes. Carbon fiber plate', 3, 129.9900, 48.00, 28, 0.65, 'Neon Green', '10'),
('HOME-001   ', 'Standing Desk', 'Electric adjustable height desk. Memory presets', 4, 449.9900, 200.00, 15, 25.00, 'Walnut', '60x30"'),
('HOME-002   ', 'LED Desk Lamp', 'Dimmable 5-mode lighting with USB-C charging port', 4, 34.9900, 10.00, 120, 0.80, 'Matte Black', NULL),
('SPRT-001   ', 'Yoga Mat Premium', 'Non-slip TPE material, extra thick 6mm', 5, 29.9900, 8.00, 200, 1.20, 'Purple', '68x24"'),
('SPRT-002   ', 'Dumbbell Set 20kg', 'Adjustable weight set. Quick-lock mechanism', 5, 79.9900, 30.00, 40, 20.00, 'Black/Chrome', NULL),
-- NOT in SQL Server (SQL Server has ELEC-004, ELEC-005 instead)
('AUTO-001   ', 'Dash Camera 4K', '4K front + 1080p rear dash camera with GPS', 7, 149.9900, 60.00, 80, 0.35, 'Black', NULL),
('HLTH-001   ', 'Vitamin D3 5000IU', '365 softgels, one year supply', 8, 14.9900, 3.00, 500, 0.20, NULL, NULL),
-- NULL stress tests
('MISC-001   ', 'Mystery Box', NULL, 6, 19.9900, NULL, 999, NULL, NULL, NULL);

-- =============================================================
-- Table: product_images (ONLY in PostgreSQL - no equivalent in SQL Server)
-- =============================================================
CREATE TABLE product_images (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,  -- DIFF: CASCADE here, no cascade in SQL Server FKs
    url TEXT NOT NULL,
    alt_text VARCHAR(500),
    sort_order SMALLINT NOT NULL DEFAULT 0,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO product_images (product_id, url, alt_text, sort_order, is_primary) VALUES
(1, 'https://cdn.example.com/products/elec-001-front.jpg', 'Smartphone Pro X front view', 1, TRUE),
(1, 'https://cdn.example.com/products/elec-001-back.jpg', 'Smartphone Pro X rear camera', 2, FALSE),
(2, 'https://cdn.example.com/products/elec-002-open.jpg', 'Laptop Ultra 15 open', 1, TRUE),
(3, 'https://cdn.example.com/products/elec-003-case.jpg', 'Wireless Earbuds with charging case', 1, TRUE),
(8, 'https://cdn.example.com/products/home-001-side.jpg', 'Standing Desk at full height', 1, TRUE),
(8, 'https://cdn.example.com/products/home-001-front.jpg', 'Standing Desk with monitor', 2, FALSE);

-- =============================================================
-- Table: shipping_zones (ONLY in PostgreSQL)
-- =============================================================
CREATE TABLE shipping_zones (
    id SERIAL PRIMARY KEY,
    zone_name VARCHAR(100) NOT NULL,
    min_days INTEGER NOT NULL,
    max_days INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

INSERT INTO shipping_zones (zone_name, min_days, max_days) VALUES
('Domestic Standard', 5, 7),
('Domestic Express', 2, 3),
('International Economy', 10, 21),
('International Priority', 5, 7);

-- =============================================================
-- Table: shipping_rates (ONLY in PostgreSQL - FK to shipping_zones)
-- =============================================================
CREATE TABLE shipping_rates (
    id SERIAL PRIMARY KEY,
    zone_id INTEGER NOT NULL REFERENCES shipping_zones(id),
    min_weight_kg NUMERIC(8, 2) NOT NULL DEFAULT 0,
    max_weight_kg NUMERIC(8, 2) NOT NULL,
    flat_rate NUMERIC(8, 2) NOT NULL,
    per_kg_rate NUMERIC(8, 2) NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT shipping_rates_weight_check CHECK (max_weight_kg > min_weight_kg)
);

INSERT INTO shipping_rates (zone_id, min_weight_kg, max_weight_kg, flat_rate, per_kg_rate) VALUES
(1, 0, 2.00, 5.99, 0),
(1, 2.01, 10.00, 8.99, 1.50),
(1, 10.01, 50.00, 12.99, 2.00),
(2, 0, 2.00, 12.99, 0),
(2, 2.01, 10.00, 18.99, 2.50),
(3, 0, 2.00, 24.99, 0),
(3, 2.01, 10.00, 34.99, 4.00),
(4, 0, 2.00, 49.99, 0),
(4, 2.01, 10.00, 64.99, 5.00);

-- =============================================================
-- Table: orders
-- DIFF: 'status' uses ENUM type here, NVARCHAR(20) in SQL Server
--       'shipping_address' split into components here, single field in SQL Server
--       No 'discount_code' column (SQL Server has it)
--       Has 'currency' column (SQL Server doesn't)
--       'subtotal/total' is DECIMAL(12,2) here, DECIMAL(10,2) in SQL Server
--       No 'shipping_cost' column (SQL Server has it)
--       Uses TIMESTAMPTZ here, DATETIME2 in SQL Server
-- =============================================================
CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    order_number VARCHAR(20) NOT NULL UNIQUE,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    status order_status NOT NULL DEFAULT 'pending',    -- DIFF: ENUM type here, NVARCHAR in SQL Server
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',        -- ONLY in PostgreSQL
    subtotal DECIMAL(12, 2) NOT NULL,                  -- DIFF: DECIMAL(12,2) here, DECIMAL(10,2) in SQL Server
    tax DECIMAL(10, 2) NOT NULL DEFAULT 0,
    -- No shipping_cost column
    total DECIMAL(12, 2) NOT NULL,                     -- DIFF: DECIMAL(12,2) here, DECIMAL(10,2) in SQL Server
    -- No discount_code column
    ship_to_street VARCHAR(500),                       -- DIFF: split address vs single field in SQL Server
    ship_to_city VARCHAR(100),
    ship_to_region VARCHAR(50),
    ship_to_postal VARCHAR(15),
    ship_to_country VARCHAR(2) DEFAULT 'US',
    notes TEXT,                                        -- DIFF: TEXT here, NVARCHAR(1000) in SQL Server
    ordered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    shipped_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ix_orders_customer ON orders(customer_id);
CREATE INDEX ix_orders_status ON orders(status);
-- DIFF: No ix_orders_date index here (SQL Server has it)

INSERT INTO orders (order_number, customer_id, status, currency, subtotal, tax, total, ship_to_street, ship_to_city, ship_to_region, ship_to_postal, ship_to_country, notes, ordered_at, shipped_at, delivered_at) VALUES
('ORD-2024-001', 1, 'delivered', 'USD', 1149.98, 97.75, 1247.73, '123 Main St Apt 4B', 'Springfield', 'IL', '62701', 'US', NULL, '2024-01-15 10:30:00+00', '2024-01-17 08:00:00+00', '2024-01-20 14:30:00+00'),
('ORD-2024-002', 2, 'delivered', 'USD', 39.99, 3.40, 43.39, '456 Oak Ave', 'Portland', 'OR', '97201', 'US', 'Gift wrap please', '2024-01-20 14:15:00+00', '2024-01-21 09:00:00+00', '2024-01-25 11:00:00+00'),
-- DIFF: status='delivered' here, 'shipped' in SQL Server
('ORD-2024-003', 3, 'delivered', 'USD', 219.98, 18.70, 238.68, '789 Pine Rd', 'Austin', 'TX', '73301', 'US', NULL, '2024-02-05 09:00:00+00', '2024-02-07 10:00:00+00', '2024-02-10 12:00:00+00'),
('ORD-2024-004', 1, 'delivered', 'USD', 449.99, 38.25, 488.24, '123 Main St Apt 4B', 'Springfield', 'IL', '62701', 'US', 'Leave at door', '2024-02-10 16:45:00+00', '2024-02-12 08:00:00+00', '2024-02-15 13:00:00+00'),
('ORD-2024-005', 5, 'pending', 'USD', 79.99, 6.80, 86.79, '654 Maple Dr Suite 200', 'Seattle', 'WA', '98101', 'US', NULL, '2024-03-01 11:20:00+00', NULL, NULL),
-- DIFF: status='refunded' here, 'cancelled' in SQL Server; different notes
('ORD-2024-006', 4, 'refunded', 'USD', 129.99, 11.05, 141.04, '321 Elm St', 'Denver', 'CO', '80201', 'US', 'Wrong size - full refund issued', '2024-03-05 13:00:00+00', NULL, NULL),
('ORD-2024-007', 8, 'delivered', 'USD', 184.98, 15.72, 200.70, '999 River Rd', 'Chicago', 'IL', '60601', 'US', NULL, '2024-03-10 08:30:00+00', '2024-03-11 09:00:00+00', '2024-03-14 16:00:00+00'),
('ORD-2024-008', 3, 'shipped', 'USD', 54.99, 4.67, 59.66, '789 Pine Rd', 'Austin', 'TX', '73301', 'US', 'Expedited shipping requested', '2024-03-15 17:00:00+00', '2024-03-16 08:00:00+00', NULL),
-- NOT in SQL Server (SQL Server has ORD-2024-009, 010, 011 instead)
('ORD-2024-012', 9, 'delivered', 'USD', 69.98, 5.95, 75.93, '555 Lake Ave', 'San Diego', 'CA', '92101', 'US', NULL, '2024-03-22 14:30:00+00', '2024-03-23 09:00:00+00', '2024-03-26 11:00:00+00'),
-- International orders (tests cross-engine with non-US addresses)
('ORD-2024-013', 11, 'shipped', 'EUR', 999.99, 190.00, 1189.99, 'Königstraße 42', 'München', 'BY', '80331', 'DE', NULL, '2024-04-01 09:00:00+00', '2024-04-03 08:00:00+00', NULL),
('ORD-2024-014', 12, 'pending', 'JPY', 39.99, 4.00, 43.99, '東京都渋谷区神宮前1-2-3', '東京', NULL, '150-0001', 'JP', NULL, '2024-04-05 02:00:00+00', NULL, NULL),
-- On-hold status (only valid in PG enum, not in SQL Server)
('ORD-2024-015', 5, 'on_hold', 'USD', 449.99, 38.25, 488.24, '654 Maple Dr Suite 200', 'Seattle', 'WA', '98101', 'US', 'Awaiting payment verification', '2024-04-10 15:00:00+00', NULL, NULL);

-- =============================================================
-- Table: order_items
-- DIFF: No 'notes' column (SQL Server has it)
--       Has 'returned_quantity' column (SQL Server doesn't)
--       Named 'discount_amount' (flat $) here, 'discount_pct' (%) in SQL Server
-- =============================================================
CREATE TABLE order_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,  -- DIFF: CASCADE here
    product_id INTEGER NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price DECIMAL(10, 2) NOT NULL,
    discount_amount DECIMAL(10, 2) NOT NULL DEFAULT 0,  -- DIFF: flat amount here, percentage in SQL Server
    line_total DECIMAL(10, 2) NOT NULL,
    returned_quantity INTEGER NOT NULL DEFAULT 0,        -- ONLY in PostgreSQL
    -- No notes column
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO order_items (order_id, product_id, quantity, unit_price, discount_amount, line_total, returned_quantity) VALUES
(1, 1, 1, 999.99, 0, 999.99, 0),
(1, 3, 1, 149.99, 0, 149.99, 0),
(2, 4, 1, 39.99, 4.00, 35.99, 0),
(3, 6, 1, 89.99, 0, 89.99, 0),
(3, 7, 1, 129.99, 0, 129.99, 1),         -- DIFF: 1 returned here
(4, 8, 1, 449.99, 90.00, 359.99, 0),
(5, 11, 1, 79.99, 0, 79.99, 0),
(6, 7, 1, 129.99, 0, 129.99, 1),          -- Full return
(7, 5, 1, 54.99, 0, 54.99, 0),
(7, 7, 1, 129.99, 0, 129.99, 0),
(8, 5, 1, 54.99, 0, 54.99, 0),
-- ORD-2024-012 items (only in PG)
(9, 4, 1, 39.99, 0, 39.99, 0),
(9, 10, 1, 29.99, 0, 29.99, 0),
-- International order items
(10, 1, 1, 999.99, 0, 999.99, 0),
(11, 4, 1, 39.99, 0, 39.99, 0),
(12, 8, 1, 449.99, 0, 449.99, 0);

-- =============================================================
-- Table: reviews
-- DIFF: 'rating' is SMALLINT here, INT in SQL Server
--       Has 'helpful_votes' and 'reported' columns (SQL Server doesn't)
--       No 'response' column (SQL Server has it for seller responses)
--       'body' is TEXT here, NVARCHAR(MAX) in SQL Server
-- =============================================================
CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    rating SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 5),  -- DIFF: SMALLINT here, INT in SQL Server
    title VARCHAR(200),
    body TEXT,                                          -- DIFF: TEXT here, NVARCHAR(MAX) in SQL Server
    helpful_votes INTEGER NOT NULL DEFAULT 0,           -- ONLY in PostgreSQL
    reported BOOLEAN NOT NULL DEFAULT FALSE,            -- ONLY in PostgreSQL
    -- No 'response' column
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ix_reviews_product ON reviews(product_id);  -- DIFF: PG has this, SQL Server doesn't
CREATE INDEX ix_reviews_rating ON reviews(rating);       -- DIFF: PG has this, SQL Server doesn't

INSERT INTO reviews (product_id, customer_id, rating, title, body, helpful_votes, reported, is_verified) VALUES
(1, 1, 5, 'Amazing phone', 'Best smartphone I have ever owned. Battery life is incredible. The camera takes stunning photos.', 24, FALSE, TRUE),
-- DIFF: rating=5 here, 4 in SQL Server; different title and body
(1, 3, 5, 'Great value flagship', 'Excellent build quality. Worth every penny at this price point. Screen is beautiful.', 18, FALSE, TRUE),
(2, 5, 5, 'Perfect for work', 'Fast, lightweight, and the screen is gorgeous. Thunderbolt ports are essential.', 31, FALSE, TRUE),
(3, 2, 3, 'Decent earbuds', 'Good sound quality but the fit could be better. Noise cancelling is okay.', 8, FALSE, TRUE),
(4, 8, 5, 'Must read for Rustaceans', 'Clear explanations, great examples throughout. Finally understood lifetimes.', 42, FALSE, TRUE),
(5, 3, 4, 'Thorough coverage', 'Very detailed. Some chapters are quite dense but rewarding.', 15, FALSE, FALSE),
(6, 4, 4, 'Keeps me dry', 'Works great in heavy rain. Runs slightly large - recommend sizing down.', 12, FALSE, TRUE),
(8, 1, 5, 'Life changing desk', 'My back pain is gone. Worth every penny. Love the memory presets.', 56, FALSE, TRUE),
(10, 7, 4, 'Good yoga mat', 'Nice grip and thickness. Slight rubber smell initially but fades.', 9, FALSE, TRUE),
-- ONLY in PostgreSQL
(9, 2, 3, 'Decent desk lamp', 'Good brightness range but the USB-C port charges slowly.', 5, FALSE, TRUE),
(12, 9, 5, 'Amazing dash cam', 'Crystal clear video day and night. GPS tracking is a great feature.', 14, FALSE, TRUE),
(13, 10, 4, 'Good vitamin D supplement', 'Easy to swallow softgels. Good value for a year supply.', 7, FALSE, FALSE),
-- Reported/problematic review
(1, 10, 1, 'WORST PHONE EVER', 'This phone is terrible!!! DO NOT BUY!!! SCAM!!!', 2, TRUE, FALSE),
-- Unicode review
(1, 11, 5, 'Ausgezeichnetes Telefon!', 'Sehr gutes Gerät. Die Kameraqualität ist hervorragend. Empfehlenswert! 👍', 3, FALSE, TRUE);

-- =============================================================
-- Table: wishlists (ONLY in PostgreSQL)
-- =============================================================
CREATE TABLE wishlists (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    product_id INTEGER NOT NULL REFERENCES products(id),
    priority SMALLINT NOT NULL DEFAULT 0,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    notes TEXT,
    CONSTRAINT uq_wishlist UNIQUE (customer_id, product_id)
);

INSERT INTO wishlists (customer_id, product_id, priority, notes) VALUES
(1, 2, 1, 'Want for work'),
(1, 10, 2, 'For home gym'),
(2, 1, 1, 'Upgrade from current phone'),
(3, 8, 1, 'Need for home office'),
(5, 12, 1, NULL),
(7, 6, 2, 'For hiking trip'),
(9, 1, 1, 'Birthday gift idea'),
(10, 3, 1, NULL);

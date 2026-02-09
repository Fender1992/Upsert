-- =============================================================
-- Upsert Test Database - SQL Server Seed Script (Complex)
-- Database: UpsertTestSource
-- Designed to stress-test schema comparison, data diff, and migration
-- =============================================================

IF NOT EXISTS (SELECT * FROM sys.databases WHERE name = 'UpsertTestSource')
    CREATE DATABASE UpsertTestSource;
GO

USE UpsertTestSource;
GO

-- Drop everything
IF OBJECT_ID('dbo.audit_log', 'U') IS NOT NULL DROP TABLE dbo.audit_log;
IF OBJECT_ID('dbo.order_items', 'U') IS NOT NULL DROP TABLE dbo.order_items;
IF OBJECT_ID('dbo.reviews', 'U') IS NOT NULL DROP TABLE dbo.reviews;
IF OBJECT_ID('dbo.orders', 'U') IS NOT NULL DROP TABLE dbo.orders;
IF OBJECT_ID('dbo.product_tags', 'U') IS NOT NULL DROP TABLE dbo.product_tags;
IF OBJECT_ID('dbo.products', 'U') IS NOT NULL DROP TABLE dbo.products;
IF OBJECT_ID('dbo.categories', 'U') IS NOT NULL DROP TABLE dbo.categories;
IF OBJECT_ID('dbo.customers', 'U') IS NOT NULL DROP TABLE dbo.customers;
IF OBJECT_ID('dbo.employees', 'U') IS NOT NULL DROP TABLE dbo.employees;
IF OBJECT_ID('dbo.warehouses', 'U') IS NOT NULL DROP TABLE dbo.warehouses;
IF OBJECT_ID('dbo.inventory', 'U') IS NOT NULL DROP TABLE dbo.inventory;
IF OBJECT_ID('dbo.promotions', 'U') IS NOT NULL DROP TABLE dbo.promotions;
GO

-- =============================================================
-- Table: categories
-- DIFF: SQL Server uses NVARCHAR, has 'sort_order' column (missing in PG)
--       PG has 'slug' column (missing here)
--       'description' is NVARCHAR(500) here vs TEXT in PG
-- =============================================================
CREATE TABLE dbo.categories (
    id INT IDENTITY(1,1) PRIMARY KEY,
    name NVARCHAR(100) NOT NULL,
    description NVARCHAR(500) NULL,
    parent_id INT NULL REFERENCES dbo.categories(id),
    sort_order INT NOT NULL DEFAULT 0,            -- ONLY in SQL Server
    is_active BIT NOT NULL DEFAULT 1,
    created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
    updated_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
);

CREATE INDEX ix_categories_parent ON dbo.categories(parent_id);
-- DIFF: SQL Server has this index, PG does not

INSERT INTO dbo.categories (name, description, parent_id, sort_order, is_active) VALUES
('Electronics', 'Phones, laptops, tablets and accessories', NULL, 1, 1),
('Books', 'Physical and digital books', NULL, 2, 1),
('Clothing', 'Apparel and fashion accessories', NULL, 3, 1),
('Home & Garden', 'Furniture, decor and garden supplies', NULL, 4, 1),
('Sports', 'Sporting goods and fitness equipment', NULL, 5, 1),
('Toys', 'Games, puzzles and children toys', NULL, 6, 0),
-- Sub-categories (only in SQL Server)
('Smartphones', 'Mobile phones and phablets', 1, 1, 1),
('Laptops', 'Notebook computers', 1, 2, 1),
('Audio', 'Headphones, speakers, earbuds', 1, 3, 1),
('Fiction', 'Novels and fiction', 2, 1, 1),
('Technical', 'Programming and engineering books', 2, 2, 1);

-- =============================================================
-- Table: customers
-- DIFF: SQL Server uses NVARCHAR, has 'middle_name' (missing in PG)
--       PG has 'date_of_birth' DATE column (missing here)
--       'loyalty_points' is INT here, BIGINT in PG
--       'phone' is NVARCHAR(20) here, VARCHAR(30) in PG
--       'email' max length 255 here, 200 in PG
-- =============================================================
CREATE TABLE dbo.customers (
    id INT IDENTITY(1,1) PRIMARY KEY,
    email NVARCHAR(255) NOT NULL UNIQUE,
    first_name NVARCHAR(100) NOT NULL,
    middle_name NVARCHAR(100) NULL,               -- ONLY in SQL Server
    last_name NVARCHAR(100) NOT NULL,
    phone NVARCHAR(20) NULL,
    address_line1 NVARCHAR(300) NULL,              -- DIFF: named 'address_line1' here, 'street_address' in PG
    address_line2 NVARCHAR(300) NULL,              -- DIFF: exists here, missing in PG
    city NVARCHAR(100) NULL,
    state NVARCHAR(50) NULL,                       -- DIFF: named 'state' here, 'region' in PG
    zip_code NVARCHAR(10) NULL,                    -- DIFF: named 'zip_code' here, 'postal_code' in PG
    country NVARCHAR(2) NOT NULL DEFAULT 'US',
    loyalty_points INT NOT NULL DEFAULT 0,         -- DIFF: INT here, BIGINT in PG
    credit_limit MONEY NULL,                       -- DIFF: MONEY type (SQL Server specific)
    is_active BIT NOT NULL DEFAULT 1,
    notes NVARCHAR(MAX) NULL,                      -- DIFF: NVARCHAR(MAX) here, TEXT in PG
    created_at DATETIME2(3) NOT NULL DEFAULT GETUTCDATE(),  -- DIFF: precision 3 here, 6 in PG
    updated_at DATETIME2(3) NOT NULL DEFAULT GETUTCDATE()
);

CREATE INDEX ix_customers_email ON dbo.customers(email);
CREATE INDEX ix_customers_state ON dbo.customers(state);     -- DIFF: PG has index on 'region' instead

INSERT INTO dbo.customers (email, first_name, middle_name, last_name, phone, address_line1, address_line2, city, state, zip_code, country, loyalty_points, credit_limit, notes) VALUES
('alice@example.com', 'Alice', 'Marie', 'Johnson', '555-0101', '123 Main St', 'Apt 4B', 'Springfield', 'IL', '62701', 'US', 1500, 5000.00, NULL),
('bob@example.com', 'Bob', NULL, 'Smith', '555-0102', '456 Oak Ave', NULL, 'Portland', 'OR', '97201', 'US', 820, 3000.00, 'Preferred customer'),
-- DIFF: Carol has loyalty_points=2100 here, 2400 in PG; different phone format
('carol@example.com', 'Carol', 'Ann', 'Williams', '(555) 010-3', '789 Pine Rd', NULL, 'Austin', 'TX', '73301', 'US', 2100, 7500.00, NULL),
('dave@example.com', 'Dave', NULL, 'Brown', '555-0104', '321 Elm St', NULL, 'Denver', 'CO', '80201', 'US', 450, 2000.00, NULL),
-- DIFF: Eve has phone='555-0105' here, '555-9999' in PG
('eve@example.com', 'Eve', 'Louise', 'Davis', '555-0105', '654 Maple Dr', 'Suite 200', 'Seattle', 'WA', '98101', 'US', 3200, 10000.00, 'VIP customer - handle with care'),
('frank@example.com', 'Frank', NULL, 'Miller', '555-0106', '987 Cedar Ln', NULL, 'Miami', 'FL', '33101', 'US', 150, 1000.00, NULL),
('grace@example.com', 'Grace', NULL, 'Wilson', '555-0107', '147 Birch Ct', NULL, 'Boston', 'MA', '02101', 'US', 920, 4000.00, NULL),
-- DIFF: Hank has different address
('hank@example.com', 'Hank', 'James', 'Moore', '555-0108', '258 Walnut Pl', 'Floor 3', 'Chicago', 'IL', '60601', 'US', 1750, 6000.00, NULL),
-- ONLY in SQL Server
('ivan@example.com', 'Ivan', NULL, 'Taylor', '555-0109', '369 Ash Blvd', NULL, 'Phoenix', 'AZ', '85001', 'US', 500, 2500.00, NULL),
('julia@example.com', 'Julia', 'Rose', 'Anderson', '555-0110', '480 Spruce Way', NULL, 'Nashville', 'TN', '37201', 'US', 680, 3500.00, NULL),
-- Unicode stress test
(N'münchen@example.de', N'Müller', NULL, N'Strauß', '+49-89-12345', N'Königstraße 42', NULL, N'München', N'BY', N'80331', 'DE', 100, 2000.00, N'German customer — special chars: äöüß €'),
(N'tokyo@example.jp', N'太郎', NULL, N'山田', '+81-3-1234-5678', N'東京都渋谷区神宮前1-2-3', NULL, N'東京', NULL, N'150-0001', 'JP', 200, 5000.00, N'Japanese customer: 日本語テスト');

-- =============================================================
-- Table: products
-- DIFF: 'weight_kg' is DECIMAL(6,2) here, REAL in PG
--       'sku' is NVARCHAR(50) here, CHAR(12) in PG (fixed width)
--       PG has 'color' and 'size' columns (missing here)
--       SQL Server has 'reorder_point' column (missing in PG)
--       'price' is DECIMAL(10,2) here, NUMERIC(12,4) in PG
-- =============================================================
CREATE TABLE dbo.products (
    id INT IDENTITY(1,1) PRIMARY KEY,
    sku NVARCHAR(50) NOT NULL UNIQUE,
    name NVARCHAR(200) NOT NULL,
    description NVARCHAR(MAX) NULL,                -- DIFF: NVARCHAR(MAX) here, VARCHAR(2000) in PG
    category_id INT NOT NULL REFERENCES dbo.categories(id),
    price DECIMAL(10, 2) NOT NULL,                 -- DIFF: DECIMAL(10,2) here, NUMERIC(12,4) in PG
    cost DECIMAL(10, 2) NULL,
    stock_quantity INT NOT NULL DEFAULT 0,
    reorder_point INT NOT NULL DEFAULT 10,         -- ONLY in SQL Server
    weight_kg DECIMAL(6, 2) NULL,                  -- DIFF: DECIMAL(6,2) here, REAL in PG
    is_active BIT NOT NULL DEFAULT 1,
    created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
    updated_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
    CONSTRAINT chk_price_positive CHECK (price > 0),
    CONSTRAINT chk_stock_nonneg CHECK (stock_quantity >= 0)
);

CREATE INDEX ix_products_category ON dbo.products(category_id);
CREATE INDEX ix_products_price ON dbo.products(price);       -- DIFF: PG does not have this index

INSERT INTO dbo.products (sku, name, description, category_id, price, cost, stock_quantity, reorder_point, weight_kg) VALUES
('ELEC-001', 'Smartphone Pro X', '6.7 inch OLED display, 128GB storage, 5G capable. Features advanced AI camera system.', 7, 999.99, 650.00, 45, 20, 0.19),
('ELEC-002', 'Laptop Ultra 15', '15.6 inch 4K display, 16GB RAM, 512GB NVMe SSD. Thunderbolt 4 ports.', 8, 1299.99, 850.00, 22, 10, 1.80),
-- DIFF: Earbuds price=149.99 here, 139.9900 in PG
('ELEC-003', 'Wireless Earbuds', 'Active noise cancelling, 24hr battery life with case. IPX4 water resistant.', 9, 149.99, 55.00, 150, 50, 0.05),
('BOOK-001', 'The Rust Programming Language', 'Official Rust book, 2nd edition. Covers ownership, lifetimes, and async.', 11, 39.99, 12.00, 200, 25, 0.45),
('BOOK-002', 'Database Internals', 'Deep dive into storage engines, distributed systems, and consensus protocols.', 11, 54.99, 18.00, 85, 15, 0.60),
('CLTH-001', 'Waterproof Jacket', 'All-weather outdoor jacket. Gore-Tex membrane, sealed seams.', 3, 89.99, 35.00, 60, 20, 0.70),
-- DIFF: Running Shoes stock=35 here, 28 in PG
('CLTH-002', 'Running Shoes', 'Lightweight performance shoes. Carbon fiber plate, responsive foam.', 3, 129.99, 48.00, 35, 15, 0.65),
('HOME-001', 'Standing Desk', 'Electric adjustable height desk. Memory presets, cable management.', 4, 449.99, 200.00, 15, 5, 25.00),
('HOME-002', 'LED Desk Lamp', 'Dimmable 5-mode lighting with USB-C charging port. Touch control.', 4, 34.99, 10.00, 120, 30, 0.80),
('SPRT-001', 'Yoga Mat Premium', 'Non-slip TPE material, extra thick 6mm. Alignment markers.', 5, 29.99, 8.00, 200, 40, 1.20),
('SPRT-002', 'Dumbbell Set 20kg', 'Adjustable weight set. Quick-lock mechanism, rubber coated.', 5, 79.99, 30.00, 40, 10, 20.00),
-- ONLY in SQL Server
('ELEC-004', 'Tablet Mini 8', '8 inch IPS tablet, 64GB storage. Stylus support.', 1, 349.99, 180.00, 30, 10, 0.32),
('ELEC-005', '4K Webcam Pro', 'Ultra HD webcam with auto-framing and noise cancelling mic.', 1, 179.99, 75.00, 65, 20, 0.15),
-- NULL description stress test
('MISC-001', 'Mystery Box', NULL, 6, 19.99, NULL, 999, 100, NULL),
-- Very long description
('BOOK-003', 'Encyclopedia of Computing', REPLICATE(N'This is a comprehensive guide to computing concepts. ', 40), 11, 89.99, 45.00, 12, 5, 2.50);

-- =============================================================
-- Table: product_tags (ONLY in SQL Server - no equivalent in PG)
-- Tests table-level schema diff
-- =============================================================
CREATE TABLE dbo.product_tags (
    id INT IDENTITY(1,1) PRIMARY KEY,
    product_id INT NOT NULL REFERENCES dbo.products(id),
    tag_name NVARCHAR(50) NOT NULL,
    created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
    CONSTRAINT uq_product_tag UNIQUE (product_id, tag_name)
);

INSERT INTO dbo.product_tags (product_id, tag_name) VALUES
(1, 'flagship'), (1, '5G'), (1, 'best-seller'),
(2, 'portable'), (2, 'business'),
(3, 'wireless'), (3, 'noise-cancelling'),
(4, 'programming'), (4, 'rust'),
(7, 'running'), (7, 'athletic'),
(8, 'ergonomic'), (8, 'office');

-- =============================================================
-- Table: employees (ONLY in SQL Server)
-- =============================================================
CREATE TABLE dbo.employees (
    id INT IDENTITY(1,1) PRIMARY KEY,
    employee_number NVARCHAR(20) NOT NULL UNIQUE,
    first_name NVARCHAR(100) NOT NULL,
    last_name NVARCHAR(100) NOT NULL,
    department NVARCHAR(100) NOT NULL,
    hire_date DATE NOT NULL,
    salary DECIMAL(10, 2) NOT NULL,
    manager_id INT NULL REFERENCES dbo.employees(id),
    is_active BIT NOT NULL DEFAULT 1
);

INSERT INTO dbo.employees (employee_number, first_name, last_name, department, hire_date, salary, manager_id) VALUES
('EMP-001', 'Sarah', 'Connor', 'Engineering', '2022-03-15', 95000.00, NULL),
('EMP-002', 'John', 'Reese', 'Sales', '2023-01-10', 72000.00, 1),
('EMP-003', 'Kyle', 'Murphy', 'Support', '2023-06-01', 65000.00, 1),
('EMP-004', 'Maria', 'Garcia', 'Engineering', '2023-09-01', 88000.00, 1),
('EMP-005', 'Chen', 'Wei', 'Sales', '2024-01-15', 70000.00, 2);

-- =============================================================
-- Table: orders
-- DIFF: 'status' is NVARCHAR(20) here, uses PG ENUM type in PG
--       'shipping_address' is single column here, split into components in PG
--       SQL Server has 'discount_code' column (missing in PG)
--       PG has 'currency' column (missing here)
--       'total' is DECIMAL(10,2) here, DECIMAL(12,2) in PG
-- =============================================================
CREATE TABLE dbo.orders (
    id INT IDENTITY(1,1) PRIMARY KEY,
    order_number NVARCHAR(20) NOT NULL UNIQUE,
    customer_id INT NOT NULL REFERENCES dbo.customers(id),
    status NVARCHAR(20) NOT NULL DEFAULT 'pending',
    subtotal DECIMAL(10, 2) NOT NULL,              -- DIFF: DECIMAL(10,2) here, DECIMAL(12,2) in PG
    tax DECIMAL(10, 2) NOT NULL DEFAULT 0,
    shipping_cost DECIMAL(10, 2) NOT NULL DEFAULT 0, -- DIFF: exists here, missing in PG
    total DECIMAL(10, 2) NOT NULL,
    discount_code NVARCHAR(20) NULL,               -- ONLY in SQL Server
    shipping_address NVARCHAR(500) NULL,            -- DIFF: single field here, split in PG
    notes NVARCHAR(1000) NULL,
    ordered_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
    shipped_at DATETIME2 NULL,
    delivered_at DATETIME2 NULL,
    created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
    updated_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
);

CREATE INDEX ix_orders_customer ON dbo.orders(customer_id);
CREATE INDEX ix_orders_status ON dbo.orders(status);
CREATE INDEX ix_orders_date ON dbo.orders(ordered_at);

INSERT INTO dbo.orders (order_number, customer_id, status, subtotal, tax, shipping_cost, total, discount_code, shipping_address, notes, ordered_at, shipped_at, delivered_at) VALUES
('ORD-2024-001', 1, 'delivered', 1149.98, 97.75, 0.00, 1247.73, NULL, '123 Main St Apt 4B, Springfield IL 62701', NULL, '2024-01-15 10:30:00', '2024-01-17 08:00:00', '2024-01-20 14:30:00'),
('ORD-2024-002', 2, 'delivered', 39.99, 3.40, 5.99, 49.38, 'WELCOME10', '456 Oak Ave, Portland OR 97201', 'Gift wrap please', '2024-01-20 14:15:00', '2024-01-21 09:00:00', '2024-01-25 11:00:00'),
-- DIFF: status='shipped' here, 'delivered' in PG
('ORD-2024-003', 3, 'shipped', 219.98, 18.70, 0.00, 238.68, NULL, '789 Pine Rd, Austin TX 73301', NULL, '2024-02-05 09:00:00', '2024-02-07 10:00:00', NULL),
('ORD-2024-004', 1, 'delivered', 449.99, 38.25, 0.00, 488.24, 'LOYALTY20', '123 Main St Apt 4B, Springfield IL 62701', 'Leave at door', '2024-02-10 16:45:00', '2024-02-12 08:00:00', '2024-02-15 13:00:00'),
('ORD-2024-005', 5, 'pending', 79.99, 6.80, 12.99, 99.78, NULL, '654 Maple Dr Suite 200, Seattle WA 98101', NULL, '2024-03-01 11:20:00', NULL, NULL),
-- DIFF: status='cancelled' here, 'refunded' in PG; different notes
('ORD-2024-006', 4, 'cancelled', 129.99, 11.05, 0.00, 141.04, NULL, '321 Elm St, Denver CO 80201', 'Wrong size', '2024-03-05 13:00:00', NULL, NULL),
('ORD-2024-007', 8, 'delivered', 184.98, 15.72, 5.99, 206.69, NULL, '258 Walnut Pl Floor 3, Chicago IL 60601', NULL, '2024-03-10 08:30:00', '2024-03-11 09:00:00', '2024-03-14 16:00:00'),
('ORD-2024-008', 3, 'shipped', 54.99, 4.67, 0.00, 59.66, 'FREESHIP', '789 Pine Rd, Austin TX 73301', 'Expedited shipping', '2024-03-15 17:00:00', '2024-03-16 08:00:00', NULL),
-- ONLY in SQL Server
('ORD-2024-009', 9, 'pending', 349.99, 29.75, 0.00, 379.74, NULL, '369 Ash Blvd, Phoenix AZ 85001', NULL, '2024-03-20 10:00:00', NULL, NULL),
('ORD-2024-010', 10, 'processing', 259.98, 22.10, 5.99, 288.07, 'SPRING25', '480 Spruce Way, Nashville TN 37201', NULL, '2024-03-25 09:30:00', NULL, NULL),
-- ONLY in SQL Server - large order
('ORD-2024-011', 5, 'delivered', 2549.97, 216.75, 0.00, 2766.72, 'VIP50', '654 Maple Dr Suite 200, Seattle WA 98101', 'Bulk order for office', '2024-04-01 10:00:00', '2024-04-02 08:00:00', '2024-04-05 14:00:00');

-- =============================================================
-- Table: order_items
-- DIFF: SQL Server has 'notes' column (missing in PG)
--       PG has 'returned_quantity' column (missing here)
-- =============================================================
CREATE TABLE dbo.order_items (
    id INT IDENTITY(1,1) PRIMARY KEY,
    order_id INT NOT NULL REFERENCES dbo.orders(id),
    product_id INT NOT NULL REFERENCES dbo.products(id),
    quantity INT NOT NULL DEFAULT 1,
    unit_price DECIMAL(10, 2) NOT NULL,
    discount_pct DECIMAL(5, 2) NOT NULL DEFAULT 0, -- DIFF: named 'discount_pct' (percentage) here, 'discount_amount' (flat) in PG
    line_total DECIMAL(10, 2) NOT NULL,
    notes NVARCHAR(500) NULL,                      -- ONLY in SQL Server
    created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
);

INSERT INTO dbo.order_items (order_id, product_id, quantity, unit_price, discount_pct, line_total, notes) VALUES
(1, 1, 1, 999.99, 0, 999.99, NULL),
(1, 3, 1, 149.99, 0, 149.99, NULL),
(2, 4, 1, 39.99, 10, 35.99, 'Gift wrapped'),
(3, 6, 1, 89.99, 0, 89.99, NULL),
(3, 7, 1, 129.99, 0, 129.99, NULL),
(4, 8, 1, 449.99, 20, 359.99, 'Loyalty discount applied'),
(5, 11, 1, 79.99, 0, 79.99, NULL),
(6, 7, 1, 129.99, 0, 129.99, NULL),
(7, 5, 1, 54.99, 0, 54.99, NULL),
(7, 7, 1, 129.99, 0, 129.99, NULL),
(8, 5, 1, 54.99, 0, 54.99, 'Expedited'),
(9, 12, 1, 349.99, 0, 349.99, NULL),
(10, 1, 1, 999.99, 25, 749.99, 'Spring sale'),
(10, 3, 1, 149.99, 25, 112.49, 'Spring sale'),
(11, 1, 1, 999.99, 50, 499.99, 'VIP discount'),
(11, 2, 1, 1299.99, 50, 649.99, 'VIP discount'),
(11, 8, 1, 449.99, 0, 449.99, NULL);

-- =============================================================
-- Table: reviews
-- DIFF: 'rating' is INT here, SMALLINT in PG
--       PG has 'helpful_votes' and 'reported' columns (missing here)
--       SQL Server has 'response' column for seller responses (missing in PG)
-- =============================================================
CREATE TABLE dbo.reviews (
    id INT IDENTITY(1,1) PRIMARY KEY,
    product_id INT NOT NULL REFERENCES dbo.products(id),
    customer_id INT NOT NULL REFERENCES dbo.customers(id),
    rating INT NOT NULL CHECK (rating >= 1 AND rating <= 5),
    title NVARCHAR(200) NULL,
    body NVARCHAR(MAX) NULL,                       -- DIFF: NVARCHAR(MAX) here, TEXT in PG
    response NVARCHAR(MAX) NULL,                   -- ONLY in SQL Server (seller response)
    is_verified BIT NOT NULL DEFAULT 0,
    created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
);

INSERT INTO dbo.reviews (product_id, customer_id, rating, title, body, response, is_verified) VALUES
(1, 1, 5, 'Amazing phone', 'Best smartphone I have ever owned. Battery life is incredible. The camera takes stunning photos even in low light.', 'Thank you for your kind review!', 1),
-- DIFF: rating=4 here, 5 in PG; different title and body
(1, 3, 4, 'Great but pricey', 'Excellent build quality. Wish it was a bit cheaper. The screen is gorgeous though.', NULL, 1),
(2, 5, 5, 'Perfect for work', 'Fast, lightweight, and the screen is gorgeous. Thunderbolt ports are a game changer.', NULL, 1),
(3, 2, 3, 'Decent earbuds', 'Good sound quality but the fit could be better. ANC works well on planes.', 'We recommend trying the included medium ear tips.', 1),
(4, 8, 5, 'Must read for Rustaceans', 'Clear explanations, great examples throughout. The ownership chapter finally made it click.', NULL, 1),
(5, 3, 4, 'Thorough coverage', 'Very detailed. Some chapters are quite dense but worth the effort.', NULL, 0),
(6, 4, 4, 'Keeps me dry', 'Works great in heavy rain. Runs slightly large - size down if between sizes.', NULL, 1),
(8, 1, 5, 'Life changing desk', 'My back pain is gone. Worth every penny. The presets are super convenient.', 'Glad to hear about your improved comfort!', 1),
(10, 7, 4, 'Good yoga mat', 'Nice grip and thickness. Slight rubber smell initially but goes away after a week.', NULL, 1),
-- ONLY in SQL Server
(1, 8, 4, 'Solid upgrade', 'Coming from an older model, huge improvement in every way.', NULL, 1),
(12, 9, 5, 'Great tablet for reading', 'Perfect size for ebooks and light browsing. Battery lasts all day.', NULL, 1),
-- Unicode review
(1, 11, 5, N'Ausgezeichnetes Telefon!', N'Sehr gutes Gerät. Die Kameraqualität ist hervorragend. Preis-Leistungs-Verhältnis stimmt. 👍', NULL, 1);

-- =============================================================
-- Table: warehouses (ONLY in SQL Server)
-- =============================================================
CREATE TABLE dbo.warehouses (
    id INT IDENTITY(1,1) PRIMARY KEY,
    code NVARCHAR(10) NOT NULL UNIQUE,
    name NVARCHAR(200) NOT NULL,
    address NVARCHAR(500) NOT NULL,
    capacity INT NOT NULL,
    is_active BIT NOT NULL DEFAULT 1
);

INSERT INTO dbo.warehouses (code, name, address, capacity) VALUES
('WH-EAST', 'East Coast Fulfillment', '100 Warehouse Blvd, Newark NJ 07101', 50000),
('WH-WEST', 'West Coast Distribution', '200 Logistics Way, Ontario CA 91761', 75000),
('WH-CENT', 'Central Hub', '300 Distribution Dr, Dallas TX 75201', 60000);

-- =============================================================
-- Table: inventory (ONLY in SQL Server - FK to warehouses)
-- =============================================================
CREATE TABLE dbo.inventory (
    id INT IDENTITY(1,1) PRIMARY KEY,
    warehouse_id INT NOT NULL REFERENCES dbo.warehouses(id),
    product_id INT NOT NULL REFERENCES dbo.products(id),
    quantity INT NOT NULL DEFAULT 0,
    last_counted_at DATETIME2 NULL,
    CONSTRAINT uq_warehouse_product UNIQUE (warehouse_id, product_id)
);

INSERT INTO dbo.inventory (warehouse_id, product_id, quantity, last_counted_at) VALUES
(1, 1, 20, '2024-03-01'), (1, 2, 10, '2024-03-01'), (1, 3, 75, '2024-03-01'),
(2, 1, 15, '2024-03-01'), (2, 2, 8, '2024-03-01'), (2, 3, 50, '2024-03-01'),
(3, 1, 10, '2024-03-01'), (3, 4, 200, '2024-03-01'), (3, 8, 15, '2024-03-01');

-- =============================================================
-- Table: promotions (ONLY in SQL Server)
-- =============================================================
CREATE TABLE dbo.promotions (
    id INT IDENTITY(1,1) PRIMARY KEY,
    code NVARCHAR(20) NOT NULL UNIQUE,
    description NVARCHAR(500) NULL,
    discount_type NVARCHAR(20) NOT NULL,  -- 'percentage' or 'fixed'
    discount_value DECIMAL(10, 2) NOT NULL,
    min_order_amount DECIMAL(10, 2) NULL,
    starts_at DATETIME2 NOT NULL,
    ends_at DATETIME2 NOT NULL,
    max_uses INT NULL,
    current_uses INT NOT NULL DEFAULT 0,
    is_active BIT NOT NULL DEFAULT 1
);

INSERT INTO dbo.promotions (code, description, discount_type, discount_value, min_order_amount, starts_at, ends_at, max_uses) VALUES
('WELCOME10', 'New customer 10% off', 'percentage', 10.00, 25.00, '2024-01-01', '2024-12-31', 1000),
('LOYALTY20', '20% off for loyalty members', 'percentage', 20.00, 100.00, '2024-01-01', '2024-12-31', NULL),
('FREESHIP', 'Free shipping on all orders', 'fixed', 12.99, 50.00, '2024-03-01', '2024-03-31', 500),
('SPRING25', 'Spring sale 25% off', 'percentage', 25.00, NULL, '2024-03-20', '2024-04-20', NULL),
('VIP50', 'VIP 50% off everything', 'percentage', 50.00, NULL, '2024-04-01', '2024-04-07', 50);

-- =============================================================
-- Table: audit_log (ONLY in SQL Server)
-- =============================================================
CREATE TABLE dbo.audit_log (
    id BIGINT IDENTITY(1,1) PRIMARY KEY,
    table_name NVARCHAR(100) NOT NULL,
    record_id INT NOT NULL,
    action NVARCHAR(10) NOT NULL,  -- INSERT, UPDATE, DELETE
    old_values NVARCHAR(MAX) NULL,
    new_values NVARCHAR(MAX) NULL,
    changed_by NVARCHAR(100) NOT NULL DEFAULT SYSTEM_USER,
    changed_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
);

INSERT INTO dbo.audit_log (table_name, record_id, action, old_values, new_values, changed_at) VALUES
('orders', 3, 'UPDATE', '{"status":"pending"}', '{"status":"shipped"}', '2024-02-07 10:00:00'),
('orders', 6, 'UPDATE', '{"status":"pending"}', '{"status":"cancelled"}', '2024-03-06 09:00:00'),
('customers', 1, 'UPDATE', '{"loyalty_points":1200}', '{"loyalty_points":1500}', '2024-02-01 12:00:00');

PRINT 'SQL Server seed complete: UpsertTestSource database ready with complex schema.';
GO

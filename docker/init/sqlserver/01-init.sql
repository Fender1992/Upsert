-- Wait for SQL Server to be ready, then create test database and sample tables
IF NOT EXISTS (SELECT name FROM sys.databases WHERE name = 'upsert_test')
BEGIN
    CREATE DATABASE upsert_test;
END
GO

USE upsert_test;
GO

IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'customers')
BEGIN
    CREATE TABLE customers (
        id INT IDENTITY(1,1) PRIMARY KEY,
        first_name NVARCHAR(100) NOT NULL,
        last_name NVARCHAR(100) NOT NULL,
        email VARCHAR(255) UNIQUE,
        phone VARCHAR(20),
        balance DECIMAL(18,2) DEFAULT 0.00,
        is_active BIT DEFAULT 1,
        created_at DATETIME2 DEFAULT GETUTCDATE(),
        updated_at DATETIMEOFFSET DEFAULT SYSDATETIMEOFFSET(),
        notes NVARCHAR(MAX),
        profile_image VARBINARY(MAX),
        customer_uuid UNIQUEIDENTIFIER DEFAULT NEWID()
    );
END
GO

IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'orders')
BEGIN
    CREATE TABLE orders (
        id BIGINT IDENTITY(1,1) PRIMARY KEY,
        customer_id INT NOT NULL REFERENCES customers(id),
        order_date DATE NOT NULL DEFAULT CAST(GETUTCDATE() AS DATE),
        total_amount MONEY NOT NULL,
        tax_amount SMALLMONEY,
        status TINYINT DEFAULT 0,
        shipping_weight REAL,
        discount_pct FLOAT,
        order_xml XML,
        order_data NVARCHAR(MAX),
        CONSTRAINT fk_orders_customer FOREIGN KEY (customer_id) REFERENCES customers(id)
    );
END
GO

IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'type_showcase')
BEGIN
    CREATE TABLE type_showcase (
        col_bit BIT,
        col_tinyint TINYINT,
        col_smallint SMALLINT,
        col_int INT,
        col_bigint BIGINT,
        col_decimal DECIMAL(18,4),
        col_numeric NUMERIC(10,2),
        col_money MONEY,
        col_smallmoney SMALLMONEY,
        col_float FLOAT,
        col_real REAL,
        col_char CHAR(10),
        col_varchar VARCHAR(255),
        col_varchar_max VARCHAR(MAX),
        col_nchar NCHAR(10),
        col_nvarchar NVARCHAR(100),
        col_nvarchar_max NVARCHAR(MAX),
        col_text TEXT,
        col_ntext NTEXT,
        col_binary BINARY(16),
        col_varbinary VARBINARY(256),
        col_varbinary_max VARBINARY(MAX),
        col_date DATE,
        col_time TIME,
        col_datetime DATETIME,
        col_datetime2 DATETIME2,
        col_smalldatetime SMALLDATETIME,
        col_datetimeoffset DATETIMEOFFSET,
        col_uniqueidentifier UNIQUEIDENTIFIER,
        col_xml XML
    );
END
GO

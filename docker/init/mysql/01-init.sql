-- MySQL init script for Upsert integration tests

USE upsert_test;

CREATE TABLE IF NOT EXISTS customers (
    id INT AUTO_INCREMENT PRIMARY KEY,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    phone VARCHAR(20),
    balance DECIMAL(18,2) DEFAULT 0.00,
    is_active TINYINT(1) DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    notes LONGTEXT,
    profile_image LONGBLOB,
    customer_uuid CHAR(36) DEFAULT (UUID())
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS orders (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    customer_id INT NOT NULL,
    order_date DATE NOT NULL DEFAULT (CURRENT_DATE),
    total_amount DECIMAL(19,4) NOT NULL,
    tax_amount DECIMAL(10,4),
    status TINYINT DEFAULT 0,
    shipping_weight FLOAT,
    discount_pct DOUBLE,
    order_data JSON,
    CONSTRAINT fk_orders_customer FOREIGN KEY (customer_id) REFERENCES customers(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS type_showcase (
    col_tinyint TINYINT,
    col_tinyint_bool TINYINT(1),
    col_smallint SMALLINT,
    col_mediumint MEDIUMINT,
    col_int INT,
    col_bigint BIGINT,
    col_float FLOAT,
    col_double DOUBLE,
    col_decimal DECIMAL(18,4),
    col_numeric NUMERIC(10,2),
    col_char CHAR(10),
    col_varchar VARCHAR(255),
    col_tinytext TINYTEXT,
    col_text TEXT,
    col_mediumtext MEDIUMTEXT,
    col_longtext LONGTEXT,
    col_binary BINARY(16),
    col_varbinary VARBINARY(256),
    col_tinyblob TINYBLOB,
    col_blob BLOB,
    col_mediumblob MEDIUMBLOB,
    col_longblob LONGBLOB,
    col_date DATE,
    col_time TIME,
    col_datetime DATETIME,
    col_timestamp TIMESTAMP NULL,
    col_year YEAR,
    col_json JSON,
    col_enum ENUM('small', 'medium', 'large'),
    col_set SET('red', 'green', 'blue')
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

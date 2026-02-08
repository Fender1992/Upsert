// MongoDB init script for Upsert integration tests

db = db.getSiblingDB('upsert_test');

db.createCollection('customers');
db.createCollection('orders');
db.createCollection('type_showcase');

db.customers.insertMany([
    {
        first_name: "Alice",
        last_name: "Smith",
        email: "alice@example.com",
        phone: "+1-555-0101",
        balance: NumberDecimal("1250.50"),
        is_active: true,
        created_at: new Date(),
        notes: "VIP customer",
        customer_uuid: UUID().toString()
    },
    {
        first_name: "Bob",
        last_name: "Jones",
        email: "bob@example.com",
        phone: "+1-555-0102",
        balance: NumberDecimal("340.00"),
        is_active: true,
        created_at: new Date(),
        notes: null,
        customer_uuid: UUID().toString()
    }
]);

db.orders.insertMany([
    {
        customer_email: "alice@example.com",
        order_date: new Date(),
        total_amount: NumberDecimal("99.99"),
        tax_amount: NumberDecimal("8.00"),
        status: NumberInt(1),
        shipping_weight: 2.5,
        items: [
            { name: "Widget A", qty: NumberInt(2), price: NumberDecimal("49.99") }
        ]
    }
]);

db.type_showcase.insertOne({
    col_bool: true,
    col_int32: NumberInt(42),
    col_int64: NumberLong("9223372036854775807"),
    col_double: 3.14159,
    col_decimal128: NumberDecimal("12345678901234567890.123456"),
    col_string: "Hello, World!",
    col_date: new Date(),
    col_objectid: new ObjectId(),
    col_bindata: BinData(0, "SGVsbG8="),
    col_array: [1, 2, 3],
    col_object: { nested: true, value: "test" },
    col_null: null,
    col_regex: /^test/i
});

// Create indexes
db.customers.createIndex({ email: 1 }, { unique: true });
db.orders.createIndex({ customer_email: 1 });

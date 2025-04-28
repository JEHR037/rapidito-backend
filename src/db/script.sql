-- Crear los tipos enum
CREATE TYPE StoreTheme AS ENUM ('Default', 'Custom');
CREATE TYPE StoreStatus AS ENUM ('Active', 'Inactive', 'Deleted');
CREATE TYPE OrderStatus AS ENUM ('Pending', 'Processing', 'Delivering', 'Completed', 'Cancelled');
CREATE TYPE UserRole AS ENUM ('Admin', 'Partner', 'Driver', 'Customer');
CREATE TYPE UserStatus AS ENUM ('Active', 'Inactive', 'Deleted');
CREATE TYPE ProductStatus AS ENUM ('Active', 'Inactive', 'Deleted','Offers');

-- Crear la tabla de usuarios
CREATE TABLE users (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role UserRole NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    status UserStatus NOT NULL
);

-- Crear la tabla de tiendas
CREATE TABLE stores (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    logo TEXT NOT NULL,
    cover TEXT NOT NULL,
    theme StoreTheme NOT NULL,
    rating REAL NOT NULL,
    delivery_time TEXT NOT NULL,
    categories TEXT[] NOT NULL,
    popularity INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    status StoreStatus NOT NULL
);

-- Crear la tabla de conductores
CREATE TABLE drivers (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    photo TEXT NOT NULL,
    rating REAL NOT NULL,
    vehicle TEXT NOT NULL,
    license_plate TEXT NOT NULL,
    email TEXT NOT NULL,
    status UserStatus NOT NULL,
    deliveries INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- Crear la tabla de productos
CREATE TABLE products (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    image TEXT NOT NULL,
    store_id UUID NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    category TEXT NOT NULL,
    popularity INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    status ProductStatus NOT NULL
);

-- Crear la tabla de órdenes
CREATE TABLE orders (
    id UUID PRIMARY KEY,
    store_id UUID NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    driver_id UUID REFERENCES drivers(id),
    status OrderStatus NOT NULL,
    total DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    estimated_delivery_time TEXT,
    customer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    address TEXT NOT NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL
);

-- Crear la tabla de items de orden
CREATE TABLE order_items (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
);

-- Crear índices
CREATE INDEX idx_products_store_id ON products(store_id);
CREATE INDEX idx_orders_store_id ON orders(store_id);
CREATE INDEX idx_orders_driver_id ON orders(driver_id);
CREATE INDEX idx_order_items_order_id ON order_items(order_id);
CREATE INDEX idx_order_items_product_id ON order_items(product_id);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);

-- Insertar un usuario administrador
INSERT INTO users (
    id, name, email, password_hash, role, created_at, updated_at
) VALUES (
    gen_random_uuid(),
    'Admin',
    'admin@rapidito.com',
    '$2a$12$',
    'admin',
    NOW(),
    NOW()
);
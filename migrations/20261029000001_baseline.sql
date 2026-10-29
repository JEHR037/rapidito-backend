DO $$
BEGIN
    IF to_regclass('public.users') IS NOT NULL THEN
        RETURN;
    END IF;

    CREATE TYPE public.orderstatus AS ENUM ('Pending', 'Processing', 'Delivering', 'Completed', 'Cancelled');
    CREATE TYPE public.productstatus AS ENUM ('Active', 'Inactive', 'Deleted', 'Offers');
    CREATE TYPE public.storestatus AS ENUM ('Active', 'Inactive', 'Deleted');
    CREATE TYPE public.storetheme AS ENUM ('Default', 'Custom');
    CREATE TYPE public.userrole AS ENUM ('Admin', 'Partner', 'Driver', 'Customer');
    CREATE TYPE public.userstatus AS ENUM ('Active', 'Inactive', 'Deleted');

    CREATE TABLE public.users (
        id uuid PRIMARY KEY,
        name text NOT NULL,
        email text NOT NULL UNIQUE,
        password_hash text NOT NULL,
        created_at timestamptz NOT NULL,
        updated_at timestamptz NOT NULL,
        status public.userstatus NOT NULL,
        role public.userrole NOT NULL
    );
    CREATE INDEX idx_users_email ON public.users USING btree (email);

    CREATE TABLE public.vehicles (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        make varchar(255) NOT NULL,
        model varchar(255) NOT NULL,
        year integer NOT NULL CONSTRAINT vehicles_year_check CHECK (year > 1990),
        license_plate varchar(20) NOT NULL UNIQUE,
        created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
        user_id uuid
    );

    CREATE TABLE public.drivers (
        id uuid PRIMARY KEY,
        name varchar NOT NULL,
        photo varchar NOT NULL,
        rating real NOT NULL,
        license_plate varchar NOT NULL,
        email varchar NOT NULL,
        deliveries integer NOT NULL,
        created_at timestamptz NOT NULL,
        updated_at timestamptz NOT NULL,
        vehicle uuid NOT NULL,
        status public.userstatus NOT NULL,
        phone varchar,
        CONSTRAINT user_id FOREIGN KEY (id) REFERENCES public.users (id) NOT VALID
    );

    CREATE TABLE public.stores (
        id uuid PRIMARY KEY,
        name text NOT NULL,
        description text NOT NULL,
        logo text NOT NULL,
        cover text NOT NULL,
        rating real NOT NULL,
        delivery_time text NOT NULL,
        categories text[] NOT NULL,
        popularity integer NOT NULL,
        created_at timestamptz NOT NULL,
        updated_at timestamptz NOT NULL,
        status public.storestatus NOT NULL,
        theme public.storetheme NOT NULL
    );

    CREATE TABLE public.products (
        id uuid PRIMARY KEY,
        name text NOT NULL,
        description text NOT NULL,
        price double precision NOT NULL,
        image text NOT NULL,
        store_id uuid NOT NULL REFERENCES public.stores (id) ON DELETE CASCADE,
        category text NOT NULL,
        popularity integer NOT NULL,
        created_at timestamptz NOT NULL,
        updated_at timestamptz NOT NULL,
        status public.productstatus NOT NULL,
        subprice double precision
    );
    CREATE INDEX idx_products_store_id ON public.products USING btree (store_id);

    CREATE TABLE public.orders (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        store_id uuid NOT NULL REFERENCES public.stores (id) ON DELETE CASCADE,
        driver_id uuid NOT NULL REFERENCES public.drivers (id),
        total double precision NOT NULL,
        created_at timestamptz NOT NULL,
        updated_at timestamptz NOT NULL,
        estimated_delivery_time text,
        customer_id uuid NOT NULL,
        address varchar NOT NULL,
        latitude double precision,
        longitude double precision,
        phone varchar NOT NULL,
        payment_method varchar,
        subtotal double precision NOT NULL,
        description varchar,
        status public.orderstatus
    );
    CREATE INDEX idx_orders_driver_id ON public.orders USING btree (driver_id);
    CREATE INDEX idx_orders_store_id ON public.orders USING btree (store_id);

    CREATE TABLE public.order_items (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        order_id uuid NOT NULL REFERENCES public.orders (id) ON DELETE CASCADE,
        product_id uuid NOT NULL REFERENCES public.products (id),
        quantity integer NOT NULL,
        price double precision NOT NULL,
        created_at timestamptz NOT NULL,
        sub_price double precision NOT NULL
    );
    CREATE INDEX idx_order_items_order_id ON public.order_items USING btree (order_id);
    CREATE INDEX idx_order_items_product_id ON public.order_items USING btree (product_id);

    CREATE TABLE public.payment_methods (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id uuid NOT NULL REFERENCES public.users (id) ON DELETE CASCADE,
        card_number varchar(16) NOT NULL,
        card_holder_name varchar(255) NOT NULL,
        expiration_date varchar(5) NOT NULL,
        cvv varchar(3) NOT NULL,
        created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

END
$$;

CREATE TABLE IF NOT EXISTS public.addresses (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES public.users (id) ON DELETE CASCADE,
    street varchar(255) NOT NULL,
    city varchar(100) NOT NULL,
    state varchar(100) NOT NULL,
    postal_code varchar(10) NOT NULL,
    country varchar(100) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);

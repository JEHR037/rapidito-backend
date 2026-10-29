ALTER TYPE public.orderstatus RENAME VALUE 'Pending' TO 'PendingPayment';
ALTER TYPE public.orderstatus RENAME VALUE 'Delivering' TO 'OutForDelivery';
ALTER TYPE public.orderstatus RENAME VALUE 'Completed' TO 'Delivered';
ALTER TYPE public.orderstatus ADD VALUE 'Confirmed' AFTER 'Processing';
ALTER TYPE public.orderstatus ADD VALUE 'Preparing' AFTER 'Confirmed';
ALTER TYPE public.orderstatus ADD VALUE 'ReadyForPickup' AFTER 'Preparing';

DROP TABLE IF EXISTS public.payment_methods;

ALTER TABLE public.users
    ADD COLUMN phone text,
    ADD COLUMN avatar text,
    ALTER COLUMN created_at SET DEFAULT CURRENT_TIMESTAMP,
    ALTER COLUMN updated_at SET DEFAULT CURRENT_TIMESTAMP;
CREATE UNIQUE INDEX users_email_lower_key ON public.users (lower(email));

ALTER TABLE public.addresses
    ADD COLUMN label varchar(50) NOT NULL DEFAULT 'Home',
    ADD COLUMN latitude double precision,
    ADD COLUMN longitude double precision,
    ADD COLUMN is_default boolean NOT NULL DEFAULT false;
CREATE INDEX idx_addresses_user_id ON public.addresses (user_id);
CREATE UNIQUE INDEX addresses_one_default_per_user ON public.addresses (user_id) WHERE is_default;

ALTER TABLE public.stores
    ADD COLUMN owner_id uuid,
    ADD COLUMN address text,
    ADD COLUMN latitude double precision,
    ADD COLUMN longitude double precision,
    ADD COLUMN primary_color varchar(9),
    ADD COLUMN secondary_color varchar(9),
    ALTER COLUMN created_at SET DEFAULT CURRENT_TIMESTAMP,
    ALTER COLUMN updated_at SET DEFAULT CURRENT_TIMESTAMP,
    ADD CONSTRAINT stores_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES public.users (id) ON DELETE SET NULL;
CREATE INDEX idx_stores_owner_id ON public.stores (owner_id);

ALTER TABLE public.products
    ALTER COLUMN price TYPE bigint USING round(price * 100)::bigint,
    ALTER COLUMN subprice TYPE bigint USING round(subprice * 100)::bigint,
    ALTER COLUMN created_at SET DEFAULT CURRENT_TIMESTAMP,
    ALTER COLUMN updated_at SET DEFAULT CURRENT_TIMESTAMP;
ALTER TABLE public.products RENAME COLUMN price TO price_cents;
ALTER TABLE public.products RENAME COLUMN subprice TO compare_at_price_cents;
ALTER TABLE public.products
    ADD CONSTRAINT products_price_cents_check CHECK (price_cents >= 0),
    ADD CONSTRAINT products_compare_at_price_cents_check CHECK (compare_at_price_cents IS NULL OR compare_at_price_cents >= 0);

ALTER TABLE public.drivers
    DROP COLUMN name,
    DROP COLUMN email,
    DROP COLUMN license_plate,
    ALTER COLUMN vehicle DROP NOT NULL,
    ALTER COLUMN deliveries SET DEFAULT 0,
    ALTER COLUMN created_at SET DEFAULT CURRENT_TIMESTAMP,
    ALTER COLUMN updated_at SET DEFAULT CURRENT_TIMESTAMP,
    ADD CONSTRAINT drivers_vehicle_fkey FOREIGN KEY (vehicle) REFERENCES public.vehicles (id) ON DELETE SET NULL NOT VALID;

ALTER TABLE public.vehicles
    ADD CONSTRAINT vehicles_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE CASCADE NOT VALID;

UPDATE public.orders SET status = 'PendingPayment' WHERE status IS NULL;
ALTER TABLE public.orders
    ALTER COLUMN total TYPE bigint USING round(total * 100)::bigint,
    ALTER COLUMN subtotal TYPE bigint USING round(subtotal * 100)::bigint,
    ALTER COLUMN driver_id DROP NOT NULL,
    ALTER COLUMN status SET NOT NULL,
    ALTER COLUMN status SET DEFAULT 'PendingPayment',
    ALTER COLUMN payment_method SET DEFAULT 'bank_transfer',
    ALTER COLUMN created_at SET DEFAULT CURRENT_TIMESTAMP,
    ALTER COLUMN updated_at SET DEFAULT CURRENT_TIMESTAMP,
    ADD COLUMN delivery_fee_cents bigint NOT NULL DEFAULT 0,
    ADD COLUMN payment_reference varchar(64),
    ADD COLUMN notes text,
    ADD CONSTRAINT orders_customer_id_fkey FOREIGN KEY (customer_id) REFERENCES public.users (id) NOT VALID;
ALTER TABLE public.orders RENAME COLUMN total TO total_cents;
ALTER TABLE public.orders RENAME COLUMN subtotal TO subtotal_cents;
ALTER TABLE public.orders RENAME COLUMN address TO delivery_address;
ALTER TABLE public.orders RENAME COLUMN latitude TO delivery_latitude;
ALTER TABLE public.orders RENAME COLUMN longitude TO delivery_longitude;
ALTER TABLE public.orders DROP COLUMN description;
CREATE INDEX idx_orders_customer_id ON public.orders (customer_id);
CREATE INDEX idx_orders_status ON public.orders (status);

ALTER TABLE public.order_items
    ALTER COLUMN price TYPE bigint USING round(price * 100)::bigint,
    ALTER COLUMN created_at SET DEFAULT CURRENT_TIMESTAMP,
    DROP COLUMN sub_price,
    ADD COLUMN product_name text NOT NULL DEFAULT '',
    ADD COLUMN position smallint NOT NULL DEFAULT 0,
    ADD CONSTRAINT order_items_quantity_check CHECK (quantity BETWEEN 1 AND 99) NOT VALID;
ALTER TABLE public.order_items RENAME COLUMN price TO unit_price_cents;

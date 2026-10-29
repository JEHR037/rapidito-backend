CREATE TYPE public.paymentmethod AS ENUM ('PagoMovil', 'BankTransfer', 'Binance', 'Paypal');
CREATE TYPE public.ratesource AS ENUM ('Provider', 'Manual');

CREATE TABLE public.payment_settings (
    id smallint PRIMARY KEY DEFAULT 1 CONSTRAINT payment_settings_singleton CHECK (id = 1),
    pago_movil_enabled boolean NOT NULL DEFAULT false,
    pago_movil_bank text,
    pago_movil_phone text,
    pago_movil_document text,
    bank_transfer_enabled boolean NOT NULL DEFAULT false,
    bank_name text,
    bank_account_number text,
    bank_account_holder text,
    bank_account_document text,
    bank_account_type text,
    binance_enabled boolean NOT NULL DEFAULT false,
    binance_pay_id text,
    binance_email text,
    paypal_enabled boolean NOT NULL DEFAULT false,
    paypal_email text,
    paypal_surcharge_bps integer NOT NULL DEFAULT 800 CONSTRAINT payment_settings_surcharge_check CHECK (paypal_surcharge_bps BETWEEN 0 AND 5000),
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by uuid REFERENCES public.users (id) ON DELETE SET NULL
);

INSERT INTO public.payment_settings (id) VALUES (1);

CREATE TABLE public.exchange_rates (
    id bigserial PRIMARY KEY,
    ves_per_usd_micros bigint NOT NULL CONSTRAINT exchange_rates_positive CHECK (ves_per_usd_micros > 0),
    source public.ratesource NOT NULL,
    published_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    checked_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by uuid REFERENCES public.users (id) ON DELETE SET NULL
);
CREATE INDEX idx_exchange_rates_created_at ON public.exchange_rates (created_at DESC);

ALTER TABLE public.orders ALTER COLUMN payment_method DROP DEFAULT;
ALTER TABLE public.orders
    ALTER COLUMN payment_method TYPE public.paymentmethod USING (
        CASE payment_method
            WHEN 'pago_movil' THEN 'PagoMovil'
            WHEN 'binance' THEN 'Binance'
            WHEN 'paypal' THEN 'Paypal'
            ELSE 'BankTransfer'
        END
    )::public.paymentmethod;
UPDATE public.orders SET payment_method = 'BankTransfer' WHERE payment_method IS NULL;
ALTER TABLE public.orders
    ALTER COLUMN payment_method SET NOT NULL,
    ALTER COLUMN payment_method SET DEFAULT 'BankTransfer',
    ADD COLUMN surcharge_cents bigint NOT NULL DEFAULT 0 CONSTRAINT orders_surcharge_check CHECK (surcharge_cents >= 0),
    ADD COLUMN exchange_rate_micros bigint,
    ADD COLUMN total_ves_cents bigint,
    ADD COLUMN payer_name text,
    ADD COLUMN payer_phone text,
    ADD COLUMN payer_bank text,
    ADD COLUMN payer_document text,
    ADD COLUMN payer_account text,
    ADD COLUMN payment_verified_at timestamptz,
    ADD COLUMN payment_verified_by uuid REFERENCES public.users (id) ON DELETE SET NULL,
    ADD COLUMN cancellation_reason text;
CREATE INDEX idx_orders_payment_pending ON public.orders (created_at) WHERE status = 'Processing';

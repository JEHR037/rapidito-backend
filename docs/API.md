# Rapidito API contract

Version 1. This document is the single source of truth shared by `rapidito-backend` and
`rapidito-app`. Any change to a route, payload or enum lands here first.

## Conventions

| Topic          | Rule                                                                     |
| -------------- | ------------------------------------------------------------------------ |
| Base path      | Every route lives under `/api`.                                          |
| Format         | JSON request and response bodies, `camelCase` keys.                      |
| Identifiers    | UUID v4 strings.                                                         |
| Timestamps     | RFC 3339 strings in UTC (`2026-10-29T21:46:00Z`).                        |
| Money          | Integer cents in fields suffixed `Cents` (`priceCents: 599` is 5.99).    |
| Enums          | `snake_case` strings.                                                    |
| Auth           | `Authorization: Bearer <jwt>` on every route marked with a role.         |
| Empty success  | `204 No Content`.                                                        |

### Errors

Every non-2xx response has this body:

```json
{ "error": { "code": "not_found", "message": "Order not found" } }
```

| HTTP | `code`         | Meaning                                             |
| ---- | -------------- | --------------------------------------------------- |
| 400  | `bad_request`  | Validation failed; `message` names the field.       |
| 401  | `unauthorized` | Missing, expired or invalid token, bad credentials. |
| 403  | `forbidden`    | Authenticated but the role or ownership is wrong.   |
| 404  | `not_found`    | Resource does not exist or is not visible.          |
| 409  | `conflict`     | Duplicate email, invalid status transition, etc.    |
| 500  | `internal`     | Unexpected failure; details only in server logs.    |

Unknown routes also answer `404` with this body. Request bodies reject unknown fields with
`400`, so clients cannot smuggle `role`, `status` or prices into a payload.

## Enums

| Enum            | Values                                                                                                         |
| --------------- | -------------------------------------------------------------------------------------------------------------- |
| `UserRole`      | `customer`, `driver`, `partner`, `admin`                                                                       |
| `UserStatus`    | `active`, `inactive`, `deleted`                                                                                |
| `StoreStatus`   | `active`, `inactive`, `deleted`                                                                                |
| `StoreTheme`    | `default`, `custom`                                                                                            |
| `ProductStatus` | `active`, `inactive`, `deleted`, `offers`                                                                      |
| `OrderStatus`   | `pending_payment`, `processing`, `confirmed`, `preparing`, `ready_for_pickup`, `out_for_delivery`, `delivered`, `cancelled` |

## Resources

```ts
type User = {
  id: string;
  name: string;
  email: string;
  role: UserRole;
  status: UserStatus;
  phone: string | null;
  avatar: string | null;
  createdAt: string;
};

type AuthResponse = { token: string; user: User };

type Address = {
  id: string;
  label: string;
  street: string;
  city: string;
  state: string;
  postalCode: string;
  country: string;
  latitude: number | null;
  longitude: number | null;
  isDefault: boolean;
};

type Store = {
  id: string;
  name: string;
  description: string;
  logo: string;
  cover: string;
  theme: StoreTheme;
  primaryColor: string | null;
  secondaryColor: string | null;
  rating: number;
  deliveryTime: string;
  categories: string[];
  popularity: number;
  status: StoreStatus;
  ownerId: string | null;
  address: string | null;
  latitude: number | null;
  longitude: number | null;
};

type StoreDetail = Store & { products: Product[] };

type Product = {
  id: string;
  storeId: string;
  storeName: string;
  name: string;
  description: string;
  priceCents: number;
  compareAtPriceCents: number | null;
  image: string;
  category: string;
  popularity: number;
  status: ProductStatus;
};

type Vehicle = {
  id: string;
  make: string;
  model: string;
  year: number;
  licensePlate: string;
};

type Driver = {
  id: string;
  name: string;
  email: string;
  phone: string | null;
  photo: string;
  rating: number;
  deliveries: number;
  status: UserStatus;
  vehicle: Vehicle | null;
};

type OrderItem = {
  productId: string;
  name: string;
  quantity: number;
  unitPriceCents: number;
  lineTotalCents: number;
};

type OrderStore = {
  id: string;
  name: string;
  address: string | null;
  latitude: number | null;
  longitude: number | null;
};

type OrderDriver = {
  id: string;
  name: string;
  phone: string | null;
  photo: string;
  rating: number;
};

type Order = {
  id: string;
  status: OrderStatus;
  customerId: string;
  customerName: string;
  store: OrderStore;
  driver: OrderDriver | null;
  items: OrderItem[];
  subtotalCents: number;
  deliveryFeeCents: number;
  totalCents: number;
  deliveryAddress: string;
  deliveryLatitude: number | null;
  deliveryLongitude: number | null;
  phone: string;
  paymentMethod: 'bank_transfer';
  paymentReference: string | null;
  notes: string | null;
  estimatedDeliveryTime: string | null;
  createdAt: string;
  updatedAt: string;
};

type AdminStats = {
  users: number;
  customers: number;
  partners: number;
  drivers: number;
  stores: number;
  products: number;
  orders: number;
  activeOrders: number;
  revenueCents: number;
};
```

## Routes

### Health

| Method | Path          | Role | Response             |
| ------ | ------------- | ---- | -------------------- |
| GET    | `/api/health` | none | `{ "status": "ok" }` |

### Auth

| Method | Path                 | Role | Body                          | Response              |
| ------ | -------------------- | ---- | ----------------------------- | --------------------- |
| POST   | `/api/auth/register` | none | `{ name, email, password }`   | `201 AuthResponse`    |
| POST   | `/api/auth/login`    | none | `{ email, password }`         | `200 AuthResponse`    |

Registration always creates an `active` `customer`; roles are granted by an admin. The first
admin is bootstrapped from the database (see the backend README). Email is
trimmed and lowercased. Password: 8 to 128 characters, at least one uppercase, one
lowercase and one digit. Name: 2 to 100 characters after trimming. Login rejects users
whose status is not `active` with `401`.

### Current user

| Method | Path                                    | Role | Body                                             | Response        |
| ------ | --------------------------------------- | ---- | ------------------------------------------------ | --------------- |
| GET    | `/api/users/me`                         | any  |                                                  | `User`          |
| PUT    | `/api/users/me`                         | any  | `{ name?, phone?, avatar? }`                     | `User`          |
| DELETE | `/api/users/me`                         | any  |                                                  | `204`           |
| GET    | `/api/users/me/addresses`               | any  |                                                  | `Address[]`     |
| POST   | `/api/users/me/addresses`               | any  | `{ label, street, city, state, postalCode, country, latitude?, longitude?, isDefault? }` | `201 Address` |
| PUT    | `/api/users/me/addresses/{id}/default`  | any  |                                                  | `Address`       |
| DELETE | `/api/users/me/addresses/{id}`          | any  |                                                  | `204`           |
| GET    | `/api/users/me/driver`                  | driver |                                                | `Driver`        |

`DELETE /api/users/me` is a soft delete: the status becomes `deleted` and login stops
working. The first address a user creates becomes the default one.

### Stores

| Method | Path                | Role          | Query / Body             | Response        |
| ------ | ------------------- | ------------- | ------------------------ | --------------- |
| GET    | `/api/stores`       | none          | `?category=&q=`          | `Store[]`       |
| GET    | `/api/stores/mine`  | partner, admin |                         | `Store[]`       |
| GET    | `/api/stores/{id}`  | none          |                          | `StoreDetail`   |
| POST   | `/api/stores`       | admin         | `StoreInput`             | `201 Store`     |
| PUT    | `/api/stores/{id}`  | admin         | `StoreInput`             | `Store`         |
| DELETE | `/api/stores/{id}`  | admin         |                          | `204`           |

```ts
type StoreInput = {
  name: string;
  description: string;
  logo: string;
  cover: string;
  theme: StoreTheme;
  primaryColor?: string | null;
  secondaryColor?: string | null;
  deliveryTime: string;
  categories: string[];
  status?: StoreStatus;
  ownerId?: string | null;
  address?: string | null;
  latitude?: number | null;
  longitude?: number | null;
};
```

Public listings only return `active` stores ordered by popularity. `StoreDetail.products`
only contains `active` and `offers` products. `GET /api/stores/mine` returns the stores a
partner owns, or every store that is not `deleted` (any status) when the caller is an admin.
`DELETE` is a soft delete. `ownerId` must reference a user with role `partner`; a `custom`
theme requires `primaryColor`; colors are hex strings such as `#1E88E5`.

### Products

| Method | Path                  | Role             | Query / Body                                   | Response       |
| ------ | --------------------- | ---------------- | ---------------------------------------------- | -------------- |
| GET    | `/api/products`       | none             | `?storeId=&category=&q=&limit=`                | `Product[]`    |
| GET    | `/api/products/{id}`  | none             |                                                | `Product`      |
| POST   | `/api/products`       | admin, partner   | `ProductInput`                                 | `201 Product`  |
| PUT    | `/api/products/{id}`  | admin, partner   | `ProductInput`                                 | `Product`      |
| DELETE | `/api/products/{id}`  | admin, partner   |                                                | `204`          |

```ts
type ProductInput = {
  storeId: string;
  name: string;
  description: string;
  priceCents: number;
  compareAtPriceCents?: number | null;
  image: string;
  category: string;
  status?: ProductStatus;
};
```

Listings return `active` and `offers` products of `active` stores, ordered by popularity;
`limit` defaults to 50 and is capped at 200. A partner may only write products of a store
they own. `DELETE` is a soft delete.

### Orders

| Method | Path                       | Role                               | Body                    | Response      |
| ------ | -------------------------- | ---------------------------------- | ----------------------- | ------------- |
| POST   | `/api/orders`              | customer                           | `CreateOrderInput`      | `201 Order`   |
| GET    | `/api/orders`              | any                                | `?status=`              | `Order[]`     |
| GET    | `/api/orders/available`    | driver                             |                         | `Order[]`     |
| GET    | `/api/orders/{id}`         | any, scoped                        |                         | `Order`       |
| PATCH  | `/api/orders/{id}/status`  | any, scoped                        | `{ status }`            | `Order`       |
| POST   | `/api/orders/{id}/assign`  | driver                             |                         | `Order`       |

```ts
type CreateOrderInput = {
  storeId: string;
  items: { productId: string; quantity: number }[];
  deliveryAddress: string;
  deliveryLatitude?: number | null;
  deliveryLongitude?: number | null;
  phone: string;
  paymentReference: string;
  notes?: string | null;
};
```

Prices come from the database, never from the client. Every product must belong to
`storeId` and be `active` or `offers`; quantities go from 1 to 99. Since the payment
reference of the bank transfer is sent with the order, a new order starts in
`processing`. `paymentReference` needs 6 to 64 characters.

Visibility of `GET /api/orders` and `GET /api/orders/{id}`:

| Role     | Sees                                         |
| -------- | -------------------------------------------- |
| customer | Orders they placed.                          |
| partner  | Orders of stores they own.                   |
| driver   | Orders assigned to them.                     |
| admin    | Every order.                                 |

`GET /api/orders/available` lists unassigned orders in `confirmed`, `preparing` or
`ready_for_pickup`. `POST /api/orders/{id}/assign` assigns the calling driver when the order
is in one of those statuses and has no driver yet; otherwise `409`.

Status transitions allowed through `PATCH /api/orders/{id}/status`:

| From               | To                 | Who                    |
| ------------------ | ------------------ | ---------------------- |
| `pending_payment`  | `processing`       | customer               |
| `pending_payment`  | `cancelled`        | customer, partner      |
| `processing`       | `confirmed`        | partner                |
| `processing`       | `cancelled`        | customer, partner      |
| `confirmed`        | `preparing`        | partner                |
| `confirmed`        | `cancelled`        | partner                |
| `preparing`        | `ready_for_pickup` | partner                |
| `ready_for_pickup` | `out_for_delivery` | assigned driver        |
| `out_for_delivery` | `delivered`        | assigned driver        |

An admin may apply any of these transitions. Anything else returns `409`.

### Drivers

| Method | Path                 | Role  | Body                 | Response      |
| ------ | -------------------- | ----- | -------------------- | ------------- |
| GET    | `/api/drivers`       | admin |                      | `Driver[]`    |
| POST   | `/api/drivers`       | admin | `DriverInput`        | `201 Driver`  |
| PUT    | `/api/drivers/{id}`  | admin | `DriverUpdateInput`  | `Driver`      |
| DELETE | `/api/drivers/{id}`  | admin |                      | `204`         |

```ts
type VehicleInput = { make: string; model: string; year: number; licensePlate: string };
type DriverInput = { userId: string; phone: string; photo: string; vehicle: VehicleInput };
type DriverUpdateInput = { phone?: string; photo?: string; status?: UserStatus; vehicle?: VehicleInput };
```

Creating a driver promotes the user to role `driver`; a user can only have one driver
profile (`409` otherwise). Deleting one demotes the user back to `customer`. The profile and
its vehicle are removed when the driver never had orders, and kept as `inactive` for the
order history otherwise. A driver with orders that are not `delivered` or `cancelled`
cannot be deleted (`409`).

### Admin

| Method | Path                     | Role  | Query / Body           | Response      |
| ------ | ------------------------ | ----- | ---------------------- | ------------- |
| GET    | `/api/admin/stats`       | admin |                        | `AdminStats`  |
| GET    | `/api/admin/users`       | admin | `?role=&q=`            | `User[]`      |
| PUT    | `/api/admin/users/{id}`  | admin | `{ role?, status? }`   | `User`        |

An admin cannot change their own role or status (`409`). The `driver` role is only granted
and removed through `/api/drivers`.

`revenueCents` sums `totalCents` of `delivered` orders. `activeOrders` counts orders that
are neither `delivered` nor `cancelled`.

## Out of scope for version 1

- Card payment methods. They were removed because the previous implementation stored full
  card numbers and CVVs. Checkout uses bank transfer with a payment reference.
- Realtime updates. The WebSocket layer arrives in version 2; until then clients poll
  `GET /api/orders/{id}`.

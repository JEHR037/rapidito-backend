import json
import os
import subprocess
import sys
import urllib.error
import urllib.request

BASE = os.environ.get("API_URL", "http://127.0.0.1:8080/api")
DATABASE_URL = os.environ["DATABASE_URL"]
failures = []


def call(method, path, body=None, token=None, expect=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(BASE + path, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", "Bearer " + token)
    try:
        with urllib.request.urlopen(req) as res:
            status, raw = res.status, res.read()
    except urllib.error.HTTPError as err:
        status, raw = err.code, err.read()
    payload = json.loads(raw) if raw else None
    if expect is not None and status != expect:
        failures.append(f"{method} {path}: expected {expect}, got {status} {payload}")
        print("FAIL", method, path, status, payload)
    else:
        print("ok  ", method, path, status)
    return status, payload


def check(cond, msg):
    if not cond:
        failures.append(msg)
        print("FAIL", msg)


def psql(sql):
    subprocess.run(["psql", DATABASE_URL, "-q", "-c", sql], check=True)


def register(name, email):
    _, body = call("POST", "/auth/register", {"name": name, "email": email, "password": "Secret123"}, expect=201)
    return body


call("GET", "/health", expect=200)

call("POST", "/auth/register", {"name": "A", "email": "a@b.co", "password": "Secret123"}, expect=400)
call("POST", "/auth/register", {"name": "Ana", "email": "a@b.co", "password": "weak"}, expect=400)
call("POST", "/auth/register", {"name": "Ana", "email": "a@b.co", "password": "Secret123", "role": "admin"}, expect=400)

admin = register("Admin", "Admin@Rapidito.com")
check(admin["user"]["role"] == "customer", "register must create customers")
check(admin["user"]["email"] == "admin@rapidito.com", "email must be lowercased")
call("POST", "/auth/register", {"name": "Dup", "email": "ADMIN@rapidito.com", "password": "Secret123"}, expect=409)
partner = register("Partner", "partner@rapidito.com")
driver_user = register("Driver", "driver@rapidito.com")
customer = register("Customer", "customer@rapidito.com")
other = register("Other", "other@rapidito.com")
psql("UPDATE users SET role = 'Admin' WHERE email = 'admin@rapidito.com'")

call("POST", "/auth/login", {"email": "admin@rapidito.com", "password": "Wrong123"}, expect=401)
call("POST", "/auth/login", {"email": "nobody@rapidito.com", "password": "Wrong123"}, expect=401)
_, login = call("POST", "/auth/login", {"email": "ADMIN@rapidito.com", "password": "Secret123"}, expect=200)
A = login["token"]
check(login["user"]["role"] == "admin", "promoted admin role")
P = partner["token"]
D = driver_user["token"]
C = customer["token"]
O = other["token"]

call("GET", "/users/me", expect=401)
call("GET", "/users/me", token="garbage", expect=401)
_, me = call("PUT", "/users/me", {"name": "Customer Uno", "phone": "+58 412-1234567"}, token=C, expect=200)
check(me["phone"] == "+58 412-1234567" and me["name"] == "Customer Uno", "profile update")
_, me = call("PUT", "/users/me", {"phone": None}, token=C, expect=200)
check(me["phone"] is None and me["name"] == "Customer Uno", "phone cleared, name kept")
call("PUT", "/users/me", {"phone": "+58 412-1234567"}, token=C, expect=200)

_, a1 = call("POST", "/users/me/addresses", {"label": "Casa", "street": "Av. Principal 123", "city": "Caracas",
                                             "state": "DC", "postalCode": "1010", "country": "VE",
                                             "latitude": 10.49, "longitude": -66.85}, token=C, expect=201)
check(a1["isDefault"], "first address default")
_, a2 = call("POST", "/users/me/addresses", {"label": "Trabajo", "street": "Calle 2", "city": "Caracas",
                                             "state": "DC", "postalCode": "1020", "country": "VE"}, token=C, expect=201)
check(not a2["isDefault"], "second address not default")
call("PUT", f"/users/me/addresses/{a2['id']}/default", token=C, expect=200)
_, addrs = call("GET", "/users/me/addresses", token=C, expect=200)
check([a["isDefault"] for a in addrs] == [True, False] and addrs[0]["id"] == a2["id"], "default switched")
call("DELETE", f"/users/me/addresses/{a2['id']}", token=O, expect=404)
call("DELETE", f"/users/me/addresses/{a2['id']}", token=C, expect=204)
_, addrs = call("GET", "/users/me/addresses", token=C)
check(len(addrs) == 1 and addrs[0]["isDefault"], "default reassigned after delete")

store_input = {"name": "Burger Place", "description": "Burgers", "logo": "https://x/l.png", "cover": "https://x/c.png",
               "theme": "custom", "primaryColor": "#ffc72c", "secondaryColor": "#DA291C", "deliveryTime": "20-30 min",
               "categories": ["Hamburguesas", "Hamburguesas", "Combos"], "ownerId": partner["user"]["id"],
               "address": "Av. Francisco de Miranda", "latitude": 10.491, "longitude": -66.849}
call("POST", "/stores", store_input, token=P, expect=403)
call("POST", "/stores", {**store_input, "ownerId": customer["user"]["id"]}, token=A, expect=400)
call("POST", "/stores", {**store_input, "primaryColor": None}, token=A, expect=400)
call("POST", "/stores", store_input, token=A, expect=400)
psql("UPDATE users SET role = 'Partner' WHERE email = 'partner@rapidito.com'")
_, store = call("POST", "/stores", store_input, token=A, expect=201)
check(store["categories"] == ["Hamburguesas", "Combos"], "categories deduplicated")
check(store["primaryColor"] == "#FFC72C", "color normalized")
_, other_store = call("POST", "/stores", {**store_input, "name": "Pizza Place", "theme": "default", "primaryColor": None,
                                         "ownerId": None, "categories": ["Pizza"]}, token=A, expect=201)

_, prod = call("POST", "/products", {"storeId": store["id"], "name": "Big Burger", "description": "Tasty",
                                     "priceCents": 599, "compareAtPriceCents": 699, "image": "https://x/b.png",
                                     "category": "Hamburguesas"}, token=P, expect=201)
_, prod2 = call("POST", "/products", {"storeId": store["id"], "name": "Fries", "description": "Crispy",
                                      "priceCents": 250, "image": "https://x/f.png", "category": "Combos",
                                      "status": "offers"}, token=A, expect=201)
_, pizza = call("POST", "/products", {"storeId": other_store["id"], "name": "Pizza", "description": "Cheese",
                                      "priceCents": 900, "image": "https://x/p.png", "category": "Pizza"}, token=A,
                expect=201)
call("POST", "/products", {"storeId": other_store["id"], "name": "Hack", "description": "xx", "priceCents": 1,
                           "image": "i", "category": "c"}, token=P, expect=403)
call("POST", "/products", {"storeId": store["id"], "name": "Neg", "description": "xx", "priceCents": -1,
                           "image": "i", "category": "c"}, token=P, expect=400)
_, hidden = call("POST", "/products", {"storeId": store["id"], "name": "Hidden", "description": "xx", "priceCents": 100,
                                       "image": "i", "category": "c", "status": "inactive"}, token=P, expect=201)

_, stores = call("GET", "/stores", expect=200)
check(len(stores) == 2, "two public stores")
_, filtered = call("GET", "/stores?category=Pizza", expect=200)
check([s["name"] for s in filtered] == ["Pizza Place"], "category filter")
_, mine = call("GET", "/stores/mine", token=P, expect=200)
check([s["id"] for s in mine] == [store["id"]], "partner stores")
call("GET", "/stores/mine", token=C, expect=403)
_, detail = call("GET", f"/stores/{store['id']}", expect=200)
check(sorted(p["name"] for p in detail["products"]) == ["Big Burger", "Fries"], "store detail hides inactive")
call("GET", "/stores/not-a-uuid", expect=400)
_, products = call("GET", f"/products?storeId={store['id']}&limit=1", expect=200)
check(len(products) == 1, "product limit")
_, products = call("GET", "/products?q=pizza", expect=200)
check([p["name"] for p in products] == ["Pizza"], "product search")
call("GET", f"/products/{hidden['id']}", expect=404)

settings_input = {"pagoMovilEnabled": True, "pagoMovilBank": "0102 Banco de Venezuela",
                  "pagoMovilPhone": "+58 412-0000000", "pagoMovilDocument": "J-12345678-9",
                  "bankTransferEnabled": False, "bankName": None, "bankAccountNumber": None,
                  "bankAccountHolder": None, "bankAccountDocument": None, "bankAccountType": None,
                  "binanceEnabled": True, "binancePayId": "123456789", "binanceEmail": None,
                  "paypalEnabled": True, "paypalEmail": "Pagos@Rapidito.app", "paypalSurchargeBps": 800}
call("PUT", "/admin/payment-settings", settings_input, token=P, expect=403)
call("PUT", "/admin/payment-settings", {**settings_input, "bankTransferEnabled": True}, token=A, expect=400)
call("PUT", "/admin/payment-settings", {**settings_input, "paypalSurchargeBps": 9000}, token=A, expect=400)
_, saved = call("PUT", "/admin/payment-settings", settings_input, token=A, expect=200)
check(saved["paypalEmail"] == "pagos@rapidito.app", "settings normalized")
_, options = call("GET", "/payments/methods", token=C, expect=200)
check([o["method"] for o in options] == ["pago_movil", "binance", "paypal"], "enabled methods listed")
check(next(o for o in options if o["method"] == "paypal")["surchargeBps"] == 800, "paypal surcharge exposed")
check(next(o for o in options if o["method"] == "pago_movil")["phone"] == "+58 412-0000000", "pago movil instructions")

cart = {"storeId": store["id"], "items": [{"productId": prod["id"], "quantity": 2},
                                          {"productId": prod2["id"], "quantity": 1}]}
_, quote = call("POST", "/orders/quote", cart, token=C, expect=200)
by_method = {m["method"]: m for m in quote["methods"]}
check(not quote["exchangeRateFresh"] and by_method["pago_movil"]["unavailableReason"] == "rate_unavailable", "no rate yet")
check(by_method["bank_transfer"]["unavailableReason"] == "disabled", "disabled method")
call("POST", "/admin/exchange-rate", {"vesPerUsd": "abc"}, token=A, expect=400)
call("POST", "/admin/exchange-rate", {"vesPerUsd": "36.5"}, token=C, expect=403)
_, rate = call("POST", "/admin/exchange-rate", {"vesPerUsd": "36,5"}, token=A, expect=201)
check(rate["fresh"] and rate["rate"]["vesPerUsdMicros"] == 36500000 and rate["rate"]["source"] == "manual", "manual rate")
_, public_rate = call("GET", "/exchange-rate", expect=200)
check(public_rate["rate"]["vesPerUsdMicros"] == 36500000, "public rate")
_, quote = call("POST", "/orders/quote", cart, token=C, expect=200)
by_method = {m["method"]: m for m in quote["methods"]}
base = 599 * 2 + 250 + 150
check(by_method["pago_movil"]["available"] and by_method["pago_movil"]["totalVesCents"] == round(base * 36.5), "ves quote")
check(by_method["paypal"]["surchargeCents"] == round(base * 0.08) and by_method["paypal"]["totalCents"] == base + round(base * 0.08), "paypal quote")
check(by_method["binance"]["totalCents"] == base and by_method["binance"]["currency"] == "USD", "binance quote")

order_input = {**cart, "deliveryAddress": "Av. Principal 123, Caracas", "deliveryLatitude": 10.49,
               "deliveryLongitude": -66.85, "phone": "+58 412-1234567", "paymentMethod": "pago_movil",
               "paymentReference": "REF123456", "payerPhone": "+58 414-5550000", "payerBank": "0105 Mercantil",
               "payerDocument": "V-12345678"}
call("POST", "/orders", {**order_input, "payerDocument": None}, token=C, expect=400)
call("POST", "/orders", {**order_input, "paymentMethod": "bank_transfer"}, token=C, expect=409)
call("POST", "/orders", {**order_input, "paymentMethod": "zelle"}, token=C, expect=400)
call("POST", "/orders", order_input, token=P, expect=403)
call("POST", "/orders", {**order_input, "items": [{"productId": pizza["id"], "quantity": 1}]}, token=C, expect=400)
call("POST", "/orders", {**order_input, "items": [{"productId": hidden["id"], "quantity": 1}]}, token=C, expect=400)
call("POST", "/orders", {**order_input, "items": [{"productId": prod["id"], "quantity": 0}]}, token=C, expect=400)
call("POST", "/orders", {**order_input, "paymentReference": "12"}, token=C, expect=400)
call("POST", "/orders", {**order_input, "items": [{"productId": prod["id"], "quantity": 1, "priceCents": 1}]},
     token=C, expect=400)
_, order = call("POST", "/orders", order_input, token=C, expect=201)
check(order["status"] == "processing", "new order processing")
check(order["subtotalCents"] == 599 * 2 + 250, "server-side subtotal")
check(order["deliveryFeeCents"] == 150 and order["totalCents"] == 599 * 2 + 250 + 150, "fee and total")
check(order["paymentMethod"] == "pago_movil" and order["surchargeCents"] == 0, "payment method stored")
check(order["exchangeRateMicros"] == 36500000 and order["totalVesCents"] == round(base * 36.5), "ves snapshot")
check(order["payer"]["document"] == "V-12345678" and order["paymentVerifiedAt"] is None, "payer stored")
check(order["items"][0]["lineTotalCents"] == 1198, "line total")
check(order["store"]["latitude"] == 10.491 and order["customerName"] == "Customer Uno", "order store/customer")
check(order["estimatedDeliveryTime"] == "20-30 min", "eta from store")
oid = order["id"]

call("GET", f"/orders/{oid}", token=O, expect=404)
call("GET", f"/orders/{oid}", token=D, expect=404)
_, lst = call("GET", "/orders", token=P, expect=200)
check([o["id"] for o in lst] == [oid], "partner sees store orders")
_, lst = call("GET", "/orders", token=O, expect=200)
check(lst == [], "other customer sees nothing")
_, lst = call("GET", "/orders?status=processing", token=A, expect=200)
check(len(lst) == 1, "admin status filter")
call("GET", "/orders?status=bogus", token=A, expect=400)

call("PATCH", f"/orders/{oid}/status", {"status": "confirmed"}, token=C, expect=403)
call("PATCH", f"/orders/{oid}/status", {"status": "delivered"}, token=P, expect=409)
call("PATCH", f"/orders/{oid}/status", {"status": "confirmed"}, token=P, expect=403)
call("PATCH", f"/orders/{oid}/status", {"status": "confirmed", "reason": "x"}, token=A, expect=400)
_, verified = call("PATCH", f"/orders/{oid}/status", {"status": "confirmed"}, token=A, expect=200)
check(verified["paymentVerifiedAt"] is not None, "payment verified timestamp")
call("PATCH", f"/orders/{oid}/status", {"status": "cancelled"}, token=C, expect=403)

call("GET", "/orders/available", token=D, expect=403)
call("POST", "/drivers", {"userId": partner["user"]["id"], "phone": "+58 414-0000000", "photo": "https://x/d.png",
                          "vehicle": {"make": "Honda", "model": "CG", "year": 2020, "licensePlate": "AB1CD"}},
     token=A, expect=409)
_, drv = call("POST", "/drivers", {"userId": driver_user["user"]["id"], "phone": "+58 414-0000000",
                                   "photo": "https://x/d.png",
                                   "vehicle": {"make": "Honda", "model": "CG", "year": 2020, "licensePlate": "AB1CD"}},
              token=A, expect=201)
check(drv["vehicle"]["licensePlate"] == "AB1CD" and drv["name"] == "Driver", "driver created")
call("POST", "/drivers", {"userId": driver_user["user"]["id"], "phone": "+58 414-0000000", "photo": "p",
                          "vehicle": {"make": "H", "model": "C", "year": 2020, "licensePlate": "ZZ9"}},
     token=A, expect=409)
_, drvs = call("GET", "/drivers", token=A, expect=200)
check(len(drvs) == 1, "driver list")
_, profile = call("GET", "/users/me/driver", token=D, expect=200)
check(profile["deliveries"] == 0, "driver profile")

_, avail = call("GET", "/orders/available", token=D, expect=200)
check([o["id"] for o in avail] == [oid], "available orders")
call("PATCH", f"/orders/{oid}/status", {"status": "preparing"}, token=P, expect=200)
call("POST", f"/orders/{oid}/assign", token=D, expect=200)
call("POST", f"/orders/{oid}/assign", token=D, expect=409)
_, avail = call("GET", "/orders/available", token=D, expect=200)
check(avail == [], "assigned order not available")
call("DELETE", f"/drivers/{drv['id']}", token=A, expect=409)
call("PATCH", f"/orders/{oid}/status", {"status": "out_for_delivery"}, token=D, expect=409)
call("PATCH", f"/orders/{oid}/status", {"status": "ready_for_pickup"}, token=P, expect=200)
call("PATCH", f"/orders/{oid}/status", {"status": "out_for_delivery"}, token=P, expect=403)
call("PATCH", f"/orders/{oid}/status", {"status": "out_for_delivery"}, token=D, expect=200)
_, tracked = call("GET", f"/orders/{oid}", token=C, expect=200)
check(tracked["driver"] and tracked["driver"]["name"] == "Driver", "customer sees driver")
call("PATCH", f"/orders/{oid}/status", {"status": "delivered"}, token=D, expect=200)
_, profile = call("GET", "/users/me/driver", token=D, expect=200)
check(profile["deliveries"] == 1, "delivery counted")

_, stats = call("GET", "/admin/stats", token=A, expect=200)
check(stats["revenueCents"] == order["totalCents"] and stats["orders"] == 1 and stats["activeOrders"] == 0, "stats")
check(stats["drivers"] == 1 and stats["partners"] == 1 and stats["stores"] == 2, "stats counts")
call("GET", "/admin/stats", token=P, expect=403)
_, users = call("GET", "/admin/users?role=customer", token=A, expect=200)
check(sorted(u["email"] for u in users) == ["customer@rapidito.com", "other@rapidito.com"], "admin user filter")
call("PUT", f"/admin/users/{admin['user']['id']}", {"status": "inactive"}, token=A, expect=409)
call("PUT", f"/admin/users/{other['user']['id']}", {"role": "driver"}, token=A, expect=400)
call("PUT", f"/admin/users/{other['user']['id']}", {"status": "inactive"}, token=A, expect=200)
call("GET", "/users/me", token=O, expect=401)
call("POST", "/auth/login", {"email": "other@rapidito.com", "password": "Secret123"}, expect=401)

_, paypal_order = call("POST", "/orders", {**cart, "deliveryAddress": "Av. Principal 123, Caracas",
                                          "phone": "+58 412-1234567", "paymentMethod": "paypal",
                                          "paymentReference": "PAYPAL-TX-1", "payerAccount": "buyer@mail.com"},
                       token=C, expect=201)
check(paypal_order["surchargeCents"] == round(base * 0.08) and paypal_order["totalVesCents"] is None, "paypal order")
_, rejected = call("PATCH", f"/orders/{paypal_order['id']}/status",
                   {"status": "cancelled", "reason": "Pago no recibido"}, token=A, expect=200)
check(rejected["cancellationReason"] == "Pago no recibido", "rejection reason stored")

call("DELETE", f"/drivers/{drv['id']}", token=A, expect=204)
_, me = call("GET", "/users/me", token=D, expect=200)
check(me["role"] == "customer", "driver demoted")
_, drvs = call("GET", "/drivers", token=A, expect=200)
check(len(drvs) == 1 and drvs[0]["status"] == "inactive", "driver kept inactive for history")

call("DELETE", f"/products/{prod2['id']}", token=P, expect=204)
call("DELETE", f"/stores/{other_store['id']}", token=A, expect=204)
call("GET", f"/stores/{other_store['id']}", expect=404)
call("DELETE", "/users/me", token=C, expect=204)
call("GET", "/users/me", token=C, expect=401)

print()
print("FAILURES:", len(failures))
for failure in failures:
    print(" -", failure)
sys.exit(1 if failures else 0)

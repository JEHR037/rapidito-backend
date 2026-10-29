use std::collections::HashMap;

use uuid::Uuid;

use crate::domain::ProductStatus;

pub const MAX_QUANTITY: i32 = 99;
pub const MAX_LINES: usize = 50;

#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub id: Uuid,
    pub store_id: Uuid,
    pub name: String,
    pub price_cents: i64,
    pub status: ProductStatus,
}

#[derive(Debug, Clone, Copy)]
pub struct RequestedLine {
    pub product_id: Uuid,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricedLine {
    pub product_id: Uuid,
    pub name: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
}

impl PricedLine {
    pub fn line_total_cents(&self) -> i64 {
        self.unit_price_cents * i64::from(self.quantity)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PricingError {
    Empty,
    TooManyLines,
    InvalidQuantity(Uuid),
    UnknownProduct(Uuid),
    WrongStore(Uuid),
    Unavailable(Uuid),
}

impl std::fmt::Display for PricingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "items must not be empty"),
            Self::TooManyLines => write!(f, "items must contain at most {MAX_LINES} products"),
            Self::InvalidQuantity(id) => write!(f, "quantity for product {id} must be between 1 and {MAX_QUANTITY}"),
            Self::UnknownProduct(id) => write!(f, "product {id} does not exist"),
            Self::WrongStore(id) => write!(f, "product {id} does not belong to the store"),
            Self::Unavailable(id) => write!(f, "product {id} is not available"),
        }
    }
}

pub fn price_lines(
    store_id: Uuid,
    requested: &[RequestedLine],
    catalog: &[CatalogEntry],
) -> Result<Vec<PricedLine>, PricingError> {
    if requested.is_empty() {
        return Err(PricingError::Empty);
    }

    let mut merged: Vec<RequestedLine> = Vec::new();
    for line in requested {
        if !(1..=MAX_QUANTITY).contains(&line.quantity) {
            return Err(PricingError::InvalidQuantity(line.product_id));
        }
        match merged.iter_mut().find(|existing| existing.product_id == line.product_id) {
            Some(existing) => {
                existing.quantity += line.quantity;
                if existing.quantity > MAX_QUANTITY {
                    return Err(PricingError::InvalidQuantity(line.product_id));
                }
            }
            None => merged.push(*line),
        }
    }
    if merged.len() > MAX_LINES {
        return Err(PricingError::TooManyLines);
    }

    let by_id: HashMap<Uuid, &CatalogEntry> = catalog.iter().map(|entry| (entry.id, entry)).collect();
    merged
        .into_iter()
        .map(|line| {
            let entry = by_id.get(&line.product_id).ok_or(PricingError::UnknownProduct(line.product_id))?;
            if entry.store_id != store_id {
                return Err(PricingError::WrongStore(line.product_id));
            }
            if !entry.status.is_purchasable() {
                return Err(PricingError::Unavailable(line.product_id));
            }
            Ok(PricedLine {
                product_id: entry.id,
                name: entry.name.clone(),
                quantity: line.quantity,
                unit_price_cents: entry.price_cents,
            })
        })
        .collect()
}

pub fn subtotal_cents(lines: &[PricedLine]) -> i64 {
    lines.iter().map(PricedLine::line_total_cents).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(store_id: Uuid, price_cents: i64, status: ProductStatus) -> CatalogEntry {
        CatalogEntry { id: Uuid::new_v4(), store_id, name: "Item".to_string(), price_cents, status }
    }

    #[test]
    fn prices_from_catalog_and_merges_duplicates() {
        let store = Uuid::new_v4();
        let burger = entry(store, 599, ProductStatus::Active);
        let fries = entry(store, 250, ProductStatus::Offers);
        let lines = price_lines(
            store,
            &[
                RequestedLine { product_id: burger.id, quantity: 1 },
                RequestedLine { product_id: fries.id, quantity: 2 },
                RequestedLine { product_id: burger.id, quantity: 1 },
            ],
            &[burger.clone(), fries.clone()],
        )
        .unwrap_or_default();
        assert_eq!(lines.len(), 2);
        assert_eq!(subtotal_cents(&lines), 599 * 2 + 250 * 2);
    }

    #[test]
    fn rejects_foreign_unavailable_and_unknown_products() {
        let store = Uuid::new_v4();
        let foreign = entry(Uuid::new_v4(), 100, ProductStatus::Active);
        let hidden = entry(store, 100, ProductStatus::Inactive);
        let catalog = [foreign.clone(), hidden.clone()];
        let unknown = Uuid::new_v4();

        assert_eq!(
            price_lines(store, &[RequestedLine { product_id: foreign.id, quantity: 1 }], &catalog),
            Err(PricingError::WrongStore(foreign.id))
        );
        assert_eq!(
            price_lines(store, &[RequestedLine { product_id: hidden.id, quantity: 1 }], &catalog),
            Err(PricingError::Unavailable(hidden.id))
        );
        assert_eq!(
            price_lines(store, &[RequestedLine { product_id: unknown, quantity: 1 }], &catalog),
            Err(PricingError::UnknownProduct(unknown))
        );
    }

    #[test]
    fn rejects_bad_quantities() {
        let store = Uuid::new_v4();
        let item = entry(store, 100, ProductStatus::Active);
        let catalog = [item.clone()];
        assert_eq!(price_lines(store, &[], &catalog), Err(PricingError::Empty));
        assert_eq!(
            price_lines(store, &[RequestedLine { product_id: item.id, quantity: 0 }], &catalog),
            Err(PricingError::InvalidQuantity(item.id))
        );
        assert_eq!(
            price_lines(
                store,
                &[
                    RequestedLine { product_id: item.id, quantity: 60 },
                    RequestedLine { product_id: item.id, quantity: 60 },
                ],
                &catalog
            ),
            Err(PricingError::InvalidQuantity(item.id))
        );
    }
}

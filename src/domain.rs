use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "userrole")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Partner,
    Driver,
    Customer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "userstatus")]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Inactive,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "storestatus")]
#[serde(rename_all = "snake_case")]
pub enum StoreStatus {
    Active,
    Inactive,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "storetheme")]
#[serde(rename_all = "snake_case")]
pub enum StoreTheme {
    Default,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "productstatus")]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    Active,
    Inactive,
    Deleted,
    Offers,
}

impl ProductStatus {
    pub fn is_purchasable(self) -> bool {
        matches!(self, Self::Active | Self::Offers)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "orderstatus")]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    PendingPayment,
    Processing,
    Confirmed,
    Preparing,
    ReadyForPickup,
    OutForDelivery,
    Delivered,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderActor {
    Customer,
    StoreOwner,
    AssignedDriver,
    Admin,
}

impl OrderStatus {
    pub const ASSIGNABLE: [OrderStatus; 3] = [Self::Confirmed, Self::Preparing, Self::ReadyForPickup];

    fn transition_actors(self, next: OrderStatus) -> &'static [OrderActor] {
        use OrderActor::{AssignedDriver, Customer, StoreOwner};
        use OrderStatus::*;
        match (self, next) {
            (PendingPayment, Processing) => &[Customer],
            (PendingPayment, Cancelled) => &[Customer, StoreOwner],
            (Processing, Confirmed) => &[StoreOwner],
            (Processing, Cancelled) => &[Customer, StoreOwner],
            (Confirmed, Preparing) => &[StoreOwner],
            (Confirmed, Cancelled) => &[StoreOwner],
            (Preparing, ReadyForPickup) => &[StoreOwner],
            (ReadyForPickup, OutForDelivery) => &[AssignedDriver],
            (OutForDelivery, Delivered) => &[AssignedDriver],
            _ => &[],
        }
    }

    pub fn is_valid_transition(self, next: OrderStatus) -> bool {
        !self.transition_actors(next).is_empty()
    }

    pub fn can_transition(self, next: OrderStatus, actors: &[OrderActor]) -> bool {
        let allowed = self.transition_actors(next);
        if allowed.is_empty() {
            return false;
        }
        actors.contains(&OrderActor::Admin) || actors.iter().any(|actor| allowed.contains(actor))
    }
}

#[cfg(test)]
mod tests {
    use super::OrderActor::*;
    use super::OrderStatus::*;

    #[test]
    fn store_owner_drives_kitchen_flow() {
        assert!(Processing.can_transition(Confirmed, &[StoreOwner]));
        assert!(Confirmed.can_transition(Preparing, &[StoreOwner]));
        assert!(Preparing.can_transition(ReadyForPickup, &[StoreOwner]));
        assert!(!ReadyForPickup.can_transition(OutForDelivery, &[StoreOwner]));
    }

    #[test]
    fn driver_only_moves_delivery_flow() {
        assert!(ReadyForPickup.can_transition(OutForDelivery, &[AssignedDriver]));
        assert!(OutForDelivery.can_transition(Delivered, &[AssignedDriver]));
        assert!(!Processing.can_transition(Confirmed, &[AssignedDriver]));
    }

    #[test]
    fn customer_can_only_cancel_before_confirmation() {
        assert!(Processing.can_transition(Cancelled, &[Customer]));
        assert!(PendingPayment.can_transition(Cancelled, &[Customer]));
        assert!(!Confirmed.can_transition(Cancelled, &[Customer]));
    }

    #[test]
    fn admin_is_bound_to_the_graph() {
        assert!(Processing.can_transition(Confirmed, &[Admin]));
        assert!(!Delivered.can_transition(Processing, &[Admin]));
        assert!(!Cancelled.can_transition(Delivered, &[Admin]));
    }

    #[test]
    fn terminal_states_have_no_exit() {
        for next in [PendingPayment, Processing, Confirmed, Preparing, ReadyForPickup, OutForDelivery, Delivered, Cancelled] {
            assert!(!Delivered.is_valid_transition(next));
            assert!(!Cancelled.is_valid_transition(next));
        }
    }
}

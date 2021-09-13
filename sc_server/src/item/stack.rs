use super::Type;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stack {
  item:   Type,
  amount: u8,
}

impl Stack {
  /// Creates an empty item stack. This has the type set to air, and the count
  /// set to 0.
  pub fn empty() -> Self {
    Stack { item: Type::Air, amount: 0 }
  }
  /// Creates an item stack containing a single item with the given type.
  pub fn new(item: Type) -> Self {
    Stack { item, amount: 1 }
  }

  /// Sets the amount in self, and returns the modified self.
  pub fn with_amount(mut self, amount: u8) -> Self {
    self.amount = amount;
    self
  }
  /// Sets the amount in the item stack.
  pub fn set_amount(&mut self, amount: u8) {
    self.amount = amount;
  }

  /// Returns the number of items in this item stack.
  pub fn amount(&self) -> u8 {
    self.amount
  }
  /// Returns the item that is in this item stack.
  pub fn item(&self) -> Type {
    self.item
  }

  /// Returns true if this item stack is considered "empty". This is true
  /// whenever the type is Air, or whenever the amount is 0.
  pub fn is_empty(&self) -> bool {
    self.item == Type::Air || self.amount == 0
  }
}

use super::Type;
use bb_common::{
  nbt::{Tag, NBT},
  util::Item,
};
use bb_transfer::{
  MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, StructRead, StructReader,
  WriteError,
};
use std::{collections::HashMap, num::NonZeroU8};

#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
  item:   Type,
  amount: NonZeroU8,
  nbt:    NBT,
}

impl From<Item> for Stack {
  /// Creates an item stack from the given item. This is how we convert protocol
  /// items into server storage.
  fn from(it: Item) -> Self {
    Stack::new(Type::from_u32(it.id() as u32)).with_amount(it.count()).with_nbt(it.nbt)
  }
}

// This is required for `Stack::empty` to be `const`.
//
// SAFETY: The value must not be zero, so using `1` is safe.
const ONE: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(1) };

impl Default for Stack {
  fn default() -> Self { Self::empty() }
}

impl MessageRead<'_> for Stack {
  fn read(r: &mut MessageReader) -> Result<Self, ReadError> { r.read_struct() }
}
impl StructRead<'_> for Stack {
  fn read_struct(mut r: StructReader) -> Result<Self, ReadError> {
    Ok(Stack::new(Type::from_u32(r.read(0)?)).with_amount(r.read(1)?))
  }
}
impl MessageWrite for Stack {
  fn write<W: std::io::Write>(&self, w: &mut MessageWriter<W>) -> Result<(), WriteError> {
    w.write_struct(2, |w| {
      w.write(&self.item.id())?;
      w.write(&self.amount())
    })
  }
}

impl Stack {
  /// The empty stack. Useful for array initializers. This is the same as
  /// [`Stack::empty`].
  pub const EMPTY: Stack = Stack::empty();
  /// Creates an empty item stck. This has the type set to air, and the count
  /// set to 0.
  pub const fn empty() -> Self { Stack { item: Type::Air, amount: ONE, nbt: NBT::empty() } }
  /// Creates an item stack containing a single item with the given type.
  pub fn new(item: Type) -> Self { Stack { item, amount: ONE, nbt: NBT::empty() } }

  /// Sets the amount in self, and returns the modified self. If the stack is
  /// air, this will do nothing.
  pub fn with_amount(mut self, amount: u8) -> Self {
    self.set_amount(amount);
    self
  }
  /// Sets the amount in the item stack. If the stack is air, this will do
  /// nothing.
  pub fn set_amount(&mut self, amount: u8) {
    if amount == 0 {
      self.item = Type::Air;
      self.amount = ONE;
      // Keep amount at 1 if we are air.
    } else if self.item != Type::Air {
      self.amount = NonZeroU8::new(amount).unwrap();
    }
  }

  /// Returns the number of items in this item stack.
  pub fn amount(&self) -> u8 {
    if self.item == Type::Air {
      0
    } else {
      self.amount.get()
    }
  }
  /// Returns the item that is in this item stack.
  pub fn item(&self) -> Type { self.item }

  /// Returns true if this item stack is considered "empty". This is true
  /// whenever the type is Air, or the count is zero.
  pub fn is_empty(&self) -> bool { self.item == Type::Air }

  /// Returns a reference to the NBT for this item.
  pub fn nbt(&self) -> &NBT { &self.nbt }
  /// Returns a mutable reference to the NBT for this item.
  pub fn nbt_mut(&mut self) -> &mut NBT {
    if self.nbt.tag() == &Tag::End {
      self.nbt = NBT::new("", Tag::Compound(HashMap::new()));
    }
    &mut self.nbt
  }
  pub fn set_nbt(&mut self, nbt: NBT) { self.nbt = nbt; }
  pub fn with_nbt(mut self, nbt: NBT) -> Self {
    self.set_nbt(nbt);
    self
  }

  pub fn to_item(&self) -> Item {
    Item {
      id:     self.item().id() as i32,
      count:  self.amount(),
      damage: 0,
      nbt:    self.nbt.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_air() {
    assert_eq!(Stack::empty(), Stack::new(Type::Air));
    assert_eq!(Stack::empty(), Stack::new(Type::Air).with_amount(10));
    assert_eq!(Stack::new(Type::Air), Stack::new(Type::Air).with_amount(10));
    assert_eq!(Stack::new(Type::Air).with_amount(10).amount(), 0);
    assert_eq!(Stack::new(Type::Stone).with_amount(0), Stack::empty());
    assert_eq!(Stack::new(Type::Stone).with_amount(0).item(), Type::Air);
    assert_eq!(Stack::new(Type::Stone).with_amount(0).amount(), 0);
  }

  #[test]
  fn test_is_empty() {
    assert!(Stack::empty().is_empty());
    assert!(Stack::new(Type::Air).is_empty());
    assert!(!Stack::new(Type::Stone).is_empty());
    assert!(Stack::new(Type::Stone).with_amount(0).is_empty());
  }

  #[test]
  fn test_item_convert() {
    fn item_eq(stack: Stack, item: Item) {
      assert_eq!(stack.to_item(), item);
      assert_eq!(Stack::from(item), stack);
    }
    item_eq(Stack::empty(), Item { id: 0, count: 0, damage: 0, nbt: NBT::empty("") });
    item_eq(
      Stack::new(Type::Air).with_amount(10),
      Item { id: 0, count: 0, damage: 0, nbt: NBT::empty("") },
    );
    item_eq(Stack::new(Type::Stone), Item { id: 1, count: 1, damage: 0, nbt: NBT::empty("") });
  }
}

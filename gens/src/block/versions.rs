use super::BlockVersion;
use std::collections::HashMap;

pub(super) fn generate(latest: &BlockVersion, old: &BlockVersion) -> (Vec<u32>, HashMap<u32, u32>) {
  let to_old = vec![];
  let to_new = HashMap::new();

  (to_old, to_new)
}

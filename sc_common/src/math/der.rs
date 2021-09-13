use rsa::{PublicKeyParts, RSAPublicKey};

/// Represents an ASN.1 `BIT STRING`. Need this because the constructor is
/// private in the asn1 crate.
#[derive(Debug, PartialEq, Clone)]
pub struct BitString<'a> {
  data:    &'a [u8],
  padding: u8,
}

impl<'a> BitString<'a> {
  pub fn new(data: &'a [u8], padding: u8) -> Option<BitString<'a>> {
    if padding > 7 || (data.is_empty() && padding != 0) {
      return None;
    }
    if padding > 0 && data[data.len() - 1] & ((1 << padding) - 1) != 0 {
      return None;
    }

    Some(BitString { data, padding })
  }
}

impl<'a> asn1::SimpleAsn1Writable<'a> for BitString<'a> {
  const TAG: u8 = 0x03;
  fn write_data(&self, dest: &mut Vec<u8>) {
    dest.push(self.padding);
    dest.extend_from_slice(self.data);
  }
}

pub fn decode(bytes: &[u8]) -> Option<RSAPublicKey> {
  let result: asn1::ParseResult<_> = asn1::parse(bytes, |d| {
    return d.read_element::<asn1::Sequence>()?.parse(|d| {
      d.read_element::<asn1::Sequence>()?.parse(|d| {
        // Not sure what I should do with these. Going to ignore for now (read:
        // forever).
        let _alg = d.read_element::<asn1::ObjectIdentifier>()?;
        // Alg is always 1.2.840.113549.1.1.1
        let _params = d.read_element::<asn1::Tlv>()?;
        // Params are always NULL
        Ok(())
      })?;
      let pub_key = d.read_element::<asn1::BitString>()?;
      let (n, e) = asn1::parse(pub_key.as_bytes(), |d| {
        d.read_element::<asn1::Sequence>()?.parse(|d| {
          let n = d.read_element::<asn1::BigUint>()?;
          let e = d.read_element::<asn1::BigUint>()?;
          Ok((n, e))
        })
      })?;
      Ok((n, e))
    });
  });

  let (n, e) = result.unwrap();

  Some(
    RSAPublicKey::new(
      rsa::BigUint::from_bytes_be(n.as_bytes()),
      rsa::BigUint::from_bytes_be(e.as_bytes()),
    )
    .unwrap(),
  )
}

fn write_big_uint(w: &mut asn1::Writer, int: &rsa::BigUint) {
  let mut bytes = int.to_bytes_be();
  let out;
  let mut tmp = vec![];
  // asn1 BigUint requires the first byte to be a 0, to disambiguate from negative
  // values
  if bytes[0] & 0x80 != 0 {
    tmp.push(0);
    tmp.append(&mut bytes);
    out = asn1::BigUint::new(tmp.as_ref()).unwrap()
  } else {
    out = asn1::BigUint::new(&bytes).unwrap()
  }
  w.write_element(&out);
}

pub fn encode(key: &RSAPublicKey) -> Vec<u8> {
  asn1::write(|w| {
    w.write_element(&asn1::SequenceWriter::new(&|w| {
      // A sequence containing the algorithm used.
      w.write_element(&asn1::SequenceWriter::new(&|w| {
        w.write_element(&asn1::ObjectIdentifier::from_string("1.2.840.113549.1.1.1"));
        w.write_element(&()); // NULL value
      }));
      // A bitstring containing the N and E of the key
      w.write_element(
        &BitString::new(
          &asn1::write(|w| {
            w.write_element(&asn1::SequenceWriter::new(&|w| {
              write_big_uint(w, key.n());
              write_big_uint(w, key.e());
            }));
          }),
          0,
        )
        .unwrap(),
      );
    }));
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::rngs::OsRng;
  use rsa::RSAPrivateKey;

  #[test]
  fn encode_decode() {
    let mut rng = OsRng;
    let key = RSAPrivateKey::new(&mut rng, 1024).expect("failed to generate a key");

    let bytes = encode(&key);
    let new_key = decode(&bytes).unwrap();

    assert_eq!(key.n(), new_key.n());
    assert_eq!(key.e(), new_key.e());
  }
}

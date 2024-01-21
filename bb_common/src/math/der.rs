use asn1::{Tag, WriteBuf, WriteResult};
use rsa::{RsaPublicKey, traits::PublicKeyParts};

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

impl<'a> asn1::SimpleAsn1Writable for BitString<'a> {
  const TAG: Tag = asn1::Tag::primitive(0x03);

  fn write_data(&self, dest: &mut WriteBuf) -> WriteResult {
    dest.push_byte(self.padding);
    dest.push_slice(self.data);
    Ok(())
  }
}

pub fn decode(bytes: &[u8]) -> Option<RsaPublicKey> {
  let result: asn1::ParseResult<_> = asn1::parse(bytes, |d| {
    d.read_element::<asn1::Sequence>()?.parse(|d| {
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
    })
  });

  let (n, e) = result.unwrap();

  Some(
    RsaPublicKey::new(
      rsa::BigUint::from_bytes_be(n.as_bytes()),
      rsa::BigUint::from_bytes_be(e.as_bytes()),
    )
    .unwrap(),
  )
}

fn write_big_uint(w: &mut asn1::Writer, int: &rsa::BigUint) {
  let mut bytes = int.to_bytes_be();
  let mut tmp = vec![];
  // asn1 BigUint requires the first byte to be a 0, to disambiguate from negative
  // values
  let out = if bytes[0] & 0x80 != 0 {
    tmp.push(0);
    tmp.append(&mut bytes);
    asn1::BigUint::new(tmp.as_ref()).unwrap()
  } else {
    asn1::BigUint::new(&bytes).unwrap()
  };
  w.write_element(&out);
}

pub fn encode(key: &RsaPublicKey) -> Vec<u8> {
  asn1::write(|w| Ok({
    w.write_element(&asn1::SequenceWriter::new(&|w| Ok({
      // A sequence containing the algorithm used.
      w.write_element(&asn1::SequenceWriter::new(&|w| Ok({
        w.write_element(&asn1::ObjectIdentifier::from_string("1.2.840.113549.1.1.1"));
        w.write_element(&()); // NULL value
      })));
      // A bitstring containing the N and E of the key
      w.write_element(
        &BitString::new(
          &asn1::write(|w| Ok({
            w.write_element(&asn1::SequenceWriter::new(&|w| Ok({
              write_big_uint(w, key.n());
              write_big_uint(w, key.e());
            })));
          })).unwrap(),
          0,
        )
        .unwrap(),
      );
    })));
  })).unwrap()
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::rngs::OsRng;
  use rsa::{RsaPrivateKey, traits::PublicKeyParts};

  #[test]
  fn encode_decode() {
    let mut rng = OsRng;
    let key = RsaPrivateKey::new(&mut rng, 1024).expect("failed to generate a key");

    let bytes = encode(&key.to_public_key());
    let new_key = decode(&bytes).unwrap();

    assert_eq!(key.n(), new_key.n());
    assert_eq!(key.e(), new_key.e());
  }
}

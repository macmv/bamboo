use asn1::BigUint;
use rsa::{PaddingScheme, PublicKey, PublicKeyParts, RSAPrivateKey, RSAPublicKey};

pub fn decode(bytes: &[u8]) -> Option<RSAPublicKey> {
  let result: asn1::ParseResult<_> = asn1::parse(bytes, |d| {
    return d.read_element::<asn1::Sequence>()?.parse(|d| {
      d.read_element::<asn1::Sequence>()?.parse(|d| {
        let alg = d.read_element::<asn1::ObjectIdentifier>()?;
        let params = d.read_element::<asn1::Tlv>()?;
        info!("alg: {}", alg);
        info!("params: {:?}", params);
        Ok(())
      })?;
      let pub_key = d.read_element::<asn1::BitString>()?;
      info!("parsed key");
      let (n, e) = asn1::parse(pub_key.as_bytes(), |d| {
        d.read_element::<asn1::Sequence>()?.parse(|d| {
          info!("reading n...");
          let n = d.read_element::<BigUint>()?;
          info!("reading e...");
          let e = d.read_element::<BigUint>()?;
          Ok((n, e))
        })
      })?;
      info!("got n, e: {:?} {:?}", n, e);
      Ok((n, e))
    });
  });

  dbg!(result);

  None
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
        &asn1::write(|w| {
          w.write_element(&asn1::SequenceWriter::new(&|w| {
            // asn1 BigUint requires the first byte to be a 0, to disambiguate from negative
            // values
            let mut n = vec![0];
            n.append(&mut key.n().to_bytes_be());
            w.write_element(&BigUint::new(&n).unwrap());
            let mut e = vec![0];
            e.append(&mut key.e().to_bytes_be());
            w.write_element(&BigUint::new(&n).unwrap());
          }));
        })
        .as_slice(),
      );
    }));
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::rngs::OsRng;

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

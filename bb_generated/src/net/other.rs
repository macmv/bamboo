// /// Creates a type url from the given keyword. This should be used to
// /// endcode/decode Any types.
// macro_rules! create_type_url {
//   ($ty: expr) => {
//     concat!("type.googleapis.com/google.rpc.", stringify!($ty))
//   };
// }

// macro_rules! generate_other {
//   [$($val: ident),*] => {
//     #[derive(Debug)]
//     pub enum Other {
//       // Generates this:
//       // Chunk(proto::Chunk),
//       // BossBar(proto::BossBar),
//       $(
//         $val(proto::$val),
//       )*
//     }
//
//     impl Other {
//       pub fn from_any(pb: Any) -> Result<Self, DecodeError> {
//         match pb.type_url.as_str() {
//           // Generates a match statement for each type url in Other
//           $(
//             create_type_url!($val) => {
//               match proto::$val::decode(pb.value.as_slice()) {
//                 Ok(msg) => Ok(Other::$val(msg)),
//                 Err(e) => Err(e),
//               }
//             },
//           )*
//           _ => panic!("unknown type {}", pb.type_url),
//         }
//       }
//       pub fn to_any(&self) -> Result<Any, EncodeError> {
//         let mut b = bytes::BytesMut::new();
//         let name = match self {
//           // Generates a match statement for each type in Other.
//           // This will always be exhaustive.
//           $(
//             Self::$val(pb) => {
//               pb.encode(&mut b)?;
//               create_type_url!($val)
//             },
//           )*
//         };
//         Ok(Any { type_url: name.to_string(), value: b.to_vec() })
//       }
//     }
//   };
// }

// generate_other![Chunk, PlayerList, BossBar];

pub mod proto {
  tonic::include_proto!("connection");

  pub const FILE_DESCRIPTOR_SET: &'static [u8] = tonic::include_file_descriptor_set!("connection");
}

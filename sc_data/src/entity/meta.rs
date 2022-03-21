use super::{cross::cross_version_metadata, Entity};
use crate::{gen::CodeGen, Version};

pub fn entity_metadata(gen: &mut CodeGen, versions: &[(&Version, Option<&Entity>)]) {
  gen.write("Metadata ");
  gen.write_block(|gen| {
    let latest = &versions.last().unwrap().1.unwrap();

    gen.write("entity: \"");
    gen.write(&latest.name);
    gen.write_line("\",");

    gen.write_line("versions: &[");
    gen.add_indent();
    for (version, ent) in versions {
      gen.write("MetadataVersion ");
      gen.write_block(|gen| {
        gen.write("version: ");
        gen.write(&version.to_block());
        gen.write_line(",");

        gen.write_line("fields: &[");
        if let Some(e) = ent {
          for meta in &e.metadata {
            gen.write("MetadataField ");
            gen.write_block(|gen| {
              gen.write("id: ");
              gen.write(&meta.id.to_string());
              gen.write_line(",");
              gen.write("name: \"");
              gen.write(&meta.name);
              gen.write_line("\",");
              gen.write("ty: MetadataType::");
              gen.write(&format!("{:?}", meta.ty));
              gen.write_line(",");
            });
            gen.write_line(",");
          }
        }
        gen.write_line("],");
      });
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write_line("],");
  });
}

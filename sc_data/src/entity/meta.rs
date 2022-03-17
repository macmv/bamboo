use super::{cross::cross_version_metadata, EntityDef};
use crate::{gen::CodeGen, Version};

pub fn entity_metadata(gen: &mut CodeGen, v: &Version, ent: &EntityDef, latest: &EntityDef) {
  gen.write("Metadata ");
  gen.write_block(|gen| {
    gen.write_line("entities: &[");
    gen.add_indent();
    for e in ent.entities.iter().flatten() {
      gen.write("EntityMetadata ");
      gen.write_block(|gen| {
        gen.write("entity: \"");
        gen.write(&e.name);
        gen.write_line("\",");

        gen.write_line("fields: &[");
        gen.add_indent();
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
        gen.remove_indent();
        gen.write_line("]");

        cross_version_metadata(gen, v, e, latest.get(&e.name));
      });
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write_line("]");
  });
}

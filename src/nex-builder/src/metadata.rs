//! Zub metadata written and read by the builder.

pub(crate) const AUTHOR: &str = "nex-builder";
pub(crate) const CHECKSUM: &str = "nex.build.checksum";
pub(crate) const RECIPE: &str = "nex.build.recipe";

pub(crate) fn build<'a>(checksum: &'a str, recipe: &'a str) -> [(&'static str, &'a str); 2] {
    [(CHECKSUM, checksum), (RECIPE, recipe)]
}

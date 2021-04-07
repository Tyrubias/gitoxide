use crate::{compound, loose};
use git_object::{owned, Kind};
use std::io::Read;

impl crate::Write for compound::Db {
    type Error = loose::db::write::Error;

    fn write(&self, object: &owned::Object, hash: git_hash::Kind) -> Result<git_hash::Id, Self::Error> {
        self.loose.write(object, hash)
    }

    fn write_buf(&self, object: Kind, from: &[u8], hash: git_hash::Kind) -> Result<git_hash::Id, Self::Error> {
        self.loose.write_buf(object, from, hash)
    }

    fn write_stream(
        &self,
        kind: Kind,
        size: u64,
        from: impl Read,
        hash: git_hash::Kind,
    ) -> Result<git_hash::Id, Self::Error> {
        self.loose.write_stream(kind, size, from, hash)
    }
}

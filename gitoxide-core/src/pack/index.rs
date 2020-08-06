use git_features::progress::Progress;
use git_odb::pack;
use std::{fs, io, path::PathBuf, str::FromStr};

#[derive(PartialEq, Debug)]
pub enum IterationMode {
    AsIs,
    Verify,
    Restore,
}

impl IterationMode {
    pub fn variants() -> &'static [&'static str] {
        &["as-is", "verify", "restore"]
    }
}

impl Default for IterationMode {
    fn default() -> Self {
        IterationMode::Verify
    }
}

impl FromStr for IterationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use IterationMode::*;
        let slc = s.to_ascii_lowercase();
        Ok(match slc.as_str() {
            "as-is" => AsIs,
            "verify" => Verify,
            "restore" => Restore,
            _ => return Err("invalid value".into()),
        })
    }
}

impl From<IterationMode> for pack::data::iter::Mode {
    fn from(v: IterationMode) -> Self {
        use pack::data::iter::Mode::*;
        match v {
            IterationMode::AsIs => AsIs,
            IterationMode::Verify => Verify,
            IterationMode::Restore => Restore,
        }
    }
}

pub struct Context {
    pub thread_limit: Option<usize>,
    pub iteration_mode: IterationMode,
}

impl From<Context> for pack::bundle::write::Options {
    fn from(
        Context {
            thread_limit,
            iteration_mode,
        }: Context,
    ) -> Self {
        pack::bundle::write::Options {
            thread_limit,
            iteration_mode: iteration_mode.into(),
            index_kind: pack::index::Kind::default(),
        }
    }
}

pub fn stream_len(mut s: impl io::Seek) -> io::Result<u64> {
    use io::SeekFrom;
    let old_pos = s.seek(SeekFrom::Current(0))?;
    let len = s.seek(SeekFrom::End(0))?;
    if old_pos != len {
        s.seek(SeekFrom::Start(old_pos))?;
    }
    Ok(len)
}

pub fn from_pack<P>(
    pack: Option<PathBuf>,
    directory: Option<PathBuf>,
    progress: P,
    context: Context,
) -> anyhow::Result<()>
where
    P: Progress,
    <<P as Progress>::SubProgress as Progress>::SubProgress: Send,
{
    use anyhow::Context;
    match pack {
        Some(pack) => {
            let pack_len = pack.metadata()?.len();
            let pack_file = fs::File::open(pack)?;
            pack::Bundle::write_to_directory(pack_file, Some(pack_len), directory, progress, context.into())
        }
        None => {
            let stdin = io::stdin();
            pack::Bundle::write_to_directory(stdin.lock(), None, directory, progress, context.into())
        }
    }
    .with_context(|| "Failed to write pack and index")
    .map(|_| ())
}

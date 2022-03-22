use git_object::bstr::BStr;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub struct Outcome<'a> {
    pub name: Cow<'a, BStr>,
    pub id: git_hash::ObjectId,
    pub hex_len: usize,
    pub depth: usize,
    pub long: bool,
    pub dirty_suffix: Option<String>,
}

impl<'a> Outcome<'a> {
    pub fn is_exact_match(&self) -> bool {
        self.depth == 0
    }
    pub fn long(&mut self) -> &mut Self {
        self.long = true;
        self
    }
    pub fn short(&mut self) -> &mut Self {
        self.long = false;
        self
    }
}

impl<'a> Display for Outcome<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.long && self.is_exact_match() {
            self.name.fmt(f)?;
        } else {
            write!(
                f,
                "{}-{}-g{}",
                self.name,
                self.depth,
                self.id.to_hex_with_len(self.hex_len)
            )?;
        }
        if let Some(suffix) = &self.dirty_suffix {
            write!(f, "-{}", suffix)?;
        }
        Ok(())
    }
}

pub(crate) mod function {
    use super::Outcome;
    use git_hash::{oid, ObjectId};
    use git_object::bstr::BStr;
    use git_object::CommitRefIter;
    use std::borrow::Cow;
    use std::collections::{hash_map, HashMap, VecDeque};
    use std::iter::FromIterator;

    #[allow(clippy::result_unit_err)]
    pub fn describe<'a, Find, E>(
        commit: &oid,
        mut find: Find,
        hex_len: usize,
        name_set: &HashMap<ObjectId, Cow<'a, BStr>>,
    ) -> Result<Option<Outcome<'a>>, E>
    where
        Find: for<'b> FnMut(&oid, &'b mut Vec<u8>) -> Result<CommitRefIter<'b>, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        if let Some(name) = name_set.get(commit) {
            return Ok(Some(Outcome {
                name: name.clone(),
                id: commit.to_owned(),
                hex_len,
                depth: 0,
                long: false,
                dirty_suffix: None,
            }));
        }
        let mut buf = Vec::new();
        // TODO: what if there is no committer?
        let commit_time = find(commit, &mut buf)?
            .committer()
            .map(|c| c.time.seconds_since_unix_epoch)
            .unwrap_or_default();
        let mut queue = VecDeque::from_iter(Some((commit.to_owned(), commit_time)));
        let mut candidates = Vec::new();
        let mut seen_commits = 0;
        let mut gave_up_on_commit = None;
        let mut seen = hash_hasher::HashedMap::default();
        seen.insert(commit.to_owned(), 0u32);
        let mut parent_buf = Vec::new();

        const MAX_CANDIDATES: usize = std::mem::size_of::<Flags>() * 8;
        while let Some((commit, _commit_time)) = queue.pop_front() {
            seen_commits += 1;
            if let Some(name) = name_set.get(&commit) {
                if candidates.len() < MAX_CANDIDATES {
                    let identity_bit = 1 << candidates.len();
                    candidates.push(Candidate {
                        name: name.clone(),
                        commits_in_its_future: seen_commits - 1,
                        identity_bit,
                        order: candidates.len(),
                    });
                    *seen.get_mut(&commit).expect("inserted") |= identity_bit;
                } else {
                    gave_up_on_commit = Some(commit);
                    break;
                }
            }

            let flags = seen[&commit];
            for candidate in candidates
                .iter_mut()
                .filter(|c| !((flags & c.identity_bit) == c.identity_bit))
            {
                candidate.commits_in_its_future += 1;
            }

            let commit_iter = find(&commit, &mut buf)?;
            for token in commit_iter {
                match token {
                    Ok(git_object::commit::ref_iter::Token::Tree { .. }) => continue,
                    Ok(git_object::commit::ref_iter::Token::Parent { id }) => {
                        let mut parent = find(id.as_ref(), &mut parent_buf)?;

                        // TODO: figure out if not having a date is a hard error.
                        let parent_commit_date = parent
                            .committer()
                            .map(|committer| committer.time.seconds_since_unix_epoch)
                            .unwrap_or_default();

                        let at = match queue.binary_search_by(|e| e.1.cmp(&parent_commit_date).reverse()) {
                            Ok(pos) => pos,
                            Err(pos) => pos,
                        };
                        match seen.entry(id) {
                            hash_map::Entry::Vacant(entry) => {
                                entry.insert(flags);
                                queue.insert(at, (id, parent_commit_date))
                            }
                            hash_map::Entry::Occupied(mut entry) => {
                                *entry.get_mut() |= flags;
                            }
                        }
                    }
                    Ok(_unused_token) => break,
                    Err(err) => todo!("return a decode error"),
                }
            }
        }
        dbg!(candidates);
        todo!("actually search for it")
    }

    type Flags = u32;
    #[derive(Debug)]
    struct Candidate<'a> {
        name: Cow<'a, BStr>,
        commits_in_its_future: Flags,
        /// A single bit identifying this candidate uniquely in a bitset
        identity_bit: Flags,
        /// The order at which we found the candidate, first one has order = 0
        order: usize,
    }
}

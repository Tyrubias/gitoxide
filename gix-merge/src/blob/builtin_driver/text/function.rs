use crate::blob::builtin_driver::text::utils::{
    assure_ends_with_nl, contains_lines, detect_line_ending, detect_line_ending_or_nl, fill_ancestor,
    hunks_differ_in_diff3, take_intersecting, tokens, write_ancestor, write_conflict_marker, write_hunks,
    zealously_contract_hunks, CollectHunks, Hunk, Side,
};
use crate::blob::builtin_driver::text::{Conflict, ConflictStyle, Labels, Options};
use crate::blob::Resolution;

/// Merge `current` and `other` with `ancestor` as base according to `opts`.
///
/// Use `labels` to annotate conflict sections.
///
/// `input` is for reusing memory for lists of tokens, but note that it grows indefinitely
/// while tokens for `current`, `ancestor` and `other` are added.
/// Place the merged result in `out` (cleared before use) and return the resolution.
///
/// # Important
///
/// *The caller* is responsible for clearing `input`, otherwise tokens will accumulate.
/// This idea is to save time if the input is known to be very similar.
#[allow(clippy::too_many_arguments)]
pub fn merge<'a>(
    out: &mut Vec<u8>,
    input: &mut imara_diff::intern::InternedInput<&'a [u8]>,
    Labels {
        ancestor: ancestor_label,
        current: current_label,
        other: other_label,
    }: Labels<'_>,
    current: &'a [u8],
    ancestor: &'a [u8],
    other: &'a [u8],
    opts: Options,
) -> Resolution {
    out.clear();
    input.update_before(tokens(ancestor));
    input.update_after(tokens(current));

    let current_hunks = imara_diff::diff(
        opts.diff_algorithm,
        input,
        CollectHunks {
            side: Side::Current,
            hunks: Default::default(),
        },
    );

    let current_tokens = std::mem::take(&mut input.after);
    input.update_after(tokens(other));

    let mut hunks = imara_diff::diff(
        opts.diff_algorithm,
        input,
        CollectHunks {
            side: Side::Other,
            hunks: current_hunks,
        },
    );

    hunks.sort_by(|a, b| a.before.start.cmp(&b.before.start));
    let mut hunks = hunks.into_iter().peekable();
    let mut intersecting = Vec::new();
    let mut ancestor_integrated_until = 0;
    let mut resolution = Resolution::Complete;
    let mut filled_hunks = Vec::with_capacity(2);
    while let Some(hunk) = hunks.next() {
        if take_intersecting(&hunk, &mut hunks, &mut intersecting) {
            fill_ancestor(&hunk.before, &mut intersecting);

            let filled_hunks_side = hunk.side;
            filled_hunks.clear();
            filled_hunks.push(hunk);
            fill_ancestor(
                &intersecting
                    .first()
                    .zip(intersecting.last())
                    .map(|(f, l)| f.before.start..l.before.end)
                    .expect("at least one entry"),
                &mut filled_hunks,
            );
            match opts.conflict {
                Conflict::Keep { style, marker_size } => {
                    let (hunks_front_and_back, num_hunks_front) = match style {
                        ConflictStyle::Merge | ConflictStyle::ZealousDiff3 => {
                            zealously_contract_hunks(&mut filled_hunks, &mut intersecting, input, &current_tokens)
                        }
                        ConflictStyle::Diff3 => (Vec::new(), 0),
                    };
                    let (our_hunks, their_hunks) = match filled_hunks_side {
                        Side::Current => (&filled_hunks, &intersecting),
                        Side::Other => (&intersecting, &filled_hunks),
                        Side::Ancestor => {
                            unreachable!("initial hunks are never ancestors")
                        }
                    };
                    let (front_hunks, back_hunks) = hunks_front_and_back.split_at(num_hunks_front);
                    let first_hunk = front_hunks
                        .first()
                        .or(our_hunks.first())
                        .expect("at least one hunk to write");
                    let last_hunk = back_hunks
                        .last()
                        .or(their_hunks.last())
                        .or(our_hunks.last())
                        .or(front_hunks.last())
                        .expect("at least one hunk");
                    write_ancestor(input, ancestor_integrated_until, first_hunk.before.start as usize, out);

                    write_hunks(front_hunks, input, &current_tokens, out);
                    if their_hunks.is_empty() {
                        write_hunks(our_hunks, input, &current_tokens, out);
                    } else if our_hunks.is_empty() {
                        // TODO: assure we run into this - currently no test triggers this. Can this happen at all?
                        write_hunks(their_hunks, input, &current_tokens, out);
                    } else {
                        // DEVIATION: this makes tests (mostly) pass, but probably is very different from what Git does.
                        let hunk_storage;
                        let nl = detect_line_ending(
                            if front_hunks.is_empty() {
                                hunk_storage = Hunk {
                                    before: ancestor_integrated_until..first_hunk.before.start,
                                    after: Default::default(),
                                    side: Side::Ancestor,
                                };
                                std::slice::from_ref(&hunk_storage)
                            } else {
                                front_hunks
                            },
                            input,
                            &current_tokens,
                        )
                        .or_else(|| detect_line_ending(our_hunks, input, &current_tokens))
                        .unwrap_or(b"\n".into());
                        match style {
                            ConflictStyle::Merge => {
                                if contains_lines(our_hunks) || contains_lines(their_hunks) {
                                    resolution = Resolution::Conflict;
                                    write_conflict_marker(out, b'<', current_label, marker_size, nl);
                                    write_hunks(our_hunks, input, &current_tokens, out);
                                    write_conflict_marker(out, b'=', None, marker_size, nl);
                                    write_hunks(their_hunks, input, &current_tokens, out);
                                    write_conflict_marker(out, b'>', other_label, marker_size, nl);
                                }
                            }
                            ConflictStyle::Diff3 | ConflictStyle::ZealousDiff3 => {
                                if contains_lines(our_hunks) || contains_lines(their_hunks) {
                                    if hunks_differ_in_diff3(style, our_hunks, their_hunks, input, &current_tokens) {
                                        resolution = Resolution::Conflict;
                                        write_conflict_marker(out, b'<', current_label, marker_size, nl);
                                        write_hunks(our_hunks, input, &current_tokens, out);
                                        let ancestor_hunk = Hunk {
                                            before: first_hunk.before.start..last_hunk.before.end,
                                            after: Default::default(),
                                            side: Side::Ancestor,
                                        };
                                        let ancestor_hunk = std::slice::from_ref(&ancestor_hunk);
                                        let ancestor_nl =
                                            detect_line_ending_or_nl(ancestor_hunk, input, &current_tokens);
                                        write_conflict_marker(out, b'|', ancestor_label, marker_size, ancestor_nl);
                                        write_hunks(ancestor_hunk, input, &current_tokens, out);
                                        write_conflict_marker(out, b'=', None, marker_size, nl);
                                        write_hunks(their_hunks, input, &current_tokens, out);
                                        write_conflict_marker(out, b'>', other_label, marker_size, nl);
                                    } else {
                                        write_hunks(our_hunks, input, &current_tokens, out);
                                    }
                                }
                            }
                        }
                    }
                    write_hunks(back_hunks, input, &current_tokens, out);
                    ancestor_integrated_until = last_hunk.before.end;
                }
                Conflict::ResolveWithOurs | Conflict::ResolveWithTheirs => {
                    let (our_hunks, their_hunks) = match filled_hunks_side {
                        Side::Current => (&filled_hunks, &intersecting),
                        Side::Other => (&intersecting, &filled_hunks),
                        Side::Ancestor => {
                            unreachable!("initial hunks are never ancestors")
                        }
                    };
                    let hunks_to_write = if opts.conflict == Conflict::ResolveWithOurs {
                        our_hunks
                    } else {
                        their_hunks
                    };
                    if let Some(first_hunk) = hunks_to_write.first() {
                        write_ancestor(input, ancestor_integrated_until, first_hunk.before.start as usize, out);
                    }
                    write_hunks(hunks_to_write, input, &current_tokens, out);
                    if let Some(last_hunk) = hunks_to_write.last() {
                        ancestor_integrated_until = last_hunk.before.end;
                    }
                }
                Conflict::ResolveWithUnion => {
                    let (hunks_front_and_back, num_hunks_front) =
                        zealously_contract_hunks(&mut filled_hunks, &mut intersecting, input, &current_tokens);

                    let (our_hunks, their_hunks) = match filled_hunks_side {
                        Side::Current => (&filled_hunks, &intersecting),
                        Side::Other => (&intersecting, &filled_hunks),
                        Side::Ancestor => {
                            unreachable!("initial hunks are never ancestors")
                        }
                    };
                    let (front_hunks, back_hunks) = hunks_front_and_back.split_at(num_hunks_front);
                    let first_hunk = front_hunks
                        .first()
                        .or(our_hunks.first())
                        .expect("at least one hunk to write");
                    write_ancestor(input, ancestor_integrated_until, first_hunk.before.start as usize, out);
                    write_hunks(front_hunks, input, &current_tokens, out);
                    assure_ends_with_nl(out, detect_line_ending_or_nl(front_hunks, input, &current_tokens));
                    write_hunks(our_hunks, input, &current_tokens, out);
                    assure_ends_with_nl(out, detect_line_ending_or_nl(our_hunks, input, &current_tokens));
                    write_hunks(their_hunks, input, &current_tokens, out);
                    if !back_hunks.is_empty() {
                        assure_ends_with_nl(out, detect_line_ending_or_nl(their_hunks, input, &current_tokens));
                    }
                    write_hunks(back_hunks, input, &current_tokens, out);
                    let last_hunk = back_hunks
                        .last()
                        .or(their_hunks.last())
                        .or(our_hunks.last())
                        .or(front_hunks.last())
                        .expect("at least one hunk");
                    ancestor_integrated_until = last_hunk.before.end;
                }
            }
        } else {
            write_ancestor(input, ancestor_integrated_until, hunk.before.start as usize, out);
            ancestor_integrated_until = hunk.before.end;
            write_hunks(std::slice::from_ref(&hunk), input, &current_tokens, out);
        }
    }
    write_ancestor(input, ancestor_integrated_until, input.before.len(), out);

    resolution
}

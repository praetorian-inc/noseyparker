--------------------------------------------------------------------------------
-- blobs
--------------------------------------------------------------------------------
CREATE TABLE blob
-- This table records basic metadata about blobs.
(
    -- An arbitrary integer identifier for the blob
    id integer primary key,

    -- The blob hash, computed a la Git, i.e., a hex digest of a fancy SHA-1 hash
    blob_id text unique not null,

    -- Size of the blob in bytes
    size integer not null,

    constraint valid_id check(
        length(blob_id) == 40 and not glob('*[^abcdefABCDEF1234567890]*', blob_id)
    ),

    constraint valid_size check(0 <= size)
) strict;

CREATE TABLE blob_mime_essence
-- This table records mime type metadata about blobs.
(
    -- The integer identifier of the blob
    blob_id integer primary key references blob(id),

    -- Guessed mime type of the blob
    mime_essence text not null
) strict;

CREATE TABLE blob_charset
-- This table records charset metadata about blobs.
(
    -- The integer identifier of the blob
    blob_id integer primary key references blob(id),

    -- Guessed charset encoding of the blob
    charset text not null
) strict;

CREATE TABLE blob_source_span
-- This table represents source span-based location information for ranges within blobs.
-- This allows you to look up line and column information given a (start byte, end byte) range.
(
    blob_id integer not null references blob(id),
    start_byte integer not null,
    end_byte integer not null,

    start_line integer not null,
    start_column integer not null,
    end_line integer not null,
    end_column integer not null,

    unique(blob_id, start_byte, end_byte),

    constraint valid_offsets check(0 <= start_byte and start_byte <= end_byte),

    constraint valid_span check(0 <= start_line
        and start_line <= end_line
        and 0 <= start_column
        and 0 <= end_column
    )
) strict;

CREATE TABLE blob_provenance
-- This table records the various ways in which blobs were encountered.
-- A blob can be encountered multiple ways when scanning; this table records all of them.
(
    -- The integer identifier of the blob
    blob_id integer not null references blob(id),

    -- The JSON-formatted provenance information
    -- TODO: deduplicate these values via another table?
    -- TODO: allow recursive representation of provenance values? I.e., structural decomposition and sharing, like `git repo` -> `commit` -> `blob path`?
    -- TODO: define special JSON object fields that will be handled specially by NP? E.g., `path`, `repo_path`, ...?
    provenance text not null,

    unique(blob_id, provenance),

    constraint payload_valid check(json_type(provenance) = 'object')
) strict;


--------------------------------------------------------------------------------
-- rules
--------------------------------------------------------------------------------
CREATE TABLE rule
-- This table records rules used for detection.
(
    -- An arbitrary integer identifier for the rule
    id integer primary key,

    -- The name specified in the rule, e.g., `AWS API Key`
    name text not null,

    -- The text-based identifier specified in the rule, e.g., `np.aws.1`
    text_id text not null,

    -- The regular expression pattern specified in the rule
    pattern text not null,

    -- TODO: add JSON representation of the rule?
    -- TODO: add structural identifier? perhaps sha1_hex(pattern)?  Or by convention, keep text_id values stable, never changing the pattern

    unique(name, text_id, pattern)
) strict;

-- FIXME: need an additional table to make this useful, like a relation of (invocation id, match id) to keep track of which matches were seen in which invocation
/*
CREATE TABLE invocation
-- This table records the scanner invocations.
(
    -- An arbitrary integer identifier
    id integer primary key,

    -- The datetime of invocation
    timestamp text not null,

    -- The JSON array of command-line arguments
    cli_args text not null,

    constraint timestamp_valid check(datetime(timestamp) is not null),

    constraint cli_args_valid check(json_type(cli_args) = 'array')
) strict;
*/

--------------------------------------------------------------------------------
-- snippets
--------------------------------------------------------------------------------
CREATE TABLE snippet
-- This table deduplicates contextual snippets.
-- Deduplication of snippets reduces the size of large datastores 20-100x or more.
(
    -- An arbitrary integer identifier for the snippet
    id integer primary key,

    -- The snippet content
    snippet blob unique not null
) strict;


--------------------------------------------------------------------------------
-- matches
--------------------------------------------------------------------------------
CREATE TABLE match
-- This table represents the matches found from scanning.
--
-- See the `noseyparker::match_type::Match` type in noseyparker for correspondence.
(
    -- An arbitrary integer identifier for the match
    id integer primary key,

    -- TODO: move this to a separate table?
    -- TODO: The content-based identifier of the match, defined as sha1_hex(rule structural identifier + '\0' + matching input)
    structural_id text not null,

    -- The blob in which this match occurs
    blob_id integer not null references blob(id),

    -- The byte offset within the blob for the start of the match
    start_byte integer not null,

    -- The byte offset within the blob for the end of the match
    end_byte integer not null,

    -- The rule that produced this match
    rule_id integer not null references rule(id),

    unique (
        blob_id,
        start_byte,
        end_byte,
        rule_id
    ),

    constraint valid_offsets check(0 <= start_byte and start_byte <= end_byte),

    foreign key (blob_id, start_byte, end_byte) references blob_source_span(blob_id, start_byte, end_byte),

    -- Ensure that snippets and groups are provided for this match
    foreign key (id) references match_snippet(match_id) deferrable initially deferred

    -- XXX This foreign key does not work with sqlite, as `match_id` does not
    -- have a unique index. See the `child_7` case: https://www.sqlite.org/foreignkeys.html#fk_indexes
    -- ,foreign key (id) references match_group(match_id) deferrable initially deferred
) strict;

CREATE TABLE match_snippet
-- This table represents the contextual snippets for each match.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The contextual snippet preceding the matching input
    before_snippet_id integer not null references snippet(id),

    -- The entire matching input
    matching_snippet_id integer not null references snippet(id),

    -- The contextual snippet trailing the matching input
    after_snippet_id integer not null references snippet(id)
) strict;

-- TODO: collect location information for each individual match group
CREATE TABLE match_group
-- This table represents the match group content of each match.
-- Most rules produce matches with a single match group; however, some rules
-- produce multiple match groups, such as to isolate both username and password
-- from a connection string.
(
    -- An arbitrary integer identifier
    id integer primary key,

    -- The match that this group belongs to
    match_id integer not null references match(id),

    -- The rule's group index for this match
    group_index integer not null,

    -- The identifier of the content of this group
    group_input_id integer not null references snippet(id),

    unique(match_id, group_index),

    constraint valid_group_index check(0 <= group_index)
) strict;

-- CREATE INDEX match_grouping_index on match (group_input_id, rule_id);
CREATE INDEX match_rule_index on match(rule_id);
CREATE INDEX match_group_input_index on match_group(group_input_id);

CREATE TABLE match_status
-- This table records the accepted/rejected status of matches.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The assigned status, either `accept` or `reject`
    status text not null,

    constraint status_valid check (status in ('accept', 'reject'))
) strict;

CREATE TABLE match_comment
-- This table records ad-hoc comments assigned to matches.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The assigned comment, a non-empty string
    comment text not null,

    constraint comment_valid check (comment != '')
) strict;

CREATE TABLE match_score
-- This table records a numeric score for matches.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The scoring method used
    method text not null,

    -- The numeric score in [0, 1]
    score real not null,

    unique(match_id, method),

    constraint score_valid check (0.0 <= score and score <= 1.0),

    constraint method_valid check (method != '')
) strict;


--------------------------------------------------------------------------------
-- Convenience Views
--------------------------------------------------------------------------------
CREATE VIEW match_denorm
-- A convenience view to view matches in their fully denormalized form rather
-- than the low-level datastore form that involves numerous indirection.
(
    id,
    structural_id,

    blob_id,
    size,
    mime_essence,
    charset,
    provenance,

    start_byte,
    end_byte,

    start_line,
    start_column,
    end_line,
    end_column,

    rule_name,
    rule_text_id,
    rule_pattern,

    group_index,
    group_input,

    before_snippet,
    matching_snippet,
    after_snippet,

    status,
    comment,
    score_method,
    score
) as
select
    m.id,
    m.structural_id,

    b.blob_id,
    b.size,
    bm.mime_essence,
    bc.charset,
    bp.provenance,

    bss.start_line,
    bss.start_column,
    bss.end_line,
    bss.end_column,

    m.start_byte,
    m.end_byte,

    r.name,
    r.text_id,
    r.pattern,

    mg.group_index,
    group_input.snippet,

    before_snippet.snippet,
    matching_snippet.snippet,
    after_snippet.snippet,

    match_status.status,
    match_comment.comment,
    match_score.method,
    match_score.score
FROM
    match m
    left outer join blob_source_span bss on (m.blob_id = bss.blob_id and m.start_byte = bss.start_byte and m.end_byte = bss.end_byte)
    left outer join match_group mg on (m.id = mg.match_id)
    left outer join match_snippet ms on (m.id = ms.match_id)
    left outer join blob b on (m.blob_id = b.id)
    left outer join blob_mime_essence bm on (m.blob_id = bm.blob_id)
    left outer join blob_charset bc on (m.blob_id = bc.blob_id)
    left outer join rule r on (m.rule_id = r.id)
    left outer join snippet group_input on (mg.group_input_id = group_input.id)
    left outer join snippet before_snippet on (ms.before_snippet_id = before_snippet.id)
    left outer join snippet matching_snippet on (ms.matching_snippet_id = matching_snippet.id)
    left outer join snippet after_snippet on (ms.after_snippet_id = after_snippet.id)
    left outer join match_status on (mg.id = match_status.match_id)
    left outer join match_comment on (mg.id = match_comment.match_id)
    left outer join match_score on (mg.id = match_score.match_id)
    left outer join blob_provenance bp on (b.id = bp.blob_id)
;

CREATE VIEW blob_denorm
-- A convenience view to view blobs in their fully denormalized form rather
-- than the low-level datastore form that involves numerous indirection.
(
    id,
    blob_id,
    size,
    mime_essence,
    charset,
    provenance
)
as
select
    b.id,
    b.blob_id,
    b.size,
    bm.mime_essence,
    bc.charset,
    bp.provenance
from
    blob b
    left outer join blob_mime_essence bm on (b.id = bm.blob_id)
    left outer join blob_charset bc on (b.id = bc.blob_id)
    left outer join blob_provenance bp on (b.id = bp.blob_id)
;

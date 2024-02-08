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
) STRICT;

CREATE TABLE blob_mime_essence
-- This table records mime type metadata about blobs.
(
    -- The integer identifier of the blob
    blob_id integer primary key references blob(id),

    -- Guessed mime type of the blob
    mime_essence text not null
) STRICT;

CREATE TABLE blob_charset
-- This table records charset metadata about blobs.
(
    -- The integer identifier of the blob
    blob_id integer primary key references blob(id),

    -- Guessed charset encoding of the blob
    charset text not null
) STRICT;

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
) STRICT;

CREATE TABLE blob_provenance
-- This table records the various ways in which blobs were encountered.
-- A blob can be encountered multiple ways when scanning; this table records all of them.
(
    -- The integer identifier of the blob
    blob_id integer not null references blob(id),

    -- The minified JSON-formatted provenance information
    -- XXX: deduplicate these values via another table?
    -- XXX: allow recursive representation of provenance values? I.e., structural decomposition and sharing, like `git repo` -> `commit` -> `blob path`?
    -- XXX: define special JSON object fields that will be handled specially by NP? E.g., `path`, `repo_path`, ...?
    provenance text not null,

    unique(blob_id, provenance),

    constraint payload_valid check(json_type(provenance) = 'object')
) STRICT;

--------------------------------------------------------------------------------
-- rules
--------------------------------------------------------------------------------
CREATE TABLE rule
-- This table records rules used for detection.
(
    -- An arbitrary integer identifier for the rule
    id integer primary key,

    -- The human-readable name of the rule
    name text not null,

    -- The textual identifier defined in the rule
    text_id text not null,

    -- A content-based identifier, defined as `text id:sha1 hash of the pattern`.
    structural_id text unique not null,

    foreign key (id) references rule_syntax(rule_id) deferrable initially deferred
) STRICT;

CREATE TABLE rule_syntax
-- This table records the abstract syntax of each rule.
(
    rule_id integer primary key references rule(id),

    -- The minified JSON serialization of the rule
    syntax text not null,

    constraint json_syntax_valid check(json_type(syntax) = 'object')
) STRICT;

--------------------------------------------------------------------------------
-- snippets
--------------------------------------------------------------------------------
CREATE TABLE snippet
-- This table represents contextual snippets in a deduplicated way.
--
-- Deduplication of snippets reduces the size of large datastores 20-100x or more.
-- Keeping them in a separate table also makes it possible to update _just_ the
-- snippets of matches when scanning using a larger context window.
(
    -- An arbitrary integer identifier for the snippet
    id integer primary key,

    -- The snippet content
    snippet blob unique not null
) STRICT;

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

    foreign key (blob_id, start_byte, end_byte) references blob_source_span(blob_id, start_byte, end_byte) deferrable initially deferred,

    -- Ensure that snippets, groups, finding IDs, and structural IDs are provided for each match
    foreign key (id) references match_snippet(match_id) deferrable initially deferred,
    foreign key (id) references match_groups(match_id) deferrable initially deferred,
    foreign key (id) references match_structural_id(match_id) deferrable initially deferred,
    foreign key (id) references match_finding_id(match_id) deferrable initially deferred
) STRICT;

CREATE TABLE match_groups
-- This table represents the capture groups belonging to each match.
(
    match_id integer primary key references match(id),

    -- The capture groups from the match, encoded as a minified JSON array of base64-encoded bytestrings
    groups text not null,

    constraint valid_groups check(json_type(groups) = 'array')
) STRICT;

CREATE INDEX match_groups_index on match_groups(groups);

CREATE TABLE match_structural_id
-- This table represents content-based identifiers assigned to each match.
(
    match_id integer primary key references match(id),

    -- The content-based unique identifier of the match
    -- sha1_hex(rule structural identifier + '\0' + hex blob id + '\0' + decimal start byte + '\0' + decimal end byte)
    structural_id text unique not null
) STRICT;

CREATE TABLE match_finding_id
-- This table represents the finding identifier assigned to each match.
-- Matches with the same finding identifier are different occurrences of the same match.
(
    match_id integer primary key references match(id),
    -- The content-based identifier of the finding this match belongs to, defined as
    -- sha1_hex(rule structural identifier + '\0' + minified JSON array of base64-encoded groups)
    finding_id text not null
) STRICT;

CREATE INDEX match_finding_id_index on match_finding_id(finding_id);

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
) STRICT;

CREATE INDEX match_rule_index on match(rule_id);

CREATE TABLE match_status
-- This table records the accepted/rejected status of matches.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The assigned status, either `accept` or `reject`
    status text not null,

    constraint status_valid check (status in ('accept', 'reject'))
) STRICT;

CREATE TABLE match_comment
-- This table records ad-hoc comments assigned to matches.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The assigned comment, a non-empty string
    comment text not null,

    constraint comment_valid check (comment != '')
) STRICT;

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
) STRICT;


--------------------------------------------------------------------------------
-- Convenience Views
--------------------------------------------------------------------------------
CREATE VIEW match_denorm
-- A convenience view for matches in their fully denormalized form rather
-- than the low-level datastore form that involves numerous indirections.
(
    id,
    structural_id,
    finding_id,

    blob_id,

    start_byte,
    end_byte,

    start_line,
    start_column,
    end_line,
    end_column,

    rule_name,
    rule_text_id,
    rule_pattern,

    groups,

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
    m.finding_id,

    b.blob_id,

    bss.start_line,
    bss.start_column,
    bss.end_line,
    bss.end_column,

    m.start_byte,
    m.end_byte,

    r.name,
    r.text_id,
    r.pattern,

    mg.groups,

    before_snippet.snippet,
    matching_snippet.snippet,
    after_snippet.snippet,

    match_status.status,
    match_comment.comment,
    match_score.method,
    match_score.score
from
    match m
    left outer join blob_source_span bss on (m.blob_id = bss.blob_id and m.start_byte = bss.start_byte and m.end_byte = bss.end_byte)
    left outer join match_snippet ms on (m.id = ms.match_id)
    left outer join blob b on (m.blob_id = b.id)
    left outer join rule r on (m.rule_id = r.id)
    left outer join match_groups mg on (m.rule_id = mg.rule_id)
    left outer join snippet before_snippet on (ms.before_snippet_id = before_snippet.id)
    left outer join snippet matching_snippet on (ms.matching_snippet_id = matching_snippet.id)
    left outer join snippet after_snippet on (ms.after_snippet_id = after_snippet.id)
    left outer join match_status on (m.id = match_status.match_id)
    left outer join match_comment on (m.id = match_comment.match_id)
    left outer join match_score on (m.id = match_score.match_id)
;

CREATE VIEW blob_denorm
-- A convenience view for blobs in their fully denormalized form rather
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

CREATE VIEW finding_denorm
-- A convenience view for findings in their fully denormalized form rather
-- than the low-level datastore form that involves numerous indirection.
(
    rule_name,
    rule_structural_id,
    rule_syntax,
    groups,
    finding_id
)
as
select
    r.name,
    r.structural_id,
    rs.syntax,
    mg.groups,
    mf.finding_id
from
    match m
    inner join match_finding_id mf on (m.id = mf.match_id)
    inner join match_groups mg on (m.id = mg.match_id)
    inner join rule r on (m.rule_id = r.id)
    inner join rule_syntax rs on (rs.rule_id = r.id)
;

CREATE VIEW finding_summary
-- A convenience view for a summary of findings in denormalized form.
(
    rule_name,
    rule_structural_id,
    total_findings,
    total_matches
)
as
select
    rule_name,
    rule_structural_id,
    count(distinct finding_id),
    count(*)
from
    finding_denorm
group by 1, 2
;
